use std::thread;
use std::time::Duration;
use std::{fs::OpenOptions, io::BufRead, io::BufReader};

pub async fn external_message() -> String {
    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .open("out.txt")
    {
        Ok(file) => file,
        Err(error) => {
            thread::sleep(Duration::from_secs(1));
            return format!("{error}");
        },
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
                return line;
            }
            Err(err) => {
                println!("Error reading line: {}", err);
                thread::sleep(Duration::from_secs(1));
                continue;
            }
        }
    }
}
