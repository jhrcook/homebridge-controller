use clap::Parser;
use log::{info, warn};
use programs::turn_morning_lights_off::TurnMorningLightsOffProgram;
use programs::Configuration;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use std::{collections::HashMap, fs};
use tokio::time::sleep;
pub mod programs;

#[derive(Serialize, Deserialize, Debug)]
struct Secrets {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct HBAuth {
    access_token: String,
    token_type: String,
    expires_in: u32,
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

    // Get an access token.
    let mut map = HashMap::new();
    map.insert("username", &secrets.username);
    map.insert("password", &secrets.password);
    let res = client
        .post("http://192.168.0.213:8581/api/auth/login")
        .json(&map)
        .send()
        .await
        .unwrap();
    let parsed_auth = match res.status() {
        reqwest::StatusCode::CREATED => match res.json::<HBAuth>().await {
            Ok(parsed_auth) => {
                info!("Successfully parsed HB auth.");
                parsed_auth
            }
            Err(e) => panic!("Error parsing auth response: {:?}", e),
        },
        other => panic!("Failed authorization: {:?}", other),
    };

    let mut lights_off_prog = TurnMorningLightsOffProgram::new(&config.turn_morning_lights_off);

    loop {
        info!("Running program loop.");
        lights_off_prog.run(&client, &parsed_auth.access_token);
        info!("Finished program loop.");
        sleep(Duration::from_secs(config.program_loop_pause)).await;
    }
}
