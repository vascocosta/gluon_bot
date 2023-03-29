use crate::database::{CsvRecord, Database};
use chrono::{DateTime, Utc};
use feed_rs::parser;
use irc::client::prelude::Command;
use irc::client::Client;
use std::collections::HashMap;
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
            id: fields[0].parse().unwrap_or_default(),
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

pub async fn feeds(
    options: Arc<HashMap<String, String>>,
    client: Arc<Mutex<Client>>,
    db: Arc<Mutex<Database>>,
) {
    loop {
        sleep(Duration::from_secs(match options.get("feed_refresh") {
            Some(feed_refresh) => match feed_refresh.parse() {
                Ok(feed_refresh) => feed_refresh,
                Err(_) => 300,
            },
            None => 300,
        }))
        .await;

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
                let id = feed.id;
                let category = feed.category;
                let url = feed.url;
                let channel = feed.channel;
                let mut last_modified = feed.published;
                let feed = match reqwest::get(url.clone()).await {
                    Ok(response) => match response.text().await {
                        Ok(feed) => feed,
                        Err(_) => return,
                    },
                    Err(_) => return,
                };
                let feed = match parser::parse(feed.as_bytes()) {
                    Ok(feed) => feed,
                    Err(_) => return,
                };

                let mut entries = feed.entries;
                entries.sort_by(|a, b| b.published.cmp(&a.published));

                for entry in entries {
                    let entry_published = match entry.published {
                        Some(entry_published) => entry_published,
                        None => return,
                    };

                    if entry_published > last_modified {
                        if let Err(error) = client_clone.lock().await.send(Command::PRIVMSG(
                            channel.clone(),
                            match entry.title {
                                Some(title) => format!("\x02[{}]\x02", title.content),
                                None => String::from(""),
                            },
                        )) {
                            eprintln!("{error}");
                        }

                        if let Err(error) = client_clone.lock().await.send(Command::PRIVMSG(
                            channel.clone(),
                            entry.links[0].href.clone(),
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
                                published: entry_published,
                            },
                            |f: &&Feed| f.id == id,
                        ) {
                            println!("Problem updating published time.");
                        }

                        last_modified = entry_published;
                    }
                }
            });
        }
    }
}
