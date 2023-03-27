use crate::database::{CsvRecord, Database};
use chrono::{DateTime, Utc};
use feed_rs::model::Category;
use feed_rs::parser;
use irc::client::prelude::Command;
use irc::client::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time::{sleep, Duration};

#[derive(Debug, PartialEq)]
struct Feed {
    id: u32,
    category: String,
    url: String,
    channel: String,
    published: DateTime<Utc>,
}

impl CsvRecord for Feed {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            id: fields[0].parse().unwrap(),
            category: fields[1].clone(),
            url: fields[2].clone(),
            channel: fields[3].clone(),
            published: match fields[4].parse() {
                Ok(published) => published,
                Err(_) => Utc::now(),
            },
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![
            self.id.to_string(),
            self.category.clone(),
            self.url.clone(),
            self.channel.clone(),
            self.published.to_string(),
        ]
    }
}

pub async fn feeds(client: Arc<Mutex<Client>>, db: Arc<Mutex<Database>>) {
    loop {
        sleep(Duration::from_secs(20)).await;

        let feeds: Vec<Feed> = match db.lock().await.select("feeds", |_| true) {
            Ok(feeds) => match feeds {
                Some(feeds) => feeds,
                None => continue,
            },
            Err(_) => {
                eprintln!("Could not get feeds.");

                continue;
            }
        };
        for feed in feeds {
            let client_clone = Arc::clone(&client);
            let db_clone = Arc::clone(&db);

            task::spawn(async move {
                let url = feed.url;
                let category = feed.category;
                let channel = feed.channel;
                let last_modified = feed.published;
                let id = feed.id;
                let feed = reqwest::get(url.clone())
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap();
                let feed = parser::parse(feed.as_bytes()).unwrap();

                for entry in feed.entries {
                    if entry.published.unwrap() > last_modified {
                        if let Err(error) = client_clone.lock().await.send(Command::PRIVMSG(
                            channel.clone(),
                            format!(
                                "{} - {}",
                                entry.title.unwrap().content,
                                entry.links[0].href
                            ),
                        )) {
                            eprintln!("{error}");
                        }

                        if let Err(_) = db_clone.lock().await.update(
                            "feeds",
                            Feed {
                                id: id,
                                category: category.clone(),
                                url: url.clone(),
                                channel: channel.clone(),
                                published: entry.published.unwrap(),
                            },
                            |f: &&Feed| f.id == id,
                        ) {
                            println!("Problem updating published time.");
                        }
                    }
                }
            });
        }
    }
}
