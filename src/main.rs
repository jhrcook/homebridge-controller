use clap::Parser;
use configuration::Configuration;
use homebridge::Homebridge;
use homebridge_controller::suntimes::SunTimes;
use log::info;
use programs::turn_morning_lights_off::TurnMorningLightsOffProgram;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
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
async fn main() {
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

    // Create programs.
    let mut lights_off_prog = TurnMorningLightsOffProgram::new(&config.turn_morning_lights_off);

    // DEMO
    let mut suntimes = SunTimes::new();
    suntimes.sunrise(&client).await;
    suntimes.sunset(&client).await;
    return;
    // ---------------

    let mut _ct = 0;
    loop {
        info!("Running program loop.");
        lights_off_prog.run(&client, &mut homebridge).await;
        info!("Finished program loop.");
        sleep(Duration::from_secs_f32(config.program_loop_pause)).await;
        _ct += 1;
        if _ct >= 5 {
            break;
        }
    }
}
