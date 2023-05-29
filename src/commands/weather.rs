use crate::database::{CsvRecord, Database};
use crate::utils;
use chrono::Utc;
use chrono_tz::Tz;
use futures::join;
use openweather_sdk::{Language, OpenWeather, Units};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

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

struct TimeZone {
    nick: String,
    name: String,
}

impl CsvRecord for TimeZone {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            nick: fields[0].clone(),
            name: fields[1].clone(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![self.nick.clone(), self.name.clone()]
    }
}

pub async fn weather(
    args: &[String],
    nick: &str,
    options: &HashMap<String, String>,
    db: Arc<Mutex<Database>>,
) -> String {
    let location = match args.len() {
        ..=0 => {
            let weather_settings: Vec<WeatherSetting> = match db
                .lock()
                .await
                .select("weather_settings", |ws: &WeatherSetting| {
                    ws.nick.to_lowercase() == nick.to_lowercase()
                }) {
                Ok(weather_settings_result) => match weather_settings_result {
                    Some(weather_settings) => weather_settings,
                    None => return String::from("Please provide a location."),
                },
                Err(_) => return String::from("Please provide a location."),
            };

            if !weather_settings.is_empty() {
                weather_settings[0].location.clone()
            } else {
                return String::from("Please provide a location.");
            }
        }
        _ => {
            let entity = WeatherSetting {
                nick: String::from(nick),
                location: args.join(" "),
            };

            if db
                .lock()
                .await
                .update("weather_settings", entity, |ws: &&WeatherSetting| {
                    ws.nick.to_lowercase() == nick.to_lowercase()
                })
                .is_err()
            {
                eprintln!("Problem storing location.")
            }

            args.join(" ")
        }
    };

    let openweather = OpenWeather::new(
        match options.get("owm_api_key") {
            Some(key) => key.to_owned(),
            None => return String::from("Could not find OWM API key."),
        },
        match options.get("owm_api_units") {
            Some(units) => match units.to_lowercase().as_str() {
                "f" | "fahrenheit" | "imperial" => Units::Imperial,
                "c" | "celsius" | "metric" => Units::Metric,
                _ => Units::Standard,
            },
            None => Units::Standard,
        },
        Language::English,
    );

    let geo = match openweather
        .geocoding
        .get_geocoding(&location, None, None, 1)
        .await
    {
        Ok(geo) => {
            if !geo.is_empty() {
                geo
            } else {
                return String::from("Could not find location.");
            }
        }
        Err(err) => {
            eprintln!("{err}");

            return String::from("Could not find location.");
        }
    };

    let time_zones: Vec<TimeZone> = match db.lock().await.select("time_zones", |tz: &TimeZone| {
        tz.nick.to_lowercase() == nick.to_lowercase()
    }) {
        Ok(timezones_result) => match timezones_result {
            Some(time_zones) => time_zones,
            None => vec![TimeZone {
                nick: nick.to_owned(),
                name: String::from("Europe/Berlin"),
            }],
        },
        Err(_) => vec![TimeZone {
            nick: nick.to_owned(),
            name: String::from("Europe/Berlin"),
        }],
    };

    let current_task = async {
        match openweather.one_call.call(geo[0].lat, geo[0].lon).await {
            Ok(weather) => match weather.current {
                Some(current) => format!(
                    "{}: {} {:.1}C | Humidity: {}% | Pressure: {}hPa | Wind: {:.1}m/s @ {} {:.1}m/s\r\n",
                    utils::upper_initials(&location),
                    current.weather[0].description,
                    current.temp,
                    current.humidity,
                    current.pressure,
                    current.wind_speed,
                    current.wind_deg,
                    current.wind_gust.unwrap_or_default()
                ),
                None => String::from("Could not fetch current weather."),
            },
            Err(err) => {
                eprintln!("{err}");

                String::from("Could not fetch weather.")
            }
        }
    };

    let forecast_task = async {
        match openweather.forecast.call(geo[0].lat, geo[0].lon, 6).await {
            Ok(forecast) => forecast
                .list
                .iter()
                .take(3)
                .enumerate()
                .map(|(i, f)| {
                    let time = match format!("{} UTC", f.dt_txt).parse() {
                        Ok(time) => time,
                        Err(_) => Utc::now(),
                    };
                    let tz: Tz = match time_zones[0].name.parse() {
                        Ok(time) => time,
                        Err(_) => Tz::Europe__Berlin,
                    };
                    let time = time.with_timezone(&tz);

                    if i < 2 {
                        format!(
                            "{}: {} {:.0}C | ",
                            time.format("%H:%M %Z"),
                            f.weather[0].description,
                            f.main.temp.round()
                        )
                    } else {
                        format!(
                            "{}: {} {:.0}C",
                            time.format("%H:%M %Z"),
                            f.weather[0].description,
                            f.main.temp.round()
                        )
                    }
                })
                .collect::<String>(),
            Err(err) => {
                eprintln!("{err}");

                String::from("Could not fetch forecast.")
            }
        }
    };

    let (current, forecast) = join!(current_task, forecast_task);

    format!("{}\r\n{}", current, forecast)
}
