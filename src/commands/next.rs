use crate::database::{CsvRecord, Database};
use crate::tasks::next::Notification;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use std::sync::Arc;
use tokio::sync::Mutex;

struct Event {
    category: String,
    name: String,
    description: String,
    datetime: DateTime<Utc>,
    channel: String,
    tags: String,
    notify: bool,
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
    let mut events: Vec<Event> = match db.lock().await.select("events", |e: &Event| {
        e.datetime > Utc::now()
            && e.channel.to_lowercase() == target.to_lowercase()
            && (e
                .category
                .to_lowercase()
                .contains(&args.join(" ").to_lowercase())
                || e.description
                    .to_lowercase()
                    .contains(&args.join(" ").to_lowercase())
                || e.tags
                    .to_lowercase()
                    .contains(&args.join(" ").to_lowercase()))
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

pub async fn notify(nick: &str, target: &str, db: Arc<Mutex<Database>>) -> String {
    let notifications: Option<Vec<Notification>> =
        match db.lock().await.select("notifications", |n: &Notification| {
            n.channel.to_lowercase() == target.to_lowercase()
        }) {
            Ok(notifications) => notifications,
            Err(_) => return String::from("Could not get notifications."),
        };

    match notifications {
        Some(notifications) => {
            if notifications[0]
                .mentions
                .to_lowercase()
                .contains(&nick.to_lowercase())
            {
                match db.lock().await.update(
                    "notifications",
                    Notification {
                        channel: notifications[0].channel.clone(),
                        mentions: notifications[0]
                            .mentions
                            .replace(&nick.to_lowercase(), "")
                            .trim()
                            .to_string(),
                    },
                    |n: &&Notification| n.channel.to_lowercase() == target.to_lowercase(),
                ) {
                    Ok(_) => String::from("Notifications disabled on this channel."),
                    Err(_) => String::from("Could not update notifications for this channel."),
                }
            } else {
                match db.lock().await.update(
                    "notifications",
                    Notification {
                        channel: notifications[0].channel.clone(),
                        mentions: format!("{} {}", notifications[0].mentions, nick),
                    },
                    |n: &&Notification| n.channel.to_lowercase() == target.to_lowercase(),
                ) {
                    Ok(_) => String::from("Notifications enabled on this channel."),
                    Err(_) => String::from("Could not update notifications for this channel."),
                }
            }
        }
        None => match db.lock().await.insert(
            "notifications",
            Notification {
                channel: String::from(target),
                mentions: String::from(nick),
            },
        ) {
            Ok(_) => String::from("Notifications enabled on this channel."),
            Err(_) => String::from("Could not update notifications for this channel."),
        },
    }
}
