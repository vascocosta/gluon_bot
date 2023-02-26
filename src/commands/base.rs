use std::collections::HashMap;
use std::time::Duration;
use tokio::time;

use chrono::Utc;

pub async fn date_time() -> String {
    format!("{} UTC", Utc::now().format("%H:%M:%S %d/%m/%Y"))
}

pub async fn hello(nick: &str) -> String {
    format!("Hello {}", nick)
}

pub async fn ping() -> String {
    format!("pong")
}

pub async fn reminder(args: &[String], nick: &str) -> String {
    if args.len() < 1 {
        return format!("Please provide a duration in minutes.");
    }

    let minutes: u64 = match args[0].parse() {
        Ok(minutes) => minutes,
        Err(_) => return format!("Please provide a duration in integer minutes."),
    };

    time::sleep(Duration::from_secs(minutes * 60)).await;

    format!("{nick}: Time is up!")
}

pub async fn weather(args: &[String], options: &HashMap<String, String>) -> String {
    if args.len() < 1 {
        return format!("Please provide a location.");
    }

    match openweathermap::weather(
        &args.join(" "),
        match options.get("owm_api_units") {
            Some(value) => value,
            None => "metric",
        },
        match options.get("owm_api_language") {
            Some(value) => value,
            None => "en",
        },
        match options.get("owm_api_key") {
            Some(value) => value,
            None => "",
        },
    )
    .await
    {
        Ok(current) => format!(
            "{}: {} {:.1}C Humidity: {}% Pressure: {}hPa Wind: {:.1}m/s @ {} Gusts: {:.1}m/s",
            current.name,
            current.weather[0].description,
            current.main.temp,
            current.main.humidity,
            current.main.pressure,
            current.wind.speed,
            current.wind.deg,
            current.wind.gust.unwrap_or_default(),
        ),
        Err(_) => format!("Could not fetch weather."),
    }
}
