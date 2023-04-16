use crate::database::{CsvRecord, Database};
use chrono::{DateTime, Datelike, Days, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use irc::{client::Client, proto::Command};
use regex::Regex;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug, PartialEq)]
struct FirstStat {
    nick: String,
    points: u32,
    wins: u32,
    tz: String,
}

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
    if Utc::now().hour() > 20 {
        return String::from("STATUS: closed (closes at 21H00 UTC)");
    }

    let time_zones: Vec<TimeZone> = match db.lock().await.select("time_zones", |tz: &TimeZone| {
        tz.nick.to_lowercase() == nick.to_lowercase()
    }) {
        Ok(time_zones_result) => match time_zones_result {
            Some(time_zones) => time_zones,
            None => return String::from("Set a time zone. Example: !timezone Europe/Berlin"),
        },
        Err(_) => return String::from("Set a time zone. Example: !timezone Europe/Berlin"),
    };
    let tz: Tz = match time_zones[0].name.parse() {
        Ok(tz) => tz,
        Err(_) => {
            return String::from("Your time zone is invalid. Example: !timezone Europe/Berlin")
        }
    };

    if Utc::now().with_timezone(&tz).hour() < 6 {
        return format!("STATUS closed (opens at 06H00 {})", tz.to_string());
    }

    match db.lock().await.select("first_results", |fr: &FirstResult| {
        fr.nick.to_lowercase() == nick.to_lowercase()
            && fr.target.to_lowercase() == target.to_lowercase()
            && fr.datetime.date_naive() == Utc::now().date_naive()
    }) {
        Ok(result) => match result {
            Some(_) => return String::from("You have already played the game today."),
            None => (),
        },
        Err(_) => return String::from("Could not check your time."),
    }

    if let Err(_) = db.lock().await.insert(
        "first_results",
        FirstResult {
            nick: String::from(nick),
            target: String::from(target),
            datetime: Utc::now(),
            tz,
        },
    ) {
        return String::from("Could not register your time.");
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

    String::new()
}

pub async fn first_stats(target: &str, db: Arc<Mutex<Database>>) -> String {
    let now = Utc::now();
    let week_day = now.weekday();
    let day_number = match week_day {
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 3,
        Weekday::Thu => 4,
        Weekday::Fri => 5,
        Weekday::Sat => 6,
        Weekday::Sun => 7,
    };
    let start_date = match now.date_naive().checked_sub_days(Days::new(day_number)) {
        Some(start_date) => start_date,
        None => return String::from("Could not get stats."),
    };
    let first_results: Vec<FirstResult> =
        match db.lock().await.select("first_results", |fr: &FirstResult| {
            fr.target.to_lowercase() == target.to_lowercase()
                && fr.datetime.date_naive() > start_date
        }) {
            Ok(first_results) => match first_results {
                Some(first_results) => first_results,
                None => return String::from("Could not get results."),
            },
            Err(_) => return String::from("Could not get results."),
        };
    let mut days: HashMap<String, Vec<FirstResult>> = HashMap::new();
    let mut stats: HashMap<String, FirstStat> = HashMap::new();

    for first_result in first_results {
        days.entry(String::from(&first_result.datetime.to_string()[..=10]))
            .or_insert(Vec::default())
            .push(first_result);
    }

    let points: [u32; 10] = [25, 18, 15, 12, 10, 8, 6, 4, 2, 1];

    for (_, mut first_results) in days {
        first_results.sort_by(|a, b| {
            a.datetime
                .with_timezone(&a.tz)
                .to_string()
                .cmp(&b.datetime.with_timezone(&b.tz).to_string())
        });

        for (position, result) in first_results.into_iter().enumerate().take(10) {
            let reference = stats.entry(result.nick.clone()).or_insert(FirstStat {
                nick: String::from(result.nick.clone()),
                points: 0,
                wins: 0,
                tz: result.tz.to_string(),
            });

            reference.points += points[position];

            if position == 0 {
                reference.wins += 1;
            }
        }
    }

    let mut stats: Vec<FirstStat> = stats.into_values().collect();

    stats.sort_by(|a, b| b.points.cmp(&a.points));

    let mut output: String = Default::default();

    for (position, stat) in stats.into_iter().enumerate() {
        if stat.points > 0 {
            let re = match Regex::new(r"[^A-Za-z0-9]+") {
                Ok(re) => re,
                Err(_) => return String::from("Could not get results."),
            };
            let nick: String = re
                .replace(&stat.nick, "")
                .to_uppercase()
                .chars()
                .take(3)
                .collect();

            output = format!(
                "{}{}. {} {} ({} wins) ({}) | ",
                output,
                position + 1,
                nick,
                stat.points,
                stat.wins,
                stat.tz
            );
        }
    }

    format!("{}", output.trim_end_matches(" | "))
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

    if Utc::now().hour() < 21 {
        String::from("STATUS: open (closes at 21H00 UTC)")
    } else {
        String::from("STATUS: closed (closes at 21H00 UTC)")
    }
}
