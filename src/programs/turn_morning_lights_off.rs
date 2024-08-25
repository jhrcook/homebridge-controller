use crate::homebridge::Homebridge;
use crate::suntimes::{SunTimes, SuntimesError};
use crate::{configuration::TurningMorningLightsOffConfig, homebridge::HBError};
use chrono::{DateTime, Duration, Local, NaiveTime};
use core::time;
use log::{debug, error, info, warn};
use std::thread;

#[derive(thiserror::Error, Debug)]
pub enum TurnMorningLightsOffProgramError {
    #[error("{0}")]
    ParseError(String),
    #[error("Error during Homebridge interaction.")]
    HomebridgeInteraction(#[from] HBError),
    #[error("{0}")]
    ConfigError(String),
    #[error("{0}")]
    NoSunTimesData(#[from] SuntimesError),
}

pub struct TurnMorningLightsOffProgram {
    pub duration: u32,
    pub off_time: Option<NaiveTime>,
    pub after_sunrise: Option<i64>,
    pub active: bool,
    pub last_call_after_scheduled_off: u32,
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
            last_call_after_scheduled_off: config.last_call_after_scheduled_off,
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

        // Calculate the off-time depending on the configuration.
        let off_time = match (self.off_time, self.after_sunrise) {
            (Some(ot), _) => ot,
            (None, Some(after_sunrise)) => {
                let sunrise = suntimes
                    .sunrise(client)
                    .await
                    .map_err(TurnMorningLightsOffProgramError::NoSunTimesData)?;
                debug!("Sunrise: {}", sunrise);
                sunrise.time() + Duration::minutes(after_sunrise)
            }
            (None, None) => {
                return Err(TurnMorningLightsOffProgramError::ConfigError(
                    "Both off-times are None.".to_string(),
                ))
            }
        };
        debug!("Off-time: {}", off_time);

        if now.time() < off_time {
            debug!("Not yet time to turn off light - nothing to do.");
            return Ok(());
        }
        if (off_time + Duration::minutes(self.last_call_after_scheduled_off as i64)) < now.time() {
            debug!("After last-call time - nothing to do.");
            return Ok(());
        }

        info!("After registered off-time, attempting to turn the light off.");
        homebridge
            .turn_bedlight_off(client)
            .await
            .map_err(TurnMorningLightsOffProgramError::HomebridgeInteraction)?;
        thread::sleep(time::Duration::from_millis(250));
        if homebridge.bed_light_is_off(client).await? {
            info!("Successfully turned OFF bed light.");
            self.last_turned_light_off = Some(now);
        } else {
            warn!("The bed light is still ON after switching OFF.");
        }
        Ok(())
    }
}
