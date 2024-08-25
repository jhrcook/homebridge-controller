use crate::homebridge::Homebridge;
use crate::suntimes::{SunTimes, SuntimesError};
use crate::{configuration::ControlEveningLightsConfig, homebridge::HBError};
use chrono::{DateTime, Duration, Local, Timelike};
use core::time;
use log::{debug, error, info};
use std::cmp::{max, min};
use std::thread;

#[derive(thiserror::Error, Debug)]
pub enum ControlEveningLightsProgramError {
    #[error("{0}")]
    ParseError(String),
    #[error("Error during Homebridge interaction.")]
    HomebridgeInteraction(#[from] HBError),
    #[error("{0}")]
    ConfigurationError(String),
    #[error("{0}")]
    NoSunTimesData(#[from] SuntimesError),
}

#[derive(Debug, Clone, Copy)]
struct LightsHistory {
    when: DateTime<Local>,
    brightness: u8,
    // set_by_program: bool,
}

#[derive(Debug)]
pub struct ControlEveningLightsProgram {
    pub active: bool,
    pub minutes_before_sunset_start: i64,
    pub minutes_after_sunset_peak: i64,
    pub minutes_after_sunset_finish: i64,
    pub start_brightness: u8,
    pub max_brightness: u8,
    pub final_brightness: u8,
    history: Option<LightsHistory>,
}

impl ControlEveningLightsProgram {
    pub fn new(
        config: &ControlEveningLightsConfig,
    ) -> Result<Self, ControlEveningLightsProgramError> {
        if !((-1 * config.minutes_before_sunset_start) <= config.minutes_after_sunset_peak) {
            error!("Logical errors in `ControlEveningLightsProgram` configuration.");
            return Err(ControlEveningLightsProgramError::ConfigurationError(
                "The start time must precede the peak time.".to_string(),
            ));
        }
        if !(config.minutes_after_sunset_peak <= config.minutes_after_sunset_finish) {
            error!("Logical errors in `ControlEveningLightsProgram` configuration.");
            return Err(ControlEveningLightsProgramError::ConfigurationError(
                "The time for peak must precede the finish time.".to_string(),
            ));
        }

        Ok(Self {
            active: config.active,
            minutes_before_sunset_start: config.minutes_before_sunset_start,
            minutes_after_sunset_peak: config.minutes_after_sunset_peak,
            minutes_after_sunset_finish: config.minutes_after_sunset_finish,
            start_brightness: config.start_brightness,
            max_brightness: config.max_brightness,
            final_brightness: config.final_brightness,
            history: None,
        })
    }
}

#[derive(Debug)]
struct TimeBrightCoord {
    dt: DateTime<Local>,
    b: f32,
}

impl TimeBrightCoord {
    fn new(dt: DateTime<Local>, b: u8) -> Self {
        return Self { dt, b: b as f32 };
    }

    fn sec_since_midnight(&self) -> f32 {
        return self.dt.num_seconds_from_midnight() as f32;
    }
}

impl ControlEveningLightsProgram {
    fn current_brightness(&self, now: &DateTime<Local>, sunset: &DateTime<Local>) -> u8 {
        let peak_time = sunset.clone() + Duration::minutes(self.minutes_after_sunset_peak);
        let (c1, c2) = match now <= &peak_time {
            true => {
                let start = TimeBrightCoord::new(
                    sunset.clone() - Duration::minutes(self.minutes_before_sunset_start),
                    self.start_brightness,
                );
                let peak = TimeBrightCoord::new(
                    sunset.clone() + Duration::minutes(self.minutes_after_sunset_peak),
                    self.max_brightness,
                );
                (start, peak)
            }
            false => {
                let peak = TimeBrightCoord::new(
                    sunset.clone() + Duration::minutes(self.minutes_after_sunset_peak),
                    self.max_brightness,
                );
                let end = TimeBrightCoord::new(
                    sunset.clone() + Duration::minutes(self.minutes_after_sunset_finish),
                    self.final_brightness,
                );
                (peak, end)
            }
        };

        debug!("c1: {:?}, c2: {:?}", c1, c2);
        let slope = (c1.b - c2.b) / (c1.sec_since_midnight() - c2.sec_since_midnight());
        let brightness =
            slope * (now.num_seconds_from_midnight() as f32 - c1.sec_since_midnight()) + c1.b;
        debug!("slope: {}, brightness: {}", slope, brightness);
        brightness as u8
    }

    pub async fn run(
        &mut self,
        client: &reqwest::Client,
        homebridge: &mut Homebridge,
        suntimes: &mut SunTimes,
    ) -> Result<(), ControlEveningLightsProgramError> {
        info!("Executing `ControlEveningLightsProgram`.");
        let sunset = suntimes
            .sunset(client)
            .await
            .map_err(ControlEveningLightsProgramError::NoSunTimesData)?;
        let now = Local::now();

        debug!("Now: {:?}", now);
        debug!("Sunset: {:?}", sunset);

        let _start = sunset - Duration::minutes(self.minutes_before_sunset_start);
        let _peak = sunset + Duration::minutes(self.minutes_after_sunset_peak);
        let _end = sunset + Duration::minutes(self.minutes_after_sunset_finish);
        let in_a = (_start <= now) && (now <= _peak);
        let in_b = (_peak < now) && (now <= _end);

        debug!("Start: {}", _start);
        debug!("Peak: {}", _peak);
        debug!("End: {}", _end);
        debug!("In A: {}, in B: {}", in_a, in_b);

        // Check if within operating window, else exit early.
        if !in_a && !in_b {
            debug!("Outside of operating times - nothing to do.");
            if self.history.is_some() {
                self.history = None;
            }
            return Ok(());
        }

        let current_bulb = homebridge.get_bed_light_status(client).await?.values;
        debug!("Current bulb values: {:?}", current_bulb);

        if current_bulb.is_off() && self.history.is_some() {
            info!("Bed light turned OFF after program started - doing nothing.");
            return Ok(());
        }

        if let Some(history) = self.history {
            if current_bulb.brightness != history.brightness {
                info!("Bed light brightness adjusted externally - doing nothing.");
                return Ok(());
            }
            if history.when.minute() == now.minute() {
                info!("Already changed values this minute - doing nothing.");
                return Ok(());
            }
        }

        let mut new_brightness = self.current_brightness(&now, &sunset);
        if in_a {
            // Only increase the brightness during step A.
            new_brightness = max(new_brightness, current_bulb.brightness);
        } else if in_b {
            // Only decrease the brightness during step B.
            new_brightness = min(new_brightness, current_bulb.brightness);
        }

        if new_brightness == 0 {
            info!("Skipping setting brightness to 0.");
            return Ok(());
        } else if new_brightness == current_bulb.brightness {
            info!("New brightness same as current brightness - doing nothing.");
            return Ok(());
        }

        if homebridge.bed_light_is_off(client).await? {
            homebridge.turn_bedlight_on(client).await?;
            thread::sleep(time::Duration::from_millis(250));
        }
        homebridge
            .set_bedlight_brightness(client, new_brightness)
            .await?;
        thread::sleep(time::Duration::from_millis(250));
        self.history = Some(LightsHistory {
            when: now,
            brightness: new_brightness,
        });
        Ok(())
    }
}
