use crate::homebridge::Homebridge;
use crate::suntimes::SunTimes;
use crate::{configuration::TurningMorningLightsOffConfig, homebridge::HBError};
use chrono::{DateTime, Duration, Local, NaiveTime};
use log::{debug, error, info, warn};

#[derive(thiserror::Error, Debug)]
pub enum TurnMorningLightsOffProgramError {
    #[error("{0}")]
    ParseError(String),
    #[error("Error during Homebridge interaction.")]
    HomebridgeInteraction(#[from] HBError),
}

pub struct TurnMorningLightsOffProgram {
    pub duration: u32,
    pub off_time: Option<NaiveTime>,
    pub after_sunrise: Option<i64>,
    pub active: bool,
    last_turned_light_off: Option<DateTime<Local>>,
}

impl TurnMorningLightsOffProgram {
    pub fn new(
        config: &TurningMorningLightsOffConfig,
    ) -> Result<Self, TurnMorningLightsOffProgramError> {
        info!("Creating a `TurnMorningLightsOffProgram` object.");

        if config.off_time.is_none() && config.after_sunrise.is_none() {
            warn!("Both `off_time` and `after_sunrise` are None.")
        } else if config.off_time.is_some() && config.after_sunrise.is_some() {
            warn!("Both `off_time` and `after_sunrise` are provided; `off_time` takes precedence.")
        }

        let off_time: Option<NaiveTime> = match &config.off_time {
            Some(t) => Some(NaiveTime::parse_from_str(t, "%H:%M:%S").map_err(|e| {
                TurnMorningLightsOffProgramError::ParseError(format!(
                    "Error parsing off time: {}",
                    e
                ))
            })?),
            None => None,
        };

        Ok(TurnMorningLightsOffProgram {
            off_time,
            after_sunrise: config.after_sunrise,
            duration: config.duration,
            active: config.active,
            last_turned_light_off: Option::None,
        })
    }
}

impl TurnMorningLightsOffProgram {
    pub async fn run(
        &mut self,
        client: &reqwest::Client,
        homebridge: &mut Homebridge,
        suntimes: &mut SunTimes,
    ) -> Result<(), TurnMorningLightsOffProgramError> {
        info!("Executing `TurnMorningLightsOffProgram`.");
        if !self.active {
            debug!("Program inactive - nothing to do.");
            return Ok(());
        }

        let now = Local::now();
        debug!("Now: {}", now);

        if let Some(last_turned_off) = self.last_turned_light_off {
            if last_turned_off.date_naive() == now.date_naive() {
                debug!("Already turned off the morning light today - nothing to do.");
                return Ok(());
            }
        }

        if let Some(off_time) = self.off_time {
            if now.time() < off_time {
                debug!("Not yet time to turn off light - nothing to do.");
                return Ok(());
            }
        } else if let Some(after_sunrise) = self.after_sunrise {
            let sunrise = suntimes.sunrise(client).await;
            if now.time() < sunrise.time() + Duration::minutes(after_sunrise) {
                debug!("Not yet time to turn off light - nothing to do.");
                return Ok(());
            }
        }

        info!("After registered off-time, attempting to turn the light off.");
        homebridge
            .turn_off_bed_light(client)
            .await
            .map_err(TurnMorningLightsOffProgramError::HomebridgeInteraction)?;
        self.last_turned_light_off = Some(now);
        Ok(())
    }
}
