use crate::homebridge::Homebridge;
use crate::{configuration::TurningMorningLightsOffConfig, homebridge::HBError};
use chrono::{DateTime, Local, NaiveTime};
use log::{debug, error, info};

#[derive(thiserror::Error, Debug)]
pub enum TurnMorningLightsOffProgramError {
    #[error("{0}")]
    ParseError(String),
    #[error("Error during Homebridge interaction.")]
    HomebridgeInteraction(#[from] HBError),
}

pub struct TurnMorningLightsOffProgram {
    pub duration: u32,
    pub off_time: NaiveTime,
    pub active: bool,
    last_turned_light_off: Option<DateTime<Local>>,
}

impl TurnMorningLightsOffProgram {
    pub fn new(
        config: &TurningMorningLightsOffConfig,
    ) -> Result<Self, TurnMorningLightsOffProgramError> {
        info!("Creating a `TurnMorningLightsOffProgram` object.");
        let off_time = NaiveTime::parse_from_str(&config.off_time, "%H:%M:%S").map_err(|e| {
            TurnMorningLightsOffProgramError::ParseError(format!("Error parsing off time - {}", e))
        })?;
        Ok(TurnMorningLightsOffProgram {
            off_time,
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
    ) -> Result<(), TurnMorningLightsOffProgramError> {
        info!("Executing `TurnMorningLightsOffProgram`.");
        if !self.active {
            info!("Program inactive - nothing to do.");
            return Ok(());
        }

        let now = Local::now();
        info!("Now: {}", now);

        if let Some(last_turned_off) = self.last_turned_light_off {
            if last_turned_off.date_naive() == now.date_naive() {
                info!("Already turned off the morning light today - nothing to do.");
                return Ok(());
            }
        }

        if now.time() < self.off_time {
            info!("Not yet time to turn off light - nothing to do.");
            return Ok(());
        }

        info!("After registered off-time, attempting to turn the light off.");
        homebridge
            .turn_off_bed_light(client)
            .await
            .map_err(TurnMorningLightsOffProgramError::HomebridgeInteraction)?;
        Ok(())
    }
}
