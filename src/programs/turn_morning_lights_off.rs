use super::TurningMorningLightsOffConfig;
use chrono::{DateTime, Local, NaiveTime, Timelike, Utc};
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
    pub fn run(&mut self, client: &reqwest::Client, token: &str) {
        info!("Executing `TurnMorningLightsOffProgram`.");
        if !self.active {
            info!("Program inactive - skipping.");
            return;
        }

        let now = Local::now();
        info!("Now: {}", now);

        if let Some(last_turned_off) = self.last_turned_light_off {
            if last_turned_off.date_naive() == now.date_naive() {
                info!("Already turned off the morning light today - skipping.");
                return;
            }
        }

        if self.off_time < now.time() {
            info!("After registered off-time.");
            println!("TURN LIGHT OFF");
            self.last_turned_light_off = Some(now);
        }
    }
}
