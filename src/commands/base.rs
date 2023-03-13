use crate::database::{CsvRecord, Database};
use chrono::Utc;
use rand::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;

struct Answer {
    answer: String,
}

impl CsvRecord for Answer {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            answer: fields[0].clone(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![self.answer.clone()]
    }
}

struct Quote {
    date: String,
    text: String,
    channel: String,
}

impl CsvRecord for Quote {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            date: fields[0].clone(),
            text: fields[1].clone(),
            channel: fields[2].clone(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![self.date.clone(), self.text.clone(), self.channel.clone()]
    }
}

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

pub async fn ask(args: &[String], db: Arc<Mutex<Database>>) -> String {
    if args.len() < 1 {
        return String::from("Please provide a question.");
    }

    let answers: Vec<Answer> = match db.lock().await.select("answers", |_| true) {
        Ok(answers_result) => match answers_result {
            Some(answers) => answers,
            None => return String::from("Could not find answer."),
        },
        Err(_) => return String::from("Could not find answer."),
    };

    let mut rng = StdRng::from_entropy();
    let index = rng.gen_range(0..answers.len());

    answers[index].answer.clone()
}

pub async fn date_time() -> String {
    format!("{} UTC", Utc::now().format("%H:%M:%S %d/%m/%Y"))
}

pub async fn hello(nick: &str) -> String {
    format!("Hello {}", nick)
}

pub async fn ping() -> String {
    String::from("pong")
}

pub async fn quote(args: &[String], target: &str, db: Arc<Mutex<Database>>) -> String {
    if args.len() == 0 {
        let quotes: Vec<Quote> = match db
            .lock()
            .await
            .select("quotes", |q: &Quote| q.channel.to_lowercase() == target)
        {
            Ok(quotes_result) => match quotes_result {
                Some(quotes) => quotes,
                None => return String::from("Could not find quotes."),
            },
            Err(_) => return String::from("Could not find quotes."),
        };

        if quotes.len() == 0 {
            return String::from("Could not find quotes.");
        }

        let mut rng = StdRng::from_entropy();
        let index = rng.gen_range(0..quotes.len());

        return format!("{} {}", quotes[index].date, quotes[index].text);
    } else {
        match db.lock().await.insert(
            "quotes",
            Quote {
                date: Utc::now().format("%d-%m-%Y").to_string(),
                text: args.join(" "),
                channel: String::from(target),
            },
        ) {
            Ok(_) => return String::from("Quote added successfully."),
            Err(_) => return String::from("Problem adding quote."),
        }
    }
}

pub async fn reminder(args: &[String], nick: &str) -> String {
    if args.len() < 1 {
        return String::from("Please provide a duration in minutes.");
    }

    let minutes: u64 = match args[0].parse() {
        Ok(minutes) => minutes,
        Err(_) => return String::from("Please provide a duration in integer minutes."),
    };

    time::sleep(Duration::from_secs(minutes * 60)).await;

    format!("{nick}: Time is up!")
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

            if weather_settings.len() > 0 {
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

            if let Err(_) =
                db.lock()
                    .await
                    .update("weather_settings", entity, |ws: &&WeatherSetting| {
                        ws.nick.to_lowercase() == nick.to_lowercase()
                    })
            {
                eprintln!("Problem storing location.")
            }

            args.join(" ")
        }
    };

    match openweathermap::blocking::weather(
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
    ) {
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
        Err(_) => String::from("Could not fetch weather."),
    }
}
