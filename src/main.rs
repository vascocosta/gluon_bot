mod commands;
mod database;
mod tasks;
mod utils;

use commands::BotCommand;
use database::Database;
use futures::prelude::*;
use irc::client::prelude::*;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Variables that run exclusively on the main task/thread are declared like regular variables.
    // Variables whose immutable/mutable reference is shared among tasks/threads use an Arc/Mutex.
    let config = Config::load("config.toml")?;
    let client = Arc::new(Mutex::new(Client::from_config(config.clone()).await?));
    let options = Arc::new(config.options);
    let prefix = match options.get("prefix") {
        Some(prefix) => prefix,
        None => "!",
    };
    let db = Arc::new(Mutex::new(Database::new(
        match options.get("database_path") {
            Some(path) => path,
            None => "data/",
        },
        None,
    )));
    let mut stream = client.lock().await.stream()?;

    client.lock().await.identify()?;

    let client_clone = Arc::clone(&client);
    let db_clone = Arc::clone(&db);

    // Spawn various different background tasks that run indefinitely.
    task::spawn(async move {
        tasks::next::next(client_clone, db_clone).await;
    });

    let client_clone = Arc::clone(&client);

    task::spawn(async move {
        tasks::base::external_message(client_clone).await;
    });

    let options_clone = Arc::clone(&options);
    let client_clone = Arc::clone(&client);
    let db_clone = Arc::clone(&db);

    task::spawn(async move { tasks::feeds::feeds(options_clone, client_clone, db_clone).await });

    // Main loop that continously gets IRC messages from an asynchronous stream.
    // Match any PRIVMSG received from the asynchronous stream of messages.
    // If the message is a bot command, spawn a Tokio task to handle the command.
    while let Some(message) = stream.next().await.transpose()? {
        let sender = client.lock().await.sender();
        let nick = match message.prefix {
            Some(prefix) => match prefix {
                Prefix::Nickname(nick, _, _) => Some(nick),
                _ => None,
            },
            None => None,
        };

        match message.command {
            Command::PRIVMSG(target, message) => {
                if message.len() > 1 && message.starts_with(prefix) {
                    let options = Arc::clone(&options);
                    let db = Arc::clone(&db);

                    task::spawn(async move {
                        if let Ok(bot_command) = BotCommand::new(&message, nick, &target, &options)
                        {
                            let output = bot_command.handle(db).await;

                            if let Err(error) = sender.send_privmsg(&target, output) {
                                eprintln!("{error}");
                            }
                        }
                    });
                } else {
                    task::spawn(async move {
                        if let Some(url) = utils::find_url(&message) {
                            if let Some(title) = utils::find_title(url).await {
                                if let Err(error) = sender.send_privmsg(&target, title) {
                                    eprint!("{error}");
                                }
                            }
                        }
                    });
                }
            }
            _ => (),
        }
    }

    Ok(())
}
