use irc::client::prelude::Command;
use irc::client::Client;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::{fs::OpenOptions, io::BufRead, io::BufReader};
use tokio::sync::Mutex;

pub async fn external_message(client: Arc<Mutex<Client>>) {
    loop {
        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .open("out.txt")
        {
            Ok(file) => file,
            Err(error) => {
                thread::sleep(Duration::from_secs(1));

                if let Err(error) = client
                    .lock()
                    .await
                    .send(Command::PRIVMSG("#aviation".to_string(), error.to_string()))
                {
                    eprintln!("{error}");
                }

                return;
            }
        };

        let mut reader = BufReader::new(&file);

        loop {
            let mut line = String::new();

            match reader.read_line(&mut line) {
                Ok(0) => {
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
                Ok(_) => {
                    file.set_len(0).unwrap();

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
                    println!("Error reading line: {}", err);
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            }
        }
    }
}
