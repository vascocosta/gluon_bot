use crate::database::{CsvRecord, Database};
use crate::tasks::next::Interest;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub category: String,
    pub name: String,
    pub description: String,
    pub datetime: DateTime<Utc>,
    pub channel: String,
    pub tags: String,
    pub notify: bool,
}

impl CsvRecord for Event {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            category: fields[0].clone(),
            name: fields[1].clone(),
            description: fields[2].clone(),
            datetime: match fields[3].parse() {
                Ok(datetime) => datetime,
                Err(_) => Utc::now(),
            },
            channel: fields[4].clone(),
            tags: fields[5].clone(),
            notify: fields[6].parse().unwrap_or_default(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![
            self.category.clone(),
            self.name.clone(),
            self.description.clone(),
            self.datetime.to_string(),
            self.channel.clone(),
            self.tags.clone(),
            self.notify.to_string(),
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

pub async fn next(args: &[String], nick: &str, target: &str, db: Arc<Mutex<Database>>) -> String {
    let search = &args.join(" ").to_lowercase();
    let mut events: Vec<Event> = match db.lock().await.select("events", |e: &Event| {
        e.datetime > Utc::now()
            && e.channel.to_lowercase() == target.to_lowercase()
            && (e.category.to_lowercase().contains(search)
                || e.description.to_lowercase().contains(search)
                || e.tags.to_lowercase().contains(search))
    }) {
        Ok(events_result) => match events_result {
            Some(events) => events,
            None => return String::from("Could not get events."),
        },
        Err(_) => return String::from("Could not get events."),
    };

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

    if !events.is_empty() {
        let tz: Tz = match time_zones[0].name.parse() {
            Ok(tz) => tz,
            Err(_) => Tz::CET,
        };

        events.sort_by(|a, b| a.datetime.cmp(&b.datetime));

        let duration = events[0].datetime.signed_duration_since(Utc::now());

        format!(
            "{} | {} {} {} | {} day(s), {} hour(s), {} minute(s)",
            events[0]
                .datetime
                .with_timezone(&tz)
                .format("%A, %d %B at %H:%M %Z (UTC%:z)"),
            events[0].category,
            events[0].name,
            events[0].description,
            duration.num_days(),
            duration.num_hours() % 24,
            duration.num_minutes() % 60
        )
    } else {
        String::from("Could not find next event.")
    }
}

pub async fn interests(args: &[String], nick: &str, db: Arc<Mutex<Database>>) -> String {
    if args.is_empty() {
        match db.lock().await.select("interests", |i: &Interest| {
            i.nick.to_lowercase() == nick.to_lowercase()
        }) {
            Ok(Some(interests)) => return interests[0].tags.clone(),
            _ => return String::from("Could not get interests."),
        };
    }

    let interest = Interest {
        nick: nick.to_string(),
        tags: args.join(" "),
    };

    match db
        .lock()
        .await
        .update("interests", interest, |i: &&Interest| {
            i.nick.to_lowercase() == nick.to_lowercase()
        }) {
        Ok(_) => String::from("Your interests were updated."),
        Err(_) => String::from("Could not update your interests."),
    }
}
