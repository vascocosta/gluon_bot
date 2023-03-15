use irc::client::prelude::Command;
use irc::client::Client;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::sync::Mutex;
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
                thread::sleep(Duration::from_secs(1));

                eprintln!("{error}");

                return;
            }
        };

        let mut reader = BufReader::new(file);

        loop {
            let mut line = String::new();

            match reader.read_line(&mut line).await {
                Ok(0) => {
                    thread::sleep(Duration::from_secs(1));

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
                    thread::sleep(Duration::from_secs(1));

                    eprintln!("Error reading line: {}", err);

                    continue;
                }
            }
        }
    }
}
