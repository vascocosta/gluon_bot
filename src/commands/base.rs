use crate::database::{CsvRecord, Database};
use chrono::{DateTime, Datelike, Offset, Utc};
use chrono_tz::Tz;
use irc::client::prelude::Command;
use irc::client::Client;
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

pub async fn alarm(
    args: &[String],
    nick: &str,
    target: &str,
    db: Arc<Mutex<Database>>,
    client: Arc<Mutex<Client>>,
) -> String {
    if args.len() < 1 {
        return String::from("Please provide a time in your time zone.");
    }

    let time_zones: Vec<TimeZone> = match db.lock().await.select("time_zones", |tz: &TimeZone| {
        tz.nick.to_lowercase() == nick.to_lowercase()
    }) {
        Ok(time_zones_result) => match time_zones_result {
            Some(time_zones) => time_zones,
            None => vec![TimeZone {
                nick: String::new(),
                name: String::from("Europe/Berlin"),
            }],
        },
        Err(_) => vec![TimeZone {
            nick: String::new(),
            name: String::from("Europe/Berlin"),
        }],
    };
    let tz: Tz = match time_zones[0].name.parse() {
        Ok(tz) => tz,
        Err(_) => Tz::CET,
    };
    let now = Utc::now();
    let now = now.with_timezone(&tz);
    let utc_offset = now.offset().fix().local_minus_utc();
    let now = Utc::now();
    let day = now.day();
    let month = now.month();
    let year = now.year();
    let alarm_str = format!("{year}-{month}-{day} {}:00 UTC", args[0]);
    let alarm_dt: DateTime<Utc> = match alarm_str.parse() {
        Ok(alarm_dt) => alarm_dt,
        Err(_) => return String::from("Please provide a time in your time zone (ex: 18:30)."),
    };
    let alarm_dt = alarm_dt - chrono::Duration::seconds(utc_offset as i64);
    let duration = alarm_dt - Utc::now();

    if let Err(error) = client.lock().await.send(Command::PRIVMSG(
        String::from(target),
        format!("Alarm set to {} {}.", args[0], tz.to_string()),
    )) {
        eprintln!("{error}");
    }

    if duration.num_seconds() > 0 {
        if let Err(error) = client.lock().await.send(Command::PRIVMSG(
            String::from(target),
            format!(
                "Up in {} hours and {} minutes.",
                duration.num_hours(),
                duration.num_minutes() % 60
            ),
        )) {
            eprintln!("{error}");
        }

        time::sleep(Duration::from_secs(duration.num_seconds() as u64)).await;
    } else {
        let corrected_duration = Duration::from_secs((duration.num_seconds() + 86400) as u64);

        if let Err(error) = client.lock().await.send(Command::PRIVMSG(
            String::from(target),
            format!(
                "Up in {} hours and {} minutes.",
                corrected_duration.as_secs() / (60 * 60),
                (corrected_duration.as_secs() / 60) % 60
            ),
        )) {
            eprintln!("{error}");
        }

        time::sleep(corrected_duration).await;
    }

    if args.len() > 1 {
        format!("{}: {}", nick, args[1..].join(" "))
    } else {
        format!("{}: Alarm is up!", nick)
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

pub async fn help() -> String {
    String::from(
        "Command list: \
        alarm | ask | city | date | f1results | first | first_results | first_stats | hello | help | \
        imdb | next | notify | ping | quote | rates | remind | timezone | weather",
    )
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

pub async fn reminder(
    args: &[String],
    nick: &str,
    target: &str,
    client: Arc<Mutex<Client>>,
) -> String {
    if args.len() < 1 {
        return String::from("Please provide a duration in minutes.");
    }

    let minutes: u64 = match args[0].parse() {
        Ok(minutes) => minutes,
        Err(_) => return String::from("Please provide a duration in integer minutes."),
    };

    if let Err(error) = client.lock().await.send(Command::PRIVMSG(
        String::from(target),
        format!("Reminder set for {} minute(s) from now.", minutes),
    )) {
        eprintln!("{error}");
    }

    time::sleep(Duration::from_secs(minutes * 60)).await;

    if args.len() > 1 {
        format!("{}: {}", nick, args[1..].join(" "))
    } else {
        format!("{}: Time is up!", nick)
    }
}

pub async fn time_zone(args: &[String], nick: &str, db: Arc<Mutex<Database>>) -> String {
    let time_zones: Vec<TimeZone> = match db.lock().await.select("time_zones", |tz: &TimeZone| {
        tz.nick.to_lowercase() == nick.to_lowercase()
    }) {
        Ok(time_zones_result) => match time_zones_result {
            Some(time_zones) => time_zones,
            None => vec![TimeZone {
                nick: String::new(),
                name: String::from("Europe/Berlin"),
            }],
        },
        Err(_) => vec![TimeZone {
            nick: String::new(),
            name: String::from("Europe/Berlin"),
        }],
    };
    let tz: Tz = match time_zones[0].name.parse() {
        Ok(tz) => tz,
        Err(_) => Tz::CET,
    };

    if args.len() == 0 {
        format!("Your current time zone: {}", tz.to_string())
    } else {
        match db.lock().await.update(
            "time_zones",
            TimeZone {
                nick: String::from(nick),
                name: args.concat(),
            },
            |tz: &&TimeZone| tz.nick.to_lowercase() == nick.to_lowercase(),
        ) {
            Ok(_) => String::from("Your time zone was successfully updated."),
            Err(_) => String::from("Problem updating your time zone."),
        }
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
        Err(_) => String::from("Could not fetch weather."),
    }
}
