use serde::{Deserialize, Serialize};

pub mod turn_morning_lights_off;

#[derive(Serialize, Deserialize, Debug)]
pub struct TurningMorningLightsOffConfig {
    pub duration: u32,
    pub off_time: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub turn_morning_lights_off: TurningMorningLightsOffConfig,
    pub program_loop_pause: u64,
}
