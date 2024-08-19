use chrono::{DateTime, Local, NaiveTime, Utc};
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct SunriseSunsetData {
    sunrise: String,
    sunset: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SunriseSunsetResponse {
    results: SunriseSunsetData,
}

pub struct SunTimes {
    sunrise: Option<DateTime<Local>>,
    sunset: Option<DateTime<Local>>,
}

impl SunTimes {
    pub fn new() -> Self {
        Self {
            sunrise: None,
            sunset: None,
        }
    }
}

impl SunTimes {
    async fn collect_sunrise_sunset_data(&mut self, client: &Client) {
        let res = client
            .get("https://api.sunrise-sunset.org/json?lat=36.7201600&lng=-4.4203400&date=today&formatted=0")
            .send()
            .await;
        let suntimes_data = match res {
            Ok(dt_res) => dt_res.json::<SunriseSunsetResponse>().await.unwrap(),
            Err(e) => {
                error!("Could not get sunrise time.");
                panic!("{}", e);
            }
        };
        debug!("suntimes_data:\n{:?}", suntimes_data);
        let sunrise = suntimes_data
            .results
            .sunrise
            .parse::<DateTime<Utc>>()
            .unwrap();
        debug!("Sunrise: {:?}", sunrise);
        let sunset = suntimes_data
            .results
            .sunset
            .parse::<DateTime<Utc>>()
            .unwrap();
        debug!("Sunset: {:?}", sunset);
        self.sunrise = Some(DateTime::from(sunrise));
        self.sunset = Some(DateTime::from(sunset));
    }

    pub async fn sunrise(&mut self, client: &Client) -> DateTime<Local> {
        if let Some(sunrise) = self.sunrise {
            if sunrise.date_naive() == Local::now().date_naive() {
                return sunrise;
            }
            debug!("Sunrise data stale.")
        }
        self.collect_sunrise_sunset_data(client).await;
        match self.sunrise {
            Some(sunrise) => sunrise,
            None => panic!("Could not collect sunrise data."),
        }
    }

    pub async fn sunset(&mut self, client: &Client) -> DateTime<Local> {
        if let Some(sunset) = self.sunset {
            if sunset.date_naive() == Local::now().date_naive() {
                return sunset;
            }
            debug!("Sunset data stale.")
        }
        self.collect_sunrise_sunset_data(client).await;
        match self.sunset {
            Some(sunset) => sunset,
            None => panic!("Could not collect sunset data."),
        }
    }
}
