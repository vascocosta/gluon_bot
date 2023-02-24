mod commands;

use commands::BotCommand;
use futures::prelude::*;
use irc::client::prelude::*;
use std::error::Error;
use std::sync::Arc;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::load("config.toml")?;
    let mut client = Client::from_config(config.clone()).await?;
    let options = Arc::new(config.options);
    let prefix = match options.get("prefix") {
        Some(prefix) => prefix,
        None => "!",
    };

    client.identify()?;

    let mut stream = client.stream()?;

    while let Some(message) = stream.next().await.transpose()? {
        let sender = client.sender();
        let nick = match message.prefix {
            Some(prefix) => match prefix {
                Prefix::Nickname(nick, _, _) => Some(nick),
                _ => None,
            },
            None => None,
        };

        match message.command {
            Command::PRIVMSG(target, message) => {
                //let options = &config.options;
                let options = Arc::clone(&options);

                if message.len() > 1 && message.starts_with(prefix) {
                    task::spawn(async move {
                        if let Ok(bot_command) = BotCommand::new(&message, nick, &target, &options)
                        {
                            let output = bot_command.handle().await;

                            if let Err(error) = sender.send_privmsg(&target, output) {
                                eprintln!("{error}");
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
