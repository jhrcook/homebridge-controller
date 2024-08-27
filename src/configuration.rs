use serde::{Deserialize, Serialize};

const fn _true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TurningMorningLightsOffConfig {
    #[serde(default = "_true")]
    pub active: bool,
    pub duration: u32,
    pub off_time: Option<String>,
    pub after_sunrise: Option<i64>,
    pub last_call_after_scheduled_off: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ControlEveningLightsConfig {
    #[serde(default = "_true")]
    pub active: bool,
    pub minutes_before_sunset_start: i64,
    pub minutes_after_sunset_peak: i64,
    pub minutes_after_sunset_finish: i64,
    pub start_brightness: u8,
    pub max_brightness: u8,
    pub final_brightness: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub turn_morning_lights_off: TurningMorningLightsOffConfig,
    pub control_evening_lights: ControlEveningLightsConfig,
    pub program_loop_pause: f32,
    pub n_cycles_reload_config: u32,
    pub ip_address: String,
    pub latitude: f32,
    pub longitude: f32,
}
