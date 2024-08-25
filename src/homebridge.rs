use chrono::{DateTime, Duration, Local};
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum HBError {
    #[error("Failed to connect to HB endpoint.")]
    UnableToConnect(#[from] reqwest::Error),
    #[error("{0}")]
    ParsingError(String),
    #[error("Authentication error with Homebridge: {0}")]
    AuthError(String),
    #[error("No access token when one is expected.")]
    NoAccessToken(),
    #[error("No accessory registered for '{0}'.")]
    UnrecognizedAccessory(String),
}

#[derive(Serialize, Deserialize, Debug)]
struct HBAccessory {
    uuid: String,
    #[serde(rename = "uniqueId")]
    unique_id: String,
    #[serde(rename = "type")]
    acc_type: String,
    #[serde(rename = "humanType")]
    huamn_type: String,
    #[serde(rename = "serviceName")]
    service_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
struct HBAccessories {
    accessories: Vec<HBAccessory>,
}

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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct HBLightbulbValues {
    pub on: u32,
    pub brightness: u8,
    pub color_temperature: u32,
    pub hue: u32,
    pub saturation: u32,
}

impl HBLightbulbValues {
    pub fn is_on(&self) -> bool {
        self.on == 1
    }
    pub fn is_off(&self) -> bool {
        !self.is_on()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HBLightbulb {
    pub uuid: String,
    #[serde(rename = "uniqueId")]
    pub unique_id: String,
    #[serde(rename = "type")]
    pub acc_type: String,
    #[serde(rename = "humanType")]
    pub huamn_type: String,
    #[serde(rename = "serviceName")]
    pub service_name: String,
    pub values: HBLightbulbValues,
}

impl Homebridge {
    pub async fn check_connection(&self, client: &reqwest::Client) -> Result<(), HBError> {
        _ = client
            .post(&self.ip_address)
            .send()
            .await
            .map_err(HBError::UnableToConnect)?;
        Ok(())
    }
}

impl Homebridge {
    async fn renew_access_token(&mut self, client: &reqwest::Client) -> Result<(), HBError> {
        let mut map = HashMap::new();
        map.insert("username", &self.username);
        map.insert("password", &self.password);
        let mut endpt = self.ip_address.clone();
        endpt.push_str("/api/auth/login");
        let res = client
            .post(endpt)
            .json(&map)
            .send()
            .await
            .map_err(HBError::UnableToConnect)?;
        let parsed_auth = match res.status() {
            reqwest::StatusCode::CREATED => res.json::<HBAuth>().await.map_err(|e| {
                HBError::ParsingError(format!("Error parsing `HBAuth` data - {}", e))
            })?,
            other => return Err(HBError::AuthError(format!("Status code {}", other))),
        };
        self.access_token = Some(parsed_auth.access_token);
        self.access_token_expiration =
            Some(Local::now() + Duration::seconds(parsed_auth.expires_in as i64 - 60));
        Ok(())
    }

    pub async fn access_token(&mut self, client: &Client) -> Result<String, HBError> {
        if self.access_token.is_none() | self.access_token_expiration.is_none() {
            debug!("No access token, requesting one.");
            self.renew_access_token(client).await?;
        } else if let Some(access_token_expiration) = self.access_token_expiration {
            if access_token_expiration < Local::now() {
                debug!("Access token expired, requesting new one.");
                self.renew_access_token(client).await?;
            }
        }
        match self.access_token.clone() {
            Some(token) => Ok(token),
            None => Err(HBError::NoAccessToken()),
        }
    }
}

impl Homebridge {
    async fn get_accessory_uuid(
        &mut self,
        client: &Client,
        acc_name: &str,
    ) -> Result<String, HBError> {
        if let Some(acc_uuid) = self.accessory_uuids.get(acc_name) {
            debug!("Found UUID for {} in accessory UUID table.", acc_name);
            return Ok(acc_uuid.clone());
        };

        let access_token = self.access_token(&client).await?;

        let mut endpt = self.ip_address.clone();
        endpt.push_str("/api/accessories");

        let res = client
            .get(endpt)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(HBError::UnableToConnect)?;
        let accesories = res.json::<HBAccessories>().await.map_err(|e| {
            HBError::ParsingError(format!("Error parsing `HBAccessories` data - {}", e))
        })?;
        for accessory in accesories.accessories.iter() {
            let acc_id = accessory.unique_id.clone();
            if accessory.service_name == acc_name {
                debug!("Adding UUID for '{}' to accessory UUID table.", acc_name);
                self.accessory_uuids
                    .insert(acc_name.to_string(), acc_id.clone());
                return Ok(acc_id);
            }
        }

        error!(
            "Did not find an accessory with service name '{}'.",
            acc_name
        );
        Err(HBError::UnrecognizedAccessory(acc_name.to_string()))
    }

    async fn bed_light_uuid(&mut self, client: &Client) -> Result<String, HBError> {
        self.get_accessory_uuid(client, "Bed Light").await
    }

    pub async fn get_bed_light_status(&mut self, client: &Client) -> Result<HBLightbulb, HBError> {
        debug!("Retrieving bed light status.");
        let access_token = self.access_token(&client).await?;
        let light_uuid = self.get_accessory_uuid(client, "Bed Light").await?;

        let mut endpt = self.ip_address.clone();
        endpt.push_str("/api/accessories/");
        endpt.push_str(&light_uuid);

        let res = client
            .get(endpt)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(HBError::UnableToConnect)?;
        debug!("Parsing bed light data.");
        res.json::<HBLightbulb>().await.map_err(|e| {
            HBError::ParsingError(format!("Error parsing `HBAccessories` data - {}", e))
        })
    }

    pub async fn bed_light_is_off(&mut self, client: &Client) -> Result<bool, HBError> {
        let values = self.get_bed_light_status(client).await?.values;
        Ok(values.on == 0)
    }
}

impl Homebridge {
    async fn _set_bedlight<T>(
        &mut self,
        client: &Client,
        characteristic: &str,
        value: T,
    ) -> Result<(), HBError>
    where
        T: Serialize,
    {
        let access_token = self.access_token(&client).await?;

        let mut endpt = self.ip_address.clone();
        endpt.push_str("/api/accessories/");
        endpt.push_str(&self.bed_light_uuid(client).await?);

        let body = json!({
            "characteristicType": characteristic,
            "value": value,
        });

        client
            .put(endpt)
            .bearer_auth(&access_token)
            .json(&body)
            .send()
            .await
            .map_err(HBError::UnableToConnect)?;

        Ok(())
    }

    pub async fn turn_bedlight_on(&mut self, client: &Client) -> Result<(), HBError> {
        info!("Turning bed light ON.");
        self._set_bedlight(client, "On", "1").await
    }
    pub async fn turn_bedlight_off(&mut self, client: &Client) -> Result<(), HBError> {
        info!("Turning bed light OFF.");
        self._set_bedlight(client, "On", "0").await
    }

    pub async fn set_bedlight_brightness(
        &mut self,
        client: &Client,
        brightness: u8,
    ) -> Result<(), HBError> {
        info!("Setting bed light brightness: {}.", brightness);
        self._set_bedlight(client, "Brightness", &brightness).await
    }

    pub async fn set_bedlight(
        &mut self,
        client: &Client,
        values: &HBLightbulbValues,
    ) -> Result<(), HBError> {
        info!("Setting bed light values: {:?}", values);
        self._set_bedlight(client, "On", &values.on.to_string())
            .await?;
        self._set_bedlight(client, "Brightness", &values.brightness.to_string())
            .await?;
        self._set_bedlight(
            client,
            "ColorTemperature",
            &values.color_temperature.to_string(),
        )
        .await?;
        self._set_bedlight(client, "Hue", &values.hue.to_string())
            .await?;
        self._set_bedlight(client, "Saturation", &values.saturation.to_string())
            .await?;
        Ok(())
    }
}
