mod base;
mod city;
mod f1results;
mod next;
mod omdb;
mod rates;

use crate::database::Database;
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

    pub async fn handle(&self, db: Arc<Mutex<Database>>) -> String {
        match &self.name[..] {
            "ask" => base::ask(&self.args, db).await,
            "city" => city::city(&self.args, db).await,
            "date" | "time" => base::date_time().await,
            "f1results" => f1results::f1results(&self.args).await,
            "hello" => base::hello(&self.nick).await,
            "imdb" | "omdb" => omdb::omdb(&self.args, self.options).await,
            "next" => next::next(&self.args, &self.nick, &self.target, db).await,
            "ping" => base::ping().await,
            "quote" => base::quote(&self.args, &self.target, db).await,
            "rates" => rates::rates(&self.args, self.options).await,
            "remind" | "reminder" => base::reminder(&self.args, &self.nick).await,
            "weather" => base::weather(&self.args, &self.nick, self.options, db).await,
            _ => "Command not found".to_string(),
        }
    }
}
