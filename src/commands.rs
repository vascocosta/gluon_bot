mod base;
mod city;
mod f1bet;
mod f1results;
mod f1standings;
mod first;
mod news;
mod next;
mod omdb;
mod plugin;
mod rates;
mod weather;

use crate::database::Database;
use irc::client::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct BotCommand<'a> {
    pub name: String,
    pub args: Vec<String>,
    pub nick: String,
    pub target: String,
    pub timeout: u64,
    pub options: &'a HashMap<String, String>,
}

impl<'a> BotCommand<'a> {
    pub fn new(
        message: &str,
        nick: Option<String>,
        target: &str,
        options: &'a HashMap<String, String>,
    ) -> Result<Self, &'static str> {
        let split_message: Vec<&str> = message.split_ascii_whitespace().collect();
        let name = split_message[0][1..].to_lowercase();
        let args = split_message[1..].iter().map(|a| a.to_string()).collect();
        let nick = match nick {
            Some(nick) => nick,
            None => return Err("Could not parse nick"),
        };
        let target = String::from(target);
        let timeout = match name.as_str() {
            "alarm" | "remind" | "reminder" => 86400,
            _ => 30,
        };

        Ok(Self {
            name,
            args,
            nick,
            target,
            timeout,
            options,
        })
    }

    pub async fn handle(&self, db: Arc<Mutex<Database>>, client: Arc<Mutex<Client>>) -> String {
        match &self.name[..] {
            "alarm" => base::alarm(&self.args, &self.nick, &self.target, db, client).await,
            "ask" => base::ask(&self.args, db).await,
            "bet" => f1bet::bet(&self.args, &self.nick, &self.target, self.options, db).await,
            "city" => city::city(&self.args, db).await,
            "date" | "time" => base::date_time().await,
            "f1results" => f1results::f1results().await,
            "f1standings" | "standings" | "wcc" | "wdc" => f1standings::f1standings().await,
            "first" | "1st" => {
                first::first(&self.nick, &self.target, self.options, db, client).await
            }
            "first_results" => first::first_results(&self.target, db, client).await,
            "first_stats" | "first_points" => first::first_stats(&self.target, db).await,
            "hello" => base::hello(&self.nick).await,
            "help" | "h" | "commands" => base::help().await,
            "imdb" | "omdb" => omdb::omdb(&self.args, self.options).await,
            "news" => news::news(&self.args, &self.target, client, self.options).await,
            "next" | "n" => next::next(&self.args, &self.nick, &self.target, db).await,
            "notify" => next::notify(&self.nick, &self.target, db).await,
            "ping" => base::ping().await,
            "points" | "wbc" => f1bet::points(false, self.options, db).await,
            "quote" => base::quote(&self.args, &self.target, db).await,
            "rates" => rates::rates(&self.args, self.options).await,
            "remind" | "reminder" => {
                base::reminder(&self.args, &self.nick, &self.target, client).await
            }
            "timezone" | "tz" => base::time_zone(&self.args, &self.nick, db).await,
            "weather" | "w" => weather::weather(&self.args, &self.nick, self.options, db).await,
            _ => plugin::plugin(&self.name, &self.args, &self.nick, self.options).await,
        }
    }
}
