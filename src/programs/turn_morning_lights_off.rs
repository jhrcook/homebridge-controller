use crate::configuration::TurningMorningLightsOffConfig;
use crate::homebridge::Homebridge;

use chrono::{DateTime, Local, NaiveTime};
use log::{debug, error, info};

pub struct TurnMorningLightsOffProgram {
    pub duration: u32,
    pub off_time: NaiveTime,
    pub active: bool,
    last_turned_light_off: Option<DateTime<Local>>,
}

impl TurnMorningLightsOffProgram {
    pub fn new(config: &TurningMorningLightsOffConfig) -> Self {
        info!("Creating a `TurnMorningLightsOffProgram` object.");
        let off_time = match NaiveTime::parse_from_str(&config.off_time, "%H:%M:%S") {
            Ok(t) => t,
            Err(e) => {
                error!("Error parsing time: {}", config.off_time);
                panic!("{:?}", e)
            }
        };
        TurnMorningLightsOffProgram {
            off_time,
            duration: config.duration,
            active: config.active,
            last_turned_light_off: Option::None,
        }
    }
}

impl TurnMorningLightsOffProgram {
    pub async fn run(&mut self, client: &reqwest::Client, homebridge: &mut Homebridge) {
        info!("Executing `TurnMorningLightsOffProgram`.");
        if !self.active {
            info!("Program inactive - nothing to do.");
            return;
        }

        let now = Local::now();
        info!("Now: {}", now);

        if let Some(last_turned_off) = self.last_turned_light_off {
            if last_turned_off.date_naive() == now.date_naive() {
                info!("Already turned off the morning light today - nothing to do.");
                return;
            }
        }

        if now.time() < self.off_time {
            info!("Not yet time to turn off light - nothing to do.");
            return;
        }

        info!("After registered off-time, attempting to turn the light off.");
        match homebridge.turn_off_bed_light(client).await {
            Ok(()) => self.last_turned_light_off = Some(now),
            Err(e) => error!("Error:{:?}", e),
        };
    }
}
