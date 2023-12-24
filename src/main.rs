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
use rocket::fs::NamedFile;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time;
use tokio_util::sync::CancellationToken;

#[macro_use]
extern crate rocket;

#[get("/<_path..>", rank = 2)]
async fn all(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/index.html")).await.ok()
}

#[tokio::main]
async fn main() {
    loop {
        // Configure an IRC cient with settings from a config file.
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

        // Connect the IRC client to the IRC server.
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

        println!("Connected to the IRC server.");

        // Spawn the API task.
        // TODO: Find a way to cancel this task during disconnects.
        let client_clone = Arc::clone(&client);
        let db_clone = Arc::clone(&db);
        let api_token = CancellationToken::new();
        task::spawn(async move {
            let my_state = api::BotState {
                client: client_clone,
                db: db_clone,
            };

            if rocket::build()
                .mount(
                    "/api",
                    routes![
                        api::add_event,
                        api::add_quote,
                        api::delete_event,
                        api::delete_quote,
                        api::events,
                        api::f1_bets,
                        api::quotes,
                        api::say,
                        api::score_f1_bets,
                        api::update_event,
                        api::update_quote,
                    ],
                )
                .mount("/", FileServer::from("static/").rank(1))
                .mount("/", routes![all])
                .manage(my_state)
                .launch()
                .await
                .is_err()
            {
                api_token.cancel();
                eprintln!("Problem with the API task.");
            }
        });

        // Spawn the next task.
        let client_clone = Arc::clone(&client);
        let db_clone = Arc::clone(&db);
        let next_token = CancellationToken::new();
        let next_token_clone = next_token.clone();
        let next_task = task::spawn(async move {
            tasks::next::next(client_clone, db_clone, next_token_clone).await;
        });

        // Spawn the external_message task.
        let client_clone = Arc::clone(&client);
        let external_message_token = CancellationToken::new();
        let external_message_token_clone = external_message_token.clone();
        let external_message_task = task::spawn(async move {
            tasks::base::external_message(client_clone, external_message_token_clone).await;
        });

        // Spawn the feeds task.
        let options_clone = Arc::clone(&options);
        let client_clone = Arc::clone(&client);
        let db_clone = Arc::clone(&db);
        let feeds_token = CancellationToken::new();
        let feeds_token_clone = feeds_token.clone();
        let feeds_task = task::spawn(async move {
            tasks::feeds::feeds(options_clone, client_clone, db_clone, feeds_token_clone).await
        });

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
                        if let Ok(bot_command) = BotCommand::new(&message, nick, &target, &options)
                        {
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

        eprintln!("Diconnected from the IRC server.");

        // Cancel the next task.
        // If the task doesn't finish, terminate the bot.
        next_token.cancel();

        if next_task.await.is_err() {
            eprintln!("Could not cancel next task.");
            eprintln!("Terminating bot...");

            return;
        }

        eprintln!("Next task finished.");

        // Cancel the external_message task.
        // If the task doesn't finish, terminate the bot.
        external_message_token.cancel();

        if external_message_task.await.is_err() {
            eprintln!("Could not cancel external_message task.");
            eprintln!("Terminating bot...");

            return;
        }

        eprintln!("External Message task finished.");

        // Cancel the feeds task.
        // If the task doesn't finish, terminate the bot.
        feeds_token.cancel();

        if feeds_task.await.is_err() {
            eprintln!("Could not cancel feeds task.");
            eprintln!("Terminating bot...");

            return;
        }

        eprintln!("Feeds task finished.");

        // Wait 30 seconds before trying to reconnect.
        // This should avoid an overly fast reconnect.
        println!("Waiting 30 seconds before reconnecting to the IRC server...");
        time::sleep(Duration::from_secs(30)).await;
        println!("Reconnecting to the IRC server...");
    }
}
