use crate::database::{CsvRecord, Database};
use chrono::{DateTime, Datelike, Days, Duration, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use irc::{client::Client, proto::Command};
use rand::prelude::*;
use regex::Regex;
use std::{collections::HashMap, ops::Range, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

const DEFAULT_OPEN_HOUR: u32 = 5;
const DEFAULT_OPEN_MIN: u32 = 0;
const DEFAULT_CLOSE_HOUR: u32 = 21;
const DEFAULT_CLOSE_MIN: u32 = 0;
const RAND_OPEN_HOUR: Range<u32> = 5..12;
const RAND_OPEN_MIN: Range<u32> = 0..59;

pub enum Period {
    Week,
    Weekly,
    Month,
    Monthly,
    Year,
    Yearly,
    Unknown,
}

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

fn open_time(options: &HashMap<String, String>, utc_now: DateTime<Utc>, next: bool) -> (u32, u32) {
    let open_hour = match options.get("first_open_hour") {
        Some(open_hour) => match open_hour.parse() {
            Ok(open_hour) => match open_hour {
                0..=23 => open_hour,
                _ => DEFAULT_OPEN_HOUR,
            },
            Err(_) => {
                let mut rng = if next {
                    StdRng::seed_from_u64((utc_now + Duration::days(1)).day() as u64)
                } else {
                    StdRng::seed_from_u64(utc_now.day() as u64)
                };

                rng.gen_range(RAND_OPEN_HOUR)
            }
        },
        None => 5,
    };
    let open_min = match options.get("first_open_min") {
        Some(open_min) => match open_min.parse() {
            Ok(open_min) => match open_min {
                0..=59 => open_min,
                _ => DEFAULT_OPEN_MIN,
            },
            Err(_) => {
                let mut rng = if next {
                    StdRng::seed_from_u64((utc_now + Duration::days(1)).day() as u64)
                } else {
                    StdRng::seed_from_u64(utc_now.day() as u64)
                };

                rng.gen_range(RAND_OPEN_MIN)
            }
        },
        None => 30,
    };

    (open_hour, open_min)
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

    for (position, result) in first_results.iter().take(10).enumerate() {
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
                        result.datetime.with_timezone(&result.tz).time(),
                        result.datetime.with_timezone(&result.tz).timezone()
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

                    break;
                }
            }
        }
    }
}

pub async fn first(
    nick: &str,
    target: &str,
    options: &HashMap<String, String>,
    db: Arc<Mutex<Database>>,
    client: Arc<Mutex<Client>>,
) -> String {
    let utc_now = Utc::now();
    let close_hour = match options.get("first_close_hour") {
        Some(close_hour) => match close_hour.parse() {
            Ok(close_hour) => match close_hour {
                0..=23 => close_hour,
                _ => DEFAULT_CLOSE_HOUR,
            },
            Err(_) => DEFAULT_CLOSE_HOUR,
        },
        None => DEFAULT_CLOSE_HOUR,
    };
    let close_min = match options.get("first_close_min") {
        Some(close_min) => match close_min.parse() {
            Ok(close_min) => match close_min {
                0..=59 => close_min,
                _ => DEFAULT_CLOSE_MIN,
            },
            Err(_) => DEFAULT_CLOSE_MIN,
        },
        None => DEFAULT_CLOSE_MIN,
    };

    if utc_now.hour() > close_hour
        || (utc_now.hour() == close_hour && utc_now.minute() >= close_min)
    {
        return format!(
            "STATUS: closed (closes at {:0>2}H{:0>2} UTC)",
            close_hour, close_min,
        );
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
    let tz: Tz = match time_zones
        .first()
        .unwrap_or(&TimeZone {
            nick: String::from(nick),
            name: String::from("Europe/Berlin"),
        })
        .name
        .parse()
    {
        Ok(tz) => tz,
        Err(_) => {
            return String::from("Your time zone is invalid. Example: !timezone Europe/Berlin")
        }
    };
    let tz_now = utc_now.with_timezone(&tz);
    let open_time_today = open_time(options, utc_now, false);
    let open_hour = open_time_today.0;
    let open_min = open_time_today.1;
    let open_time_next = open_time(options, utc_now, true);
    let open_hour_next = open_time_next.0;
    let open_min_next = open_time_next.1;

    if tz_now.hour() < open_hour || (tz_now.hour() == open_hour && tz_now.minute() < open_min) {
        return format!(
            "STATUS closed (opens at {:0>2}H{:0>2} {})",
            open_hour, open_min, tz
        );
    }

    match db.lock().await.select("first_results", |fr: &FirstResult| {
        fr.nick.to_lowercase() == nick.to_lowercase()
            && fr.target.to_lowercase() == target.to_lowercase()
            && fr.datetime.date_naive() == utc_now.date_naive()
    }) {
        Ok(result) => {
            if result.is_some() && !result.unwrap().is_empty() {
                return format!(
                    "STATUS played (today opened at {:0>2}H{:0>2} {} | tomorrow opens at {:0>2}H{:0>2} {})",
                    open_hour, open_min, tz, open_hour_next, open_min_next, tz
                );
            }
        }
        Err(_) => return String::from("Could not check your time."),
    }

    if db
        .lock()
        .await
        .insert(
            "first_results",
            FirstResult {
                nick: String::from(nick),
                target: String::from(target),
                datetime: utc_now,
                tz,
            },
        )
        .is_err()
    {
        return String::from("Could not register your time.");
    }

    let mut first_results: Vec<FirstResult> =
        match db.lock().await.select("first_results", |fr: &FirstResult| {
            utc_now.date_naive() == fr.datetime.date_naive()
                && fr.target.to_lowercase() == target.to_lowercase()
        }) {
            Ok(first_results) => match first_results {
                Some(first_results) => first_results,
                None => return String::from("Could not get results."),
            },
            Err(_) => return String::from("Could not get results."),
        };

    show_results(&mut first_results, Some(nick), target, client).await;

    format!(
        "STATUS open (today opened at {:0>2}H{:0>2} {} | tomorrow opens at {:0>2}H{:0>2} {})",
        open_hour, open_min, tz, open_hour_next, open_min_next, tz
    )
}

pub async fn first_stats(args: &[String], target: &str, db: Arc<Mutex<Database>>) -> String {
    match args.concat().to_lowercase().as_str() {
        "week" => stats(Period::Week, target, db).await,
        "weekly" => stats(Period::Weekly, target, db).await,
        "month" => stats(Period::Month, target, db).await,
        "monthly" => stats(Period::Monthly, target, db).await,
        "year" => stats(Period::Year, target, db).await,
        "yearly" => stats(Period::Yearly, target, db).await,
        _ => stats(Period::Unknown, target, db).await,
    }
}

pub async fn stats(period: Period, target: &str, db: Arc<Mutex<Database>>) -> String {
    let now = Utc::now();
    let day_number = match period {
        Period::Week => {
            let week_day = now.weekday();

            match week_day {
                Weekday::Mon => 1,
                Weekday::Tue => 2,
                Weekday::Wed => 3,
                Weekday::Thu => 4,
                Weekday::Fri => 5,
                Weekday::Sat => 6,
                Weekday::Sun => 7,
            }
        }
        Period::Weekly => 7,
        Period::Month => now.day() as u64,
        Period::Monthly => 30,
        Period::Year => now
            .signed_duration_since(
                DateTime::parse_from_str(
                    format!("{}-12-31 11:59 pm +0000", now.year() - 1).as_str(),
                    "%Y-%m-%d %H:%M %P %z",
                )
                .unwrap_or_default(),
            )
            .num_days() as u64,
        Period::Yearly => 365,
        Period::Unknown => {
            return String::from(
                "Argument must be one of: week, weekly, month, monthly, year, yearly.",
            )
        }
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
            .or_default()
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
                nick: result.nick.clone(),
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

    for (position, stat) in stats.into_iter().take(10).enumerate() {
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

    output.trim_end_matches(" | ").to_string()
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

    String::new()
}
