use crate::database::{CsvRecord, Database};
use chrono::{DateTime, Timelike, Utc};
use chrono_tz::Tz;
use irc::{client::Client, proto::Command};
use regex::Regex;
use std::{str::FromStr, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug, PartialEq)]
struct FirstResult {
    nick: String,
    target: String,
    datetime: DateTime<Utc>,
    tz: Tz,
}

impl CsvRecord for FirstResult {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            nick: fields[0].clone(),
            target: fields[1].clone(),
            datetime: match fields[2].parse() {
                Ok(datetime) => datetime,
                Err(_) => Utc::now(),
            },
            tz: Tz::from_str(&fields[3].clone()).unwrap_or(Tz::Europe__Berlin),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![
            self.nick.clone(),
            self.target.clone(),
            self.datetime.to_string(),
            self.tz.to_string(),
        ]
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

async fn show_results(
    first_results: &mut [FirstResult],
    nick: Option<&str>,
    target: &str,
    client: Arc<Mutex<Client>>,
) {
    first_results.sort_by(|a, b| {
        a.datetime
            .with_timezone(&a.tz)
            .to_string()
            .cmp(&b.datetime.with_timezone(&b.tz).to_string())
    });

    for (position, result) in first_results.into_iter().take(5).enumerate() {
        let re = match Regex::new(r"[^A-Za-z0-9]+") {
            Ok(re) => re,
            Err(_) => {
                if let Err(error) = client.lock().await.send(Command::PRIVMSG(
                    String::from(target),
                    String::from("Could not print results."),
                )) {
                    eprintln!("{error}");
                }

                return;
            }
        };

        match nick {
            None => {
                let nick = re.replace_all(&result.nick, "").to_uppercase();

                if let Err(error) = client.lock().await.send(Command::PRIVMSG(
                    String::from(target),
                    format!(
                        "{}. {} | {} ({})",
                        position + 1,
                        &nick[..3],
                        result.datetime.with_timezone(&result.tz).time().to_string(),
                        result
                            .datetime
                            .with_timezone(&result.tz)
                            .timezone()
                            .to_string(),
                    ),
                )) {
                    eprintln!("{error}");
                }
            }
            Some(nick) => {
                if nick.to_lowercase() == result.nick.to_lowercase() {
                    if let Err(error) = client.lock().await.send(Command::PRIVMSG(
                        String::from(target),
                        format!("You are currently P{}.", position + 1),
                    )) {
                        eprintln!("{error}");
                    }
                }
            }
        }
    }
}

pub async fn first(
    nick: &str,
    target: &str,
    db: Arc<Mutex<Database>>,
    client: Arc<Mutex<Client>>,
) -> String {
    if Utc::now().hour() > 14 {
        return String::from("STATUS: closed (deadline is 15H00 UTC)");
    }

    let time_zones: Vec<TimeZone> = match db.lock().await.select("time_zones", |tz: &TimeZone| {
        tz.nick.to_lowercase() == nick.to_lowercase()
    }) {
        Ok(time_zones_result) => match time_zones_result {
            Some(time_zones) => time_zones,
            None => {
                return String::from(
                    "Could not get your time zone.\r\n
                    To play this game set one with the timezone command.\r\n
                    Example: !timezone Europe/Berlin",
                )
            }
        },
        Err(_) => {
            return String::from(
                "Could not get your time zone.\r\n
                To play this game set one with the timezone command.\r\n
                Example: !timezone Europe/Berlin",
            )
        }
    };
    let tz: Tz = match time_zones[0].name.parse() {
        Ok(tz) => tz,
        Err(_) => return String::from("Your time zone is invalid."),
    };

    if let Err(_) = db.lock().await.update(
        "first_results",
        FirstResult {
            nick: String::from(nick),
            target: String::from(target),
            datetime: Utc::now(),
            tz,
        },
        |fr: &&FirstResult| {
            fr.nick.to_lowercase() == nick.to_lowercase()
                && fr.target.to_lowercase() == target.to_lowercase()
        },
    ) {
        return String::from("Problem registering your time.");
    }

    let mut first_results: Vec<FirstResult> =
        match db.lock().await.select("first_results", |fr: &FirstResult| {
            Utc::now().date_naive() == fr.datetime.date_naive()
                && fr.target.to_lowercase() == target.to_lowercase()
        }) {
            Ok(first_results) => match first_results {
                Some(first_results) => first_results,
                None => return String::from("Could not get results."),
            },
            Err(_) => return String::from("Could not get results."),
        };

    show_results(&mut first_results, Some(nick), target, client).await;

    String::from("Check the full results with !first_results.")
}

pub async fn first_results(
    target: &str,
    db: Arc<Mutex<Database>>,
    client: Arc<Mutex<Client>>,
) -> String {
    let mut first_results: Vec<FirstResult> =
        match db.lock().await.select("first_results", |fr: &FirstResult| {
            Utc::now().date_naive() == fr.datetime.date_naive()
                && fr.target.to_lowercase() == target.to_lowercase()
        }) {
            Ok(first_results) => match first_results {
                Some(first_results) => first_results,
                None => return String::from("Could not get results."),
            },
            Err(_) => return String::from("Could not get results."),
        };

    show_results(&mut first_results, None, target, client).await;

    if Utc::now().hour() < 15 {
        String::from("STATUS: open (deadline is 15H00 UTC)")
    } else {
        String::from("STATUS: closed (deadline is 15H00 UTC)")
    }
}
