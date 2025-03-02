use std::{collections::HashMap, process::Command};
use tokio::task;

pub async fn plugin(
    name: &str,
    args: &[String],
    nick: &str,
    target: &str,
    options: &HashMap<String, String>,
) -> String {
    let path = match options.get("plugins_path") {
        Some(path) => path.to_owned(),
        None => "plugins".to_owned(),
    };
    let name = name.to_owned();
    let mut args: Vec<String> = args.to_vec();

    args.insert(0, String::from(nick));
    args.insert(1, String::from(target));

    match task::spawn_blocking(move || {
        match Command::new(format!("{}/{}", path, name))
            .args(args)
            .output()
        {
            Ok(output) => String::from_utf8_lossy(&output.stdout).replace('\n', "\r\n"),
            Err(_) => String::new(),
        }
    })
    .await
    {
        Ok(output) => output,
        Err(_) => String::new(),
    }
}
