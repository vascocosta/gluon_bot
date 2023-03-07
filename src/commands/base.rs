use crate::database::{CsvRecord, Database};
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time;

#[derive(PartialEq)]
struct WeatherSetting {
    nick: String,
    location: String,
}

impl CsvRecord for WeatherSetting {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            nick: fields[0].clone(),
            location: fields[1].clone(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![self.nick.clone(), self.location.clone()]
    }
}

pub async fn date_time() -> String {
    format!("{} UTC", Utc::now().format("%H:%M:%S %d/%m/%Y"))
}

pub async fn hello(nick: &str) -> String {
    format!("Hello {}", nick)
}

pub async fn ping() -> String {
    format!("pong")
}

pub async fn reminder(args: &[String], nick: &str) -> String {
    if args.len() < 1 {
        return format!("Please provide a duration in minutes.");
    }

    let minutes: u64 = match args[0].parse() {
        Ok(minutes) => minutes,
        Err(_) => return format!("Please provide a duration in integer minutes."),
    };

    time::sleep(Duration::from_secs(minutes * 60)).await;

    format!("{nick}: Time is up!")
}

pub async fn weather(args: &[String], nick: &str, options: &HashMap<String, String>) -> String {
    let db = Database::new(
        match options.get("database_path") {
            Some(path) => path,
            None => "data/",
        },
        None,
    );

    // let mut location = args.join(" ");

    // if args.len() < 1 {
    //     let weather_settings: Vec<WeatherSetting> = match db
    //         .select("weather_settings", |wl: &WeatherSetting| {
    //             wl.nick.to_lowercase() == nick.to_lowercase()
    //         }) {
    //         Ok(weather_settings_result) => match weather_settings_result {
    //             Some(weather_settings) => weather_settings,
    //             None => return format!("Please provide a location."),
    //         },
    //         Err(_) => return format!("Please provide a location."),
    //     };

    //     if weather_settings.len() > 0 {
    //         location = weather_settings[0].location.clone();
    //     } else {
    //         return format!("Please provide a locationb.")
    //     }
    // }

    let location = match args.len() {
        ..=0 => {
            let weather_settings: Vec<WeatherSetting> = match db
                .select("weather_settings", |ws: &WeatherSetting| {
                    ws.nick.to_lowercase() == nick.to_lowercase()
                }) {
                Ok(weather_settings_result) => match weather_settings_result {
                    Some(weather_settings) => weather_settings,
                    None => return format!("Please provide a location."),
                },
                Err(_) => return format!("Please provide a location."),
            };

            if weather_settings.len() > 0 {
                weather_settings[0].location.clone()
            } else {
                return format!("Please provide a location.");
            }
        }
        _ => {
            if let Err(_) = db.delete("weather_settings", |ws: &&WeatherSetting| {
                ws.nick.to_lowercase() == nick.to_lowercase()
            }) {
                eprint!("Problem storing location.")
            }

            if let Err(_) = db.insert(
                "weather_settings",
                WeatherSetting {
                    nick: nick.to_string(),
                    location: args.join(" "),
                },
            ) {
                eprint!("Problem storing location.")
            }

            args.join(" ")
        }
    };

    match openweathermap::weather(
        &location,
        match options.get("owm_api_units") {
            Some(value) => value,
            None => "metric",
        },
        match options.get("owm_api_language") {
            Some(value) => value,
            None => "en",
        },
        match options.get("owm_api_key") {
            Some(value) => value,
            None => "",
        },
    )
    .await
    {
        Ok(current) => format!(
            "{}: {} {:.1}C | Humidity: {}% | Pressure: {}hPa | Wind: {:.1}m/s @ {} {:.1}m/s",
            current.name,
            current.weather[0].description,
            current.main.temp,
            current.main.humidity,
            current.main.pressure,
            current.wind.speed,
            current.wind.deg,
            current.wind.gust.unwrap_or_default(),
        ),
        Err(_) => format!("Could not fetch weather."),
    }
}
