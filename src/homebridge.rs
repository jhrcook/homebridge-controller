use std::{collections::HashMap, fmt::Error};

use chrono::{DateTime, Duration, Local};
use log::{debug, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct Homebridge {
    pub ip_address: String,
    username: String,
    password: String,
    access_token: Option<String>,
    access_token_expiration: Option<DateTime<Local>>,
    accessory_uuids: HashMap<String, String>,
}

impl Homebridge {
    pub fn new(ip_address: &str, username: &str, password: &str) -> Self {
        Self {
            ip_address: ip_address.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            access_token: None,
            access_token_expiration: None,
            accessory_uuids: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct HBAuth {
    access_token: String,
    token_type: String,
    expires_in: u32,
}

impl Homebridge {
    async fn renew_access_token(&mut self, client: &reqwest::Client) {
        let mut map = HashMap::new();
        map.insert("username", &self.username);
        map.insert("password", &self.password);
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
        self.access_token = Some(parsed_auth.access_token);
        self.access_token_expiration =
            Some(Local::now() + Duration::seconds(parsed_auth.expires_in as i64 - 60))
    }

    pub async fn access_token(&mut self, client: &Client) -> String {
        if self.access_token.is_none() | self.access_token_expiration.is_none() {
            debug!("No access token, requesting one.");
            self.renew_access_token(client).await;
        } else if let Some(access_token_expiration) = self.access_token_expiration {
            if access_token_expiration < Local::now() {
                debug!("Access token expired, requesting new one.");
                self.renew_access_token(client).await;
            }
        }
        match self.access_token.clone() {
            Some(token) => token,
            None => panic!("No access token available."),
        }
    }
}

impl Homebridge {
    async fn bed_light_uuid(&mut self, client: &Client) -> String {
        // TODO: get the bed light UUID automatically and store for later in `accessory_uuids`.
        return "3b41b98a6fc7128c27917ac7cb89395ece21a9a2213ebc9e6dd2d95868b9d0a5".to_string();
    }
}

impl Homebridge {
    pub async fn turn_off_bed_light(&mut self, client: &Client) -> Result<(), Error> {
        info!("Turning off bed light.");

        let mut body = HashMap::new();
        body.insert("characteristicType", "On");
        body.insert("value", "0");

        let access_token = self.access_token(&client).await;

        let mut endpt = self.ip_address.clone();
        endpt.push_str("/api/accessories/");
        endpt.push_str(&self.bed_light_uuid(client).await);

        let res = client
            .put(endpt)
            .bearer_auth(&access_token)
            .json(&body)
            .send()
            .await
            .unwrap();
        debug!("Changing light on/off status code: {}", res.status());
        Ok(())
    }
}
