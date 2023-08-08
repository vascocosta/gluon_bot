mod api;
mod commands;
mod database;
mod tasks;
mod utils;

use commands::BotCommand;
use database::Database;
use futures::prelude::*;
use irc::client::prelude::*;
use rocket::fs::FileServer;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time;

#[macro_use]
extern crate rocket;

#[tokio::main]
async fn main() {
    // Variables that run exclusively on the main task/thread are declared like regular variables.
    // Variables whose immutable/mutable reference is shared among tasks/threads use an Arc/Mutex.
    let config = match Config::load("config.toml") {
        Ok(config) => config,
        Err(error) => match error {
            irc::error::Error::Io(_) => {
                eprintln!("Could not read configuration file (config.toml).");

                return;
            }
            _ => {
                eprintln!("Unknown error parsing configuration file.");

                return;
            }
        },
    };
    let client = Arc::new(Mutex::new(
        match Client::from_config(config.clone()).await {
            Ok(client) => client,
            Err(error) => match error {
                irc::error::Error::InvalidConfig { path, cause } => {
                    eprintln!("Invalid configuration file ({path}). Cause: {cause}.");

                    return;
                }
                _ => {
                    eprintln!("Unknown error parsing configuration file.");

                    return;
                }
            },
        },
    ));
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

    let mut stream = match client.lock().await.stream() {
        Ok(stream) => stream,
        Err(error) => {
            eprintln!("{error}");

            return;
        }
    };

    if let Err(error) = client.lock().await.identify() {
        eprintln!("{error}");
    }

    let client_clone = Arc::clone(&client);
    let db_clone = Arc::clone(&db);

    // Spawn a background task to handle API requests to the bot.
    task::spawn(async move {
        let my_state = api::BotState {
            client: client_clone,
            db: db_clone,
        };

        let _rocket = rocket::build()
            .mount(
                "/api",
                routes![api::f1bets, api::events, api::say, api::addevent,],
            )
            .mount("/", FileServer::from("static/"))
            .manage(my_state)
            .launch()
            .await
            .unwrap();
    });

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
    while let Ok(Some(message)) = stream.next().await.transpose() {
        let sender = client.lock().await.sender();
        let nick = match message.prefix {
            Some(Prefix::Nickname(nick, _, _)) => Some(nick),
            Some(Prefix::ServerName(_)) => None,
            None => None,
        };

        if let Command::PRIVMSG(target, message) = message.command {
            if message.len() > 1 && message.starts_with(prefix) {
                let options = Arc::clone(&options);
                let db = Arc::clone(&db);
                let client = Arc::clone(&client);

                task::spawn(async move {
                    if let Ok(bot_command) = BotCommand::new(&message, nick, &target, &options) {
                        let output = match time::timeout(
                            Duration::from_secs(bot_command.timeout),
                            bot_command.handle(db, client),
                        )
                        .await
                        {
                            Ok(output) => output,
                            Err(_) => String::from("Timeout while running command."),
                        };

                        if let Err(error) = sender.send_privmsg(&target, output) {
                            eprintln!("{error}");
                        }
                    }
                });
            } else {
                task::spawn(async move {
                    if let Some(url) = utils::find_url(&message) {
                        if let Ok(Some(title)) = utils::find_title(url).await {
                            if let Err(error) = sender.send_privmsg(&target, title) {
                                eprint!("{error}");
                            }
                        }
                    }
                });
            }
        }
    }
}
