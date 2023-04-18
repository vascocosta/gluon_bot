mod base;
mod city;
mod f1results;
mod first;
mod next;
mod omdb;
mod rates;

use crate::database::Database;
use irc::client::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct BotCommand<'a> {
    name: String,
    args: Vec<String>,
    nick: String,
    target: String,
    options: &'a HashMap<String, String>,
}

impl<'a> BotCommand<'a> {
    pub fn new(
        message: &str,
        nick: Option<String>,
        target: &str,
        options: &'a HashMap<String, String>,
    ) -> Result<Self, &'static str> {
        let split_message: Vec<&str> = message.split_ascii_whitespace().collect();

        Ok(Self {
            name: split_message[0][1..].to_string(),
            args: split_message[1..]
                .into_iter()
                .map(|a| a.to_string())
                .collect(),
            nick: match nick {
                Some(nick) => nick,
                None => return Err("Could not parse nick"),
            },
            target: String::from(target),
            options,
        })
    }

    pub async fn handle(&self, db: Arc<Mutex<Database>>, client: Arc<Mutex<Client>>) -> String {
        match &self.name[..] {
            "alarm" => base::alarm(&self.args, &self.nick, &self.target, db, client).await,
            "ask" => base::ask(&self.args, db).await,
            "city" => city::city(&self.args, db).await,
            "date" | "time" => base::date_time().await,
            "f1results" => f1results::f1results(&self.args).await,
            "first" | "1st" => {
                first::first(&self.nick, &self.target, self.options, db, client).await
            }
            "first_results" => first::first_results(&self.target, db, client).await,
            "first_stats" | "first_points" => first::first_stats(&self.target, db).await,
            "hello" => base::hello(&self.nick).await,
            "imdb" | "omdb" => omdb::omdb(&self.args, self.options).await,
            "next" | "n" => next::next(&self.args, &self.nick, &self.target, db).await,
            "notify" => next::notify(&self.nick, &self.target, db).await,
            "ping" => base::ping().await,
            "quote" => base::quote(&self.args, &self.target, db).await,
            "rates" => rates::rates(&self.args, self.options).await,
            "remind" | "reminder" => {
                base::reminder(&self.args, &self.nick, &self.target, client).await
            }
            "timezone" | "tz" => base::time_zone(&self.args, &self.nick, db).await,
            "weather" | "w" => base::weather(&self.args, &self.nick, self.options, db).await,
            _ => "".to_string(),
        }
    }
}
