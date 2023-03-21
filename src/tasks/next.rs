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
            announced: match fields[6].parse() {
                Ok(announced) => announced,
                Err(_) => false,
            },
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

        let hash = calculate_hash(&events[0]);

        if let None = hashes.iter().find(|h| h == &&hash) {
            if let Err(error) = client.lock().await.send(Command::PRIVMSG(
                events[0].channel.clone(),
                format!(
                    "Starting in 5 minutes: {} {} {}",
                    events[0].category, events[0].name, events[0].description
                ),
            )) {
                eprintln!("{error}");
            }

            hashes.push(hash);
        }

        // let events: Vec<Event> = match db
        //     .lock()
        //     .await
        //     .select("events", |e: &Event| e.announced == false)
        // {
        //     Ok(events) => match events {
        //         Some(events) => events,
        //         None => continue,
        //     },
        //     Err(_) => {
        //         eprintln!("Could not get events.");

        //         continue;
        //     }
        // };

        // let duration = events[0].datetime.signed_duration_since(Utc::now());

        // println!("{}", duration.num_seconds());

        // if duration.num_seconds() <= 300 && duration.num_seconds() > 240 {
        //     if let Err(error) = client.lock().await.send(Command::PRIVMSG(
        //         events[0].channel.clone(),
        //         format!(
        //             "Starting in 5 minutes: {} {} {}",
        //             events[0].category, events[0].name, events[0].description
        //         ),
        //     )) {
        //         eprintln!("{error}");
        //     }

        //     let event = Event {
        //         category: events[0].category.clone(),
        //         name: events[0].name.clone(),
        //         description: events[0].description.clone(),
        //         datetime: events[0].datetime,
        //         tags: events[0].tags.clone(),
        //         channel: events[0].channel.clone(),
        //         announced: true,
        //     };

        //     if let Err(_) = db.lock().await.update("events", event, |e: &&Event| {
        //         e.category == events[0].category
        //             && e.name == events[0].name
        //             && e.description == events[0].description
        //             && e.datetime == events[0].datetime
        //             && e.tags == events[0].tags
        //             && e.channel == events[0].channel
        //             && e.announced == events[0].announced
        //     }) {
        //         eprintln!("Problem updating event.")
        //     }
        // }
    }
}