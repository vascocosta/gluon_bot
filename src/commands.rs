mod base;
mod omdb;

use std::collections::HashMap;

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
            target: target.to_string(),
            options,
        })
    }

    pub async fn handle(&self) -> String {
        match &self.name[..] {
            "date" | "time" => base::date_time().await,
            "hello" => base::hello(&self.nick).await,
            "imdb" | "omdb" => omdb::omdb(&self.args, self.options).await,
            "ping" => base::ping().await,
            "remind" | "reminder" => base::reminder(&self.args, &self.nick).await,
            "weather" => base::weather(&self.args, self.options).await,
            _ => "Command not found".to_string(),
        }
    }
}
