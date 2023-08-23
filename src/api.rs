use crate::commands::base::Quote;
use crate::commands::f1bet::Bet;
use crate::commands::next::Event;
use crate::database::Database;
use irc::client::prelude::Command;
use irc::client::Client;
use itertools::Itertools;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

const MIN_KEY_SIZE: usize = 32;

async fn validate_api_key(key: &str) -> bool {
    let keys = match tokio::fs::read_to_string("api_keys.txt").await {
        Ok(keys) => keys,
        Err(_) => return false,
    };

    keys.lines().any(|line| {
        line.len() >= MIN_KEY_SIZE && !line.is_empty() && !line.contains(' ') && line == key
    })
}

#[derive(Debug)]
pub enum ApiKeyError {
    Invalid,
    Missing,
}

pub struct ApiKey(String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKey {
    type Error = ApiKeyError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.headers().get_one("x-api-key") {
            Some(key) if validate_api_key(key).await => Outcome::Success(ApiKey(key.to_string())),
            Some(_) => Outcome::Failure((Status::Unauthorized, ApiKeyError::Invalid)),
            None => Outcome::Failure((Status::Unauthorized, ApiKeyError::Missing)),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    channel: String,
    body: String,
}

pub struct BotState {
    pub client: Arc<Mutex<Client>>,
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

#[allow(clippy::too_many_arguments)]
#[get("/events?<category>&<name>&<description>&<datetime>&<channel>&<tags>&<orderby>&<descending>")]
pub async fn events(
    category: Option<&str>,
    name: Option<&str>,
    description: Option<&str>,
    datetime: Option<&str>,
    channel: Option<&str>,
    tags: Option<&str>,
    orderby: Option<&str>,
    descending: Option<bool>,
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
                && e.datetime
                    .to_string()
                    .to_lowercase()
                    .contains(datetime.unwrap_or_default().to_lowercase().as_str())
                && e.channel
                    .to_lowercase()
                    .contains(channel.unwrap_or_default().to_lowercase().as_str())
                && e.tags
                    .to_lowercase()
                    .contains(tags.unwrap_or_default().to_lowercase().as_str())
        })
        .unwrap_or_default()
        .unwrap_or_default();

    let ordering = match orderby.unwrap_or_default().to_lowercase().as_str() {
        "category" => match descending.unwrap_or_default() {
            false => |a: &Event, b: &Event| Ord::cmp(&a.category, &b.category),
            true => |a: &Event, b: &Event| Ord::cmp(&b.category, &a.category),
        },
        "name" => match descending.unwrap_or_default() {
            false => |a: &Event, b: &Event| Ord::cmp(&a.name, &b.name),
            true => |a: &Event, b: &Event| Ord::cmp(&b.name, &a.name),
        },
        "description" => match descending.unwrap_or_default() {
            false => |a: &Event, b: &Event| Ord::cmp(&a.description, &b.description),
            true => |a: &Event, b: &Event| Ord::cmp(&b.description, &a.description),
        },
        _ => match descending.unwrap_or_default() {
            false => |a: &Event, b: &Event| Ord::cmp(&a.datetime, &b.datetime),
            true => |a: &Event, b: &Event| Ord::cmp(&b.datetime, &a.datetime),
        },
    };

    Json(events.into_iter().sorted_by(ordering).collect())
}

#[post("/events/add", format = "application/json", data = "<event>")]
pub async fn add_event(event: Json<Event>, _key: ApiKey, state: &State<BotState>) -> &'static str {
    let event = Event {
        category: event.category.clone(),
        name: event.name.clone(),
        description: event.description.clone(),
        datetime: event.datetime,
        channel: event.channel.clone(),
        tags: event.tags.clone(),
        notify: event.notify,
    };

    if state.db.lock().await.insert("events", event).is_err() {
        return "Failure";
    }

    "Success"
}

#[post("/events/delete", format = "application/json", data = "<event>")]
pub async fn delete_event(
    event: Json<Event>,
    _key: ApiKey,
    state: &State<BotState>,
) -> &'static str {
    if state
        .db
        .lock()
        .await
        .delete("events", |e: &&Event| {
            e.category.to_lowercase() == event.category.to_lowercase()
                && e.name.to_lowercase() == event.name.to_lowercase()
                && e.description.to_lowercase() == event.description.to_lowercase()
                && e.datetime == event.datetime
        })
        .is_err()
    {
        return "Failure";
    }

    "Success"
}

#[get("/f1bets?<race>&<nick>")]
pub async fn f1_bets(
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

#[get("/quotes?<date>&<text>&<channel>")]
pub async fn quotes(
    date: Option<&str>,
    text: Option<&str>,
    channel: Option<&str>,
    state: &State<BotState>,
) -> Json<Vec<Quote>> {
    let quotes = state
        .db
        .lock()
        .await
        .select("quotes", |q: &Quote| {
            q.date
                .to_lowercase()
                .contains(date.unwrap_or_default().to_lowercase().as_str())
                && q.text
                    .to_lowercase()
                    .contains(text.unwrap_or_default().to_lowercase().as_str())
                && q.channel
                    .to_lowercase()
                    .contains(channel.unwrap_or_default().to_lowercase().as_str())
        })
        .unwrap_or_default()
        .unwrap_or_default();

    Json(quotes)
}

#[post("/quotes/add", format = "application/json", data = "<quote>")]
pub async fn add_quote(quote: Json<Quote>, _key: ApiKey, state: &State<BotState>) -> &'static str {
    let quote = Quote {
        date: quote.date.clone(),
        text: quote.text.clone(),
        channel: quote.channel.clone(),
    };

    if state.db.lock().await.insert("quotes", quote).is_err() {
        return "Failure";
    }

    "Success"
}

#[post("/quotes/delete", format = "application/json", data = "<quote>")]
pub async fn delete_quote(
    quote: Json<Quote>,
    _key: ApiKey,
    state: &State<BotState>,
) -> &'static str {
    if state
        .db
        .lock()
        .await
        .delete("quotes", |q: &&Quote| {
            q.date.to_lowercase() == quote.date.to_lowercase()
                && q.text.to_lowercase() == quote.text.to_lowercase()
                && q.channel.to_lowercase() == quote.channel.to_lowercase()
        })
        .is_err()
    {
        return "Failure";
    }

    "Success"
}

#[post("/say", format = "application/json", data = "<message>")]
pub async fn say(message: Json<Message>, _key: ApiKey, state: &State<BotState>) -> &'static str {
    if state
        .client
        .lock()
        .await
        .send(Command::PRIVMSG(
            message.channel.clone(),
            message.body.clone(),
        ))
        .is_err()
    {
        return "Failure";
    }

    "Success"
}
