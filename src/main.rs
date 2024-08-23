use crate::configuration::Configuration;
use crate::homebridge::Homebridge;
use crate::programs::turn_morning_lights_off::TurnMorningLightsOffProgram;
use crate::suntimes::SunTimes;
use clap::Parser;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::env::VarError;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use std::{env, fs};
use tokio::time::sleep;

pub mod configuration;
pub mod homebridge;
pub mod programs;
pub mod suntimes;

#[derive(Serialize, Deserialize, Debug)]
struct Secrets {
    username: String,
    password: String,
}

impl Secrets {
    fn from_env() -> Result<Self, VarError> {
        let username = env::var("HB_USER")?;
        let password = env::var("HB_PASSWORD")?;
        return Ok(Self { username, password });
    }
}

/// Automated programs controlling Homebridge accessories.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Configuration file.
    config: PathBuf,
}

#[tokio::main]
async fn main() -> ExitCode {
    env_logger::init();

    let args = Arguments::parse();
    info!("Parsed CLI arguments.");

    // Configuration.
    let config_file = fs::File::open(args.config).unwrap();
    let config: Configuration = serde_json::from_reader(config_file).unwrap();
    info!("Config:\n{:?}", config);

    // Secrets.
    // let secrets_file = fs::File::open(args.secrets).unwrap();
    // let secrets: Secrets = serde_json::from_reader(secrets_file).unwrap();
    let secrets = match Secrets::from_env() {
        Ok(s) => s,
        Err(e) => {
            error!("Error getting Homebridge auth values: {}.", e);
            return ExitCode::from(4);
        }
    };

    // Create `reqwest` client.
    let client = reqwest::Client::new();

    // Create Homebridge client.
    let mut homebridge = Homebridge::new(&config.ip_address, &secrets.username, &secrets.password);
    match homebridge.check_connection(&client).await {
        Ok(()) => info!("Test Homebridge connection successful."),
        Err(e) => {
            error!("Could not connect to Homebridge: {}", e);
            return ExitCode::from(4);
        }
    };

    // Create programs.
    let mut lights_off_prog =
        match TurnMorningLightsOffProgram::new(&config.turn_morning_lights_off) {
            Ok(p) => p,
            Err(e) => {
                error!("{}", e);
                return ExitCode::from(4);
            }
        };

    // Sunrise/sunset data.
    let mut suntimes = SunTimes::new(config.longitude, config.latitude);

    loop {
        info!("Running program loop.");
        match lights_off_prog
            .run(&client, &mut homebridge, &mut suntimes)
            .await
        {
            Ok(()) => info!("Successfully executed lights-off program."),
            Err(e) => error!("Error running programing to turn morning lights off: {}", e),
        };
        info!("Finished program loop.");
        sleep(Duration::from_secs_f32(config.program_loop_pause)).await;
    }
}
