use crate::database::{CsvRecord, Database};
use chrono::{DateTime, Utc};
use circular_queue::CircularQueue;
use irc::client::prelude::Command;
use irc::client::Client;
use itertools::Itertools;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

#[derive(Hash, PartialEq)]
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

#[derive(PartialEq)]
pub struct Interest {
    pub nick: String,
    pub tags: String,
}

impl CsvRecord for Interest {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            nick: fields[0].clone(),
            tags: fields[1].clone(),
        }
    }
    fn to_fields(&self) -> Vec<String> {
        vec![self.nick.clone(), self.tags.clone()]
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();

    t.hash(&mut s);
    s.finish()
}

pub async fn next(client: Arc<Mutex<Client>>, db: Arc<Mutex<Database>>, token: CancellationToken) {
    let mut hashes = CircularQueue::with_capacity(10);

    while !token.is_cancelled() {
        sleep(Duration::from_secs(30)).await;

        let events: Vec<Event> = match db.lock().await.select("events", |e: &Event| {
            e.datetime.signed_duration_since(Utc::now()).num_seconds() <= 300
                && e.datetime.signed_duration_since(Utc::now()).num_seconds() > 240
        }) {
            Ok(events) => match events {
                Some(events) => events
                    .into_iter()
                    .sorted_by(|a, b| a.datetime.cmp(&b.datetime))
                    .collect(),
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

                if event.notify {
                    let interests: Option<Vec<Interest>> =
                        match db.lock().await.select("interests", |_| true) {
                            Ok(interests) => interests,
                            Err(_) => None,
                        };

                    if let Some(interests) = interests {
                        let mut mentions: String = String::new();
                        let event_tags_set: HashSet<&str> = event.tags.split_whitespace().collect();

                        for i in interests {
                            let user_tags_set: HashSet<&str> = i.tags.split_whitespace().collect();
                            if event_tags_set.intersection(&user_tags_set).next().is_some() {
                                mentions = format!("{} {}", mentions, i.nick);
                            }
                        }

                        if !mentions.is_empty() {
                            if let Err(error) = client
                                .lock()
                                .await
                                .send(Command::PRIVMSG(event.channel.clone(), mentions))
                            {
                                eprintln!("{error}");
                            }
                        }
                    }
                }

                hashes.push(hash);
            }
        }
    }
}
