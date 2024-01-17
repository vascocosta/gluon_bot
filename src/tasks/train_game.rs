use crate::database::{CsvRecord, Database};
use chrono::DateTime;
use chrono::Timelike;
use chrono::Utc;
use irc::client::prelude::Command;
use irc::client::Client;
use itertools::Itertools;
use rand::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time;
use tokio_util::sync::CancellationToken;

const MAX_DELAY: u64 = 10;
const STOP_TIME: u64 = 5;
const DERAIL_PROB: u8 = 15;

#[derive(Clone)]
pub struct TrainSchedule {
    number: usize,
    hour: u32,
    minute: u32,
    delta: u64,
    score: u64,
    route: Vec<String>,
}

impl CsvRecord for TrainSchedule {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            number: fields[0].parse().unwrap_or(0),
            hour: fields[1].parse().unwrap_or(25),
            minute: fields[2].parse().unwrap_or(61),
            delta: fields[3].parse().unwrap_or(60),
            score: fields[4].parse().unwrap_or(10),
            route: fields[5].split(':').map(String::from).collect(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![
            self.number.to_string(),
            self.hour.to_string(),
            self.minute.to_string(),
            self.delta.to_string(),
            self.score.to_string(),
            self.route.join(":"),
        ]
    }
}

pub struct Arrival {
    datetime: DateTime<Utc>,
    nick: String,
    number: usize,
}

impl CsvRecord for Arrival {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            datetime: fields[0].parse().unwrap_or_default(),
            nick: fields[1].to_lowercase(),
            number: fields[2].parse().unwrap_or_default(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![
            self.datetime.to_string(),
            self.nick.clone(),
            self.number.to_string(),
        ]
    }
}

#[derive(PartialEq)]
pub struct Boarding {
    nick: String,
    number: usize,
    station: String,
}

impl CsvRecord for Boarding {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            nick: fields[0].clone(),
            number: fields[1].parse().unwrap_or(0),
            station: fields[2].clone(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![
            self.nick.clone(),
            self.number.to_string(),
            self.station.clone(),
        ]
    }
}

pub struct TrainGame {
    token: CancellationToken,
    services: Vec<TrainService>,
}

impl TrainGame {
    pub async fn new(
        client: Arc<Mutex<Client>>,
        db: Arc<Mutex<Database>>,
        token: CancellationToken,
    ) -> Self {
        if let Err(error) = db
            .lock()
            .await
            .delete("train_boardings", |_: &&Boarding| true)
        {
            eprintln!("{error}");
        }

        let schedules = db
            .lock()
            .await
            .select("train_schedules", |_: &TrainSchedule| true)
            .unwrap_or_default()
            .unwrap_or_default();

        Self {
            token,
            services: schedules
                .into_iter()
                .map(|s| TrainService::new(client.clone(), db.clone(), s, &[]))
                .collect(),
        }
    }

    pub async fn run(&self) {
        while !self.token.is_cancelled() {
            let now = Utc::now();

            for service in &self.services {
                let mut service = service.clone();

                if now.hour() == service.schedule.hour && now.minute() == service.schedule.minute {
                    task::spawn(async move {
                        service.run().await;
                    });
                }
            }

            time::sleep(Duration::from_secs(60)).await;
        }
    }
}

#[derive(Clone)]
struct TrainService {
    client: Arc<Mutex<Client>>,
    db: Arc<Mutex<Database>>,
    schedule: TrainSchedule,
    passengers: Vec<String>,
}

impl TrainService {
    fn new(
        client: Arc<Mutex<Client>>,
        db: Arc<Mutex<Database>>,
        schedule: TrainSchedule,
        passengers: &[String],
    ) -> Self {
        Self {
            client,
            db,
            schedule,
            passengers: passengers.to_vec(),
        }
    }

    async fn arrive(&self) {
        let arrivals: Vec<Arrival> = self
            .passengers
            .iter()
            .map(|p| Arrival {
                datetime: Utc::now(),
                nick: p.to_lowercase(),
                number: self.schedule.number,
            })
            .collect();

        for arrival in arrivals {
            if let Err(error) = self.db.lock().await.insert("train_arrivals", arrival) {
                eprintln!("{error}");
            }
        }
    }

    async fn board(&mut self, number: usize, station: &str) {
        let boardings: Vec<Boarding> = self
            .db
            .lock()
            .await
            .select("train_boardings", |b: &Boarding| {
                b.number == number && b.station.to_lowercase() == station
            })
            .unwrap_or_default()
            .unwrap_or_default();

        for boarding in boardings {
            if !self.passengers.contains(&boarding.nick.to_lowercase()) {
                self.passengers.push(boarding.nick.to_lowercase());
            }
        }
    }

    async fn deboard(&self) {
        if let Err(error) = self
            .db
            .lock()
            .await
            .delete("train_boardings", |b: &&Boarding| {
                b.number == self.schedule.number
            })
        {
            eprintln!("{error}");
        }
    }

    fn passengers(&self) -> String {
        self.passengers
            .iter()
            .map(|p| {
                p.to_uppercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .take(3)
                    .collect()
            })
            .collect::<Vec<String>>()
            .join(", ")
    }

    async fn run(&mut self) {
        // This clone is needed to avoid having an immutable reference to self at the same time as
        // a mutable reference to self (calling the board method).
        let route = self.schedule.route.clone();

        for (index, station) in route.iter().enumerate() {
            let mut rng = StdRng::from_entropy();
            let delay = rng.gen_range(0..=MAX_DELAY);

            time::sleep(Duration::from_secs((self.schedule.delta + delay) * 60)).await;

            if rng.gen_range(1..=100) <= DERAIL_PROB {
                if let Err(error) = self.client.lock().await.send(Command::PRIVMSG(
                    station.to_owned(),
                    format!(
                        "!!! ⚠️ {} has derailed before reaching {}! Survivors: {}",
                        self.schedule.number,
                        station,
                        self.passengers()
                    ),
                )) {
                    eprintln!("{error}");
                }

                self.deboard().await;

                return;
            }

            if let Err(error) = self.client.lock().await.send(Command::PRIVMSG(
                station.to_owned(),
                format!(
                    "--> 🚉 {} has arrived at {} ({} min delayed). Points: {}. To board: !board {}",
                    self.schedule.number, station, delay, self.schedule.score, self.schedule.number
                ),
            )) {
                eprintln!("{error}");
            }

            time::sleep(Duration::from_secs(STOP_TIME * 60)).await;
            self.board(self.schedule.number, station).await;

            if index != route.len() - 1 {
                if let Err(error) = self.client.lock().await.send(Command::PRIVMSG(
                    station.to_owned(),
                    format!(
                        "<-- 🚉 {} has departed {}. Passengers: {}",
                        self.schedule.number,
                        station,
                        self.passengers()
                    ),
                )) {
                    eprintln!("{error}");
                }
            } else if let Err(error) = self.client.lock().await.send(Command::PRIVMSG(
                station.to_owned(),
                format!(
                    "--- 🛑 Train {} has ended. Passengers: {}. Route: {:?}",
                    self.schedule.number,
                    self.passengers(),
                    self.schedule.route
                ),
            )) {
                eprintln!("{error}");
            }
        }

        self.arrive().await;
        self.deboard().await;
    }
}

pub async fn board(nick: &str, station: &str, args: &[String], db: Arc<Mutex<Database>>) -> String {
    if db
        .lock()
        .await
        .select("train_boardings", |b: &Boarding| {
            b.nick.to_lowercase() == nick.to_lowercase()
        })
        .is_ok_and(|f| f.is_some())
    {
        return String::from("You've already boarded or scheduled a boarding.");
    }

    let number = args
        .first()
        .unwrap_or(&String::from("7001"))
        .parse()
        .unwrap_or(7001);
    let boarding = Boarding {
        nick: nick.to_lowercase(),
        number,
        station: station.to_lowercase(),
    };

    db.lock()
        .await
        .update("train_boardings", boarding, |b: &&Boarding| {
            b.nick.to_lowercase() == nick.to_lowercase()
        })
        .unwrap_or_default();

    format!("You've scheduled a boarding to train {}.", number)
}

pub async fn schedules(db: Arc<Mutex<Database>>) -> String {
    let schedules = db
        .lock()
        .await
        .select("train_schedules", |_: &TrainSchedule| true)
        .unwrap_or_default()
        .unwrap_or_default();

    schedules
        .into_iter()
        .map(|s| {
            format!(
                "{}: {:0>2}:{:0>2} (UTC) {}",
                s.number,
                s.hour,
                s.minute,
                s.route.first().unwrap_or(&String::from("NA"))
            )
        })
        .collect::<Vec<String>>()
        .join(" | ")
}

pub async fn scores(db: Arc<Mutex<Database>>) -> HashMap<usize, u64> {
    let mut scores: HashMap<usize, u64> = HashMap::new();
    match db
        .lock()
        .await
        .select("train_schedules", |_: &TrainSchedule| true)
    {
        Ok(Some(schedules)) => {
            for schedule in schedules {
                scores.insert(schedule.number, schedule.score);
            }

            scores
        }
        _ => scores,
    }
}

pub async fn points(db: Arc<Mutex<Database>>) -> String {
    let scores = scores(db.clone()).await;
    let arrivals = match db.lock().await.select("train_arrivals", |_: &Arrival| true) {
        Ok(Some(arrivals)) => arrivals,
        Ok(None) => return String::from("There are no arrivals."),
        Err(_) => return String::from("Could not read arrivals."),
    };
    let grouped_arrivals: Vec<(String, Vec<Arrival>)> = arrivals
        .into_iter()
        .sorted_by_key(|a: &Arrival| a.nick.to_lowercase())
        .group_by(|a: &Arrival| a.nick.to_lowercase())
        .into_iter()
        .map(|(key, group)| (key, group.collect()))
        .collect();
    let scored_arrivals: Vec<(String, u64)> = grouped_arrivals
        .into_iter()
        .map(|e| {
            (
                e.0,
                e.1.into_iter()
                    .fold(0, |total, a| total + scores.get(&a.number).unwrap_or(&0)),
            )
        })
        .sorted_by(|a, b| b.1.cmp(&a.1))
        .collect();

    scored_arrivals
        .iter()
        .enumerate()
        .map(|(pos, (nick, points))| {
            format!(
                "{}. {} {}",
                pos + 1,
                nick.to_uppercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .take(3)
                    .collect::<String>(),
                points
            )
        })
        .collect::<Vec<String>>()
        .join(" | ")
}
