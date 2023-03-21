use irc::client::prelude::Command;
use irc::client::Client;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio::{fs::OpenOptions, io::BufReader};

pub async fn external_message(client: Arc<Mutex<Client>>) {
    loop {
        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .open("out.txt")
            .await
        {
            Ok(file) => file,
            Err(error) => {
                sleep(Duration::from_secs(1)).await;

                eprintln!("{error}");

                return;
            }
        };

        let mut reader = BufReader::new(file);

        loop {
            let mut line = String::new();

            match reader.read_line(&mut line).await {
                Ok(0) => {
                    sleep(Duration::from_secs(1)).await;

                    continue;
                }
                Ok(_) => {
                    if line.len() > 2 && line.starts_with("#") {
                        let split_line: Vec<&str> = line.split_ascii_whitespace().collect();

                        if split_line.len() > 1 {
                            if let Err(error) = client.lock().await.send(Command::PRIVMSG(
                                split_line[0].to_string(),
                                split_line[1..].join(" "),
                            )) {
                                eprintln!("{error}");
                            }
                        }
                    }

                    break;
                }
                Err(err) => {
                    sleep(Duration::from_secs(1)).await;

                    eprintln!("Error reading line: {}", err);

                    continue;
                }
            }
        }
    }
}
