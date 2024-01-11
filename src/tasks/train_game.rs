use crate::database::{CsvRecord, Database};
use chrono::Timelike;
use chrono::Utc;
use irc::client::prelude::Command;
use irc::client::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct TrainSchedule {
    number: usize,
    hour: u32,
    minute: u32,
    delta: u64,
    capacity: usize,
    route: Vec<String>,
}

impl CsvRecord for TrainSchedule {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            number: fields[0].parse().unwrap_or(0),
            hour: fields[1].parse().unwrap_or(25),
            minute: fields[2].parse().unwrap_or(61),
            delta: fields[3].parse().unwrap_or(60),
            capacity: fields[4].parse().unwrap_or(4),
            route: fields[5].split(':').map(String::from).collect(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![
            self.number.to_string(),
            self.hour.to_string(),
            self.minute.to_string(),
            self.delta.to_string(),
            self.capacity.to_string(),
            self.route.join(":"),
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

    async fn run(&mut self) {
        // This clone is needed to avoid having an immutable reference to self at the same time as
        // a mutable reference to self (calling the board method).
        let route = self.schedule.route.clone();

        for station in &route {
            time::sleep(Duration::from_secs(self.schedule.delta * 60)).await;

            if let Err(error) = self.client.lock().await.send(Command::PRIVMSG(
                station.to_owned(),
                format!(
                    "--> 🚉 Train {} has arrived at the {} station. Leaving in 1 minute...",
                    self.schedule.number, station
                ),
            )) {
                eprintln!("{error}");
            }

            time::sleep(Duration::from_secs(60)).await;
            self.board(self.schedule.number, station).await;

            if let Err(error) = self.client.lock().await.send(Command::PRIVMSG(
                station.to_owned(),
                format!(
                    "<-- 🚉 Train {} has departed the {} station. Onboard: {:?}",
                    self.schedule.number, station, self.passengers
                ),
            )) {
                eprintln!("{error}");
            }
        }

        let station = match self.schedule.route.last() {
            Some(station) => station.to_owned(),
            None => String::from("#aviation"),
        };

        if let Err(error) = self.client.lock().await.send(Command::PRIVMSG(
            station,
            format!(
                "--- 🛑 Train {} has ended. Start time: {:0>2}:{:0>2}. Route: {:?}",
                self.schedule.number, self.schedule.hour, self.schedule.minute, self.schedule.route
            ),
        )) {
            eprintln!("{error}");
        }

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
