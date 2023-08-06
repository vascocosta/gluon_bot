use crate::database::{CsvRecord, Database};
use chrono::{DateTime, Utc};
use rocket::serde::{json::Json, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Bet {
    race: String,
    nick: String,
    p1: String,
    p2: String,
    p3: String,
    fl: String,
}

impl CsvRecord for Bet {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            race: fields[0].clone(),
            nick: fields[1].clone(),
            p1: fields[2].clone(),
            p2: fields[3].clone(),
            p3: fields[4].clone(),
            fl: fields[5].clone(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![
            self.race.clone(),
            self.nick.clone(),
            self.p1.clone(),
            self.p2.clone(),
            self.p3.clone(),
            self.fl.clone(),
        ]
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Event {
    category: String,
    name: String,
    description: String,
    datetime: DateTime<Utc>,
    channel: String,
    tags: String,
    notify: bool,
}

impl CsvRecord for Event {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            category: fields[0].clone(),
            name: fields[1].clone(),
            description: fields[2].clone(),
            datetime: match fields[3].parse() {
                Ok(datetime) => datetime,
                Err(_) => Utc::now(),
            },
            channel: fields[4].clone(),
            tags: fields[5].clone(),
            notify: false,
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![
            self.category.clone(),
            self.name.clone(),
            self.description.clone(),
            self.datetime.to_string(),
            self.channel.clone(),
            self.tags.clone(),
            self.notify.to_string(),
        ]
    }
}

pub struct BotState {
    pub db: Arc<Mutex<Database>>,
}

fn lookup_race(race: &str) -> String {
    let result = match race.to_lowercase().as_str() {
        "bahrain" | "sakhir" => "bahrain",
        "saudi arabia" => "saudi arabian",
        "australia" | "melbourne" => "australian",
        "azerbaijan" | "baku" => "azerbaijan",
        "miami" => "miami",
        "imola" | "san marino" => "emilia-romagna",
        "monaco" => "monaco",
        "spain" | "barcelona" => "spanish",
        "canada" => "canadian",
        "austria" | "spielberg" | "red bull ring" => "austrian",
        "great britain" | "uk" | "silverstone" => "british",
        "hungary" => "hungarian",
        "belgium" | "spa" => "belgian",
        "netherlands" | "zandvoort" => "dutch",
        "italy" | "monza" => "italian",
        "singapore" => "singapore",
        "japan" | "suzuka" => "japanese",
        "qatar" => "qatar",
        "united states" | "usa" | "austin" | "texas" | "cota" => "united states",
        "mexico" => "mexican",
        "brazil" | "sao paulo" | "interlagos" => "brazilian",
        "las vegas" | "vegas" => "las vegas",
        "abu dhabi" => "dhabi",
        _ => race,
    };

    result.to_lowercase()
}

#[get("/events?<category>&<name>&<description>")]
pub async fn events(
    category: Option<&str>,
    name: Option<&str>,
    description: Option<&str>,
    state: &rocket::State<BotState>,
) -> Json<Vec<Event>> {
    let events = state
        .db
        .lock()
        .await
        .select("events", |e: &Event| {
            e.category
                .to_lowercase()
                .contains(category.unwrap_or_default().to_lowercase().as_str())
                && e.name
                    .to_lowercase()
                    .contains(name.unwrap_or_default().to_lowercase().as_str())
                && e.description
                    .to_lowercase()
                    .contains(description.unwrap_or_default().to_lowercase().as_str())
        })
        .unwrap_or_default()
        .unwrap_or_default();

    Json(events)
}

#[get("/f1bets?<race>&<nick>")]
pub async fn f1bets(
    race: Option<&str>,
    nick: Option<&str>,
    state: &rocket::State<BotState>,
) -> Json<Vec<Bet>> {
    let bets = state
        .db
        .lock()
        .await
        .select("bets", |b: &Bet| {
            b.race.to_lowercase().contains(
                lookup_race(race.unwrap_or_default())
                    .to_lowercase()
                    .as_str(),
            ) && b
                .nick
                .to_lowercase()
                .contains(nick.unwrap_or_default().to_lowercase().as_str())
        })
        .unwrap_or_default()
        .unwrap_or_default();

    Json(bets)
}
