use crate::database::{CsvRecord, Database};
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

pub struct BotState {
    pub db: Arc<Mutex<Database>>,
}

#[get("/f1bets/<race>")]
pub async fn f1bets(race: &str, state: &rocket::State<BotState>) -> Json<Vec<Bet>> {
    let bets = state
        .db
        .lock()
        .await
        .select("bets", |b: &Bet| {
            b.race.to_lowercase().contains(race.to_lowercase().as_str())
        })
        .unwrap_or_default()
        .unwrap_or_default();

    Json(bets)
}
