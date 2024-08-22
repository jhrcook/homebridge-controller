use clap::Parser;
use configuration::Configuration;
use homebridge::Homebridge;
use homebridge_controller::suntimes::SunTimes;
use log::{error, info};
use programs::turn_morning_lights_off::TurnMorningLightsOffProgram;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use tokio::time::sleep;
pub mod configuration;
pub mod homebridge;
pub mod programs;

#[derive(Serialize, Deserialize, Debug)]
struct Secrets {
    username: String,
    password: String,
}

/// Automated programs controlling Homebridge accessories.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Configuration file.
    config: PathBuf,
    /// Secrets file.
    #[arg(short, long, default_value = "./secrets.json")]
    secrets: PathBuf,
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
    let secrets_file = fs::File::open(args.secrets).unwrap();
    let secrets: Secrets = serde_json::from_reader(secrets_file).unwrap();

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
    let mut _suntimes = SunTimes::new(config.longitude, config.latitude);

    loop {
        info!("Running program loop.");
        match lights_off_prog.run(&client, &mut homebridge).await {
            Ok(()) => info!("Successfully executed lights-off program."),
            Err(e) => error!("Error running programing to turn morning lights off: {}", e),
        };
        info!("Finished program loop.");
        sleep(Duration::from_secs_f32(config.program_loop_pause)).await;
    }
}
