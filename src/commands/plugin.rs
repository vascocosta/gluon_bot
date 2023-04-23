use std::{collections::HashMap, process::Command};

pub async fn plugin(
    name: &str,
    args: &[String],
    nick: &str,
    options: &HashMap<String, String>,
) -> String {
    let path = match options.get("plugins_path") {
        Some(path) => path,
        None => "plugins",
    };
    let mut args: Vec<String> = args.to_vec();
    args.insert(0, String::from(nick));

    match Command::new(format!("{path}/{name}")).args(args).output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).replace('\n', "\r\n"),
        Err(_) => String::new(),
    }
}
