use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TurningMorningLightsOffConfig {
    pub duration: u32,
    pub off_time: Option<String>,
    pub after_sunrise: Option<i64>,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub turn_morning_lights_off: TurningMorningLightsOffConfig,
    pub program_loop_pause: f32,
    pub ip_address: String,
    pub latitude: f32,
    pub longitude: f32,
}
