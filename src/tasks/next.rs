use crate::database::{CsvRecord, Database};
use chrono::{DateTime, Utc};
use circular_queue::CircularQueue;
use irc::client::prelude::Command;
use irc::client::Client;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Hash, PartialEq)]
struct Event {
    category: String,
    name: String,
    description: String,
    datetime: DateTime<Utc>,
    channel: String,
    tags: String,
    announced: bool,
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
            announced: fields[6].parse().unwrap_or(false),
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
            self.announced.to_string(),
        ]
    }
}

#[derive(PartialEq)]
pub struct Notification {
    pub channel: String,
    pub mentions: String,
}

impl CsvRecord for Notification {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            channel: fields[0].clone(),
            mentions: fields[1].clone(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![self.channel.clone(), self.mentions.clone()]
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();

    t.hash(&mut s);
    s.finish()
}

pub async fn next(client: Arc<Mutex<Client>>, db: Arc<Mutex<Database>>) {
    let mut hashes = CircularQueue::with_capacity(10);

    loop {
        sleep(Duration::from_secs(30)).await;

        let events: Vec<Event> = match db.lock().await.select("events", |e: &Event| {
            e.datetime.signed_duration_since(Utc::now()).num_seconds() <= 300
                && e.datetime.signed_duration_since(Utc::now()).num_seconds() > 240
        }) {
            Ok(events) => match events {
                Some(events) => events,
                None => continue,
            },
            Err(_) => {
                eprintln!("Could not get events.");

                continue;
            }
        };

        for event in events {
            let hash = calculate_hash(&event);

            if !hashes.iter().any(|h| h == &hash) {
                if let Err(error) = client.lock().await.send(Command::PRIVMSG(
                    event.channel.clone(),
                    format!(
                        "\x034Starting in 5 minutes:\x03 \x02{} {} {}\x02",
                        event.category, event.name, event.description
                    ),
                )) {
                    eprintln!("{error}");
                }

                let notifications: Option<Vec<Notification>> =
                    match db.lock().await.select("notifications", |n: &Notification| {
                        n.channel.to_lowercase() == event.channel.clone()
                    }) {
                        Ok(notifications) => notifications,
                        Err(_) => None,
                    };

                if let Some(notifications) = notifications {
                    if let Err(error) = client.lock().await.send(Command::PRIVMSG(
                        event.channel.clone(),
                        notifications[0].mentions.clone(),
                    )) {
                        eprintln!("{error}");
                    }
                }

                hashes.push(hash);
            }
        }
    }
}
