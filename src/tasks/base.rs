use irc::client::prelude::Command;
use irc::client::Client;
use std::io::ErrorKind;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio::{
    fs::{File, OpenOptions},
    io::BufReader,
};
use tokio_util::sync::CancellationToken;

pub async fn external_message(client: Arc<Mutex<Client>>, token: CancellationToken) {
    while !token.is_cancelled() {
        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .open("out.txt")
            .await
        {
            Ok(file) => file,
            Err(error) => match error.kind() {
                ErrorKind::NotFound => match File::create("out.txt").await {
                    Ok(_) => continue,
                    Err(_) => {
                        eprintln!("Could not create out.txt.");

                        return;
                    }
                },
                _ => {
                    eprintln!("Could not create out.txt.");

                    return;
                }
            },
        };

        let mut reader = BufReader::new(file);

        while !token.is_cancelled() {
            let mut line = String::new();

            match reader.read_line(&mut line).await {
                Ok(0) => {
                    sleep(Duration::from_secs(1)).await;

                    continue;
                }
                Ok(_) => {
                    if line.len() > 2 && line.starts_with('#') {
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
