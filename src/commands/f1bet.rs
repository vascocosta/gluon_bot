use crate::database::{CsvRecord, Database};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

struct Driver {
    number: u32,
    code: String,
}

impl CsvRecord for Driver {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            number: fields[0].parse().unwrap_or(0),
            code: fields[1].clone(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![self.number.to_string(), self.code.clone()]
    }
}

struct Event {
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

struct ScoringSystem {
    boost: i32,
    correct: i32,
    fl: i32,
    podium: i32,
}

impl ScoringSystem {
    fn from_options(options: &HashMap<String, String>) -> Self {
        ScoringSystem {
            boost: match options.get("f1bet_boost") {
                Some(boost) => boost.parse().unwrap_or(10),
                None => 10,
            },
            correct: match options.get("f1bet_correct") {
                Some(correct) => correct.parse().unwrap_or(5),
                None => 5,
            },
            fl: match options.get("f1bet_fl") {
                Some(fl) => fl.parse().unwrap_or(1),
                None => 1,
            },
            podium: match options.get("f1bet_podium") {
                Some(podium) => podium.parse().unwrap_or(3),
                None => 3,
            },
        }
    }
}

#[derive(PartialEq)]
struct Bet {
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

async fn valid_drivers(drivers: &[String], db: Arc<Mutex<Database>>) -> bool {
    let podium_drivers: Vec<Driver> = match db.lock().await.select("drivers", |d: &Driver| {
        d.code.to_lowercase() == drivers[0].to_lowercase()
            || d.code.to_lowercase() == drivers[1].to_lowercase()
            || d.code.to_lowercase() == drivers[2].to_lowercase()
    }) {
        Ok(drivers_result) => match drivers_result {
            Some(drivers) => drivers,
            None => return false,
        },
        Err(_) => return false,
    };
    let fl_driver: Vec<Driver> = match db.lock().await.select("drivers", |d: &Driver| {
        d.code.to_lowercase() == drivers[3].to_lowercase()
    }) {
        Ok(drivers_result) => match drivers_result {
            Some(drivers) => drivers,
            None => return false,
        },
        Err(_) => return false,
    };

    podium_drivers.len() + fl_driver.len() == 4
}

async fn next_race(target: &str, db: Arc<Mutex<Database>>) -> Option<Event> {
    match db.lock().await.select("events", |e: &Event| {
        e.datetime > Utc::now()
            && e.channel.to_lowercase() == target.to_lowercase()
            && e.category.to_lowercase().contains("formula 1")
            && e.description.eq_ignore_ascii_case("race")
    }) {
        Ok(events_result) => match events_result {
            Some(events) => events.into_iter().next(),
            None => None,
        },
        Err(_) => None,
    }
}

fn bets_log(
    nick: &str,
    bets: Vec<Bet>,
    results: Vec<Bet>,
    scoring_system: ScoringSystem,
) -> Option<String> {
    let user_bets: Vec<String> = bets
        .iter()
        .filter(|b| b.nick.to_lowercase() == nick.to_lowercase())
        .map(|b| {
            let bet = [
                b.p1.to_lowercase(),
                b.p2.to_lowercase(),
                b.p3.to_lowercase(),
            ];
            let results: Vec<_> = results
                .iter()
                .filter(|r| r.race.to_lowercase() == b.race.to_lowercase())
                .collect();

            if results.is_empty() {
                return (b.race.clone(), bet, b.fl.clone(), 0);
            }

            let result = [
                results[0].p1.to_lowercase(),
                results[0].p2.to_lowercase(),
                results[0].p3.to_lowercase(),
            ];
            let zipped: Vec<(String, String)> = bet
                .iter()
                .zip(result.iter())
                .filter(|(b, _)| result.contains(b))
                .map(|(b, r)| (b.to_owned(), r.to_owned()))
                .collect();
            let podium_score: i32 = zipped
                .iter()
                .map(|(b, r)| {
                    if b == r {
                        scoring_system.correct
                    } else {
                        scoring_system.podium
                    }
                })
                .sum();
            let boost_score = if podium_score == (3 * scoring_system.correct) {
                podium_score + scoring_system.boost
            } else {
                podium_score
            };

            if b.fl.to_lowercase() == results[0].fl.to_lowercase() {
                (
                    b.race.clone(),
                    bet,
                    b.fl.clone(),
                    boost_score + scoring_system.fl,
                )
            } else {
                (b.race.clone(), bet, b.fl.clone(), boost_score)
            }
        })
        .rev()
        .take(3)
        .map(|b| {
            format!(
                "{}: {} {} {} {} {}",
                b.0,
                b.1[0].to_uppercase(),
                b.1[1].to_uppercase(),
                b.1[2].to_uppercase(),
                b.2.to_uppercase(),
                b.3
            )
        })
        .collect();

    if user_bets.is_empty() {
        None
    } else {
        Some(user_bets.join(" | "))
    }
}

fn score_bets(
    bets: Vec<Bet>,
    results: Vec<Bet>,
    scoring_system: ScoringSystem,
) -> Vec<(String, i32)> {
    let bets_grouped: Vec<(_, Vec<Bet>)> = bets
        .into_iter()
        .sorted_by_key(|b: &Bet| b.nick.to_lowercase())
        .group_by(|e: &Bet| e.nick.to_lowercase())
        .into_iter()
        .map(|(key, group)| (key, group.collect()))
        .collect();
    let bets_scored: Vec<(String, i32)> = bets_grouped
        .iter()
        .map(|b| {
            (
                b.0.clone(),
                b.1.iter()
                    .map(|b| {
                        let bet = [
                            b.p1.to_lowercase(),
                            b.p2.to_lowercase(),
                            b.p3.to_lowercase(),
                        ];
                        let results: Vec<_> = results
                            .iter()
                            .filter(|r| r.race.to_lowercase() == b.race.to_lowercase())
                            .collect();

                        if results.is_empty() {
                            return 0;
                        }

                        let result = [
                            results[0].p1.to_lowercase(),
                            results[0].p2.to_lowercase(),
                            results[0].p3.to_lowercase(),
                        ];
                        let zipped: Vec<(String, String)> = bet
                            .iter()
                            .zip(result.iter())
                            .filter(|(b, _)| result.contains(b))
                            .map(|(b, r)| (b.to_owned(), r.to_owned()))
                            .collect();
                        let podium_score: i32 = zipped
                            .iter()
                            .map(|(b, r)| {
                                if b == r {
                                    scoring_system.correct
                                } else {
                                    scoring_system.podium
                                }
                            })
                            .sum();
                        let boost_score = if podium_score == (3 * scoring_system.correct) {
                            podium_score + scoring_system.boost
                        } else {
                            podium_score
                        };

                        if b.fl.to_lowercase() == results[0].fl.to_lowercase() {
                            boost_score + scoring_system.fl
                        } else {
                            boost_score
                        }
                    })
                    .sum::<i32>(),
            )
        })
        .sorted_by(|a, b| b.1.cmp(&a.1))
        .collect();

    bets_scored
}

pub async fn bet(
    args: &[String],
    nick: &str,
    target: &str,
    options: &HashMap<String, String>,
    db: Arc<Mutex<Database>>,
) -> String {
    if args.is_empty() || (args.len() == 1 && args[0].to_lowercase() == "log") {
        let bets: Vec<Bet> = match db.lock().await.select("bets", |_| true) {
            Ok(bets_result) => match bets_result {
                Some(bets) => bets,
                None => return String::from("Could not find any bets."),
            },
            Err(_) => return String::from("Could not find any bets."),
        };
        let results: Vec<Bet> = match db.lock().await.select("results", |_| true) {
            Ok(bets_result) => match bets_result {
                Some(bets) => bets,
                None => return String::from("Could not find any results."),
            },
            Err(_) => return String::from("Could not find any results."),
        };

        match bets_log(nick, bets, results, ScoringSystem::from_options(options)) {
            Some(bets_log) => return bets_log,
            None => return String::from("Could not find any bets."),
        }
    }

    if args.len() != 4 {
        return String::from("The bet must contain 4 drivers: <1st> <2nd> <3rd> <fl>.");
    }

    if !valid_drivers(args, Arc::clone(&db)).await {
        return String::from("Invalid drivers.");
    }

    let next_race = match next_race(target, Arc::clone(&db)).await {
        Some(next_race) => next_race,
        None => return String::from("Could not find next race."),
    };

    match db.lock().await.update(
        "bets",
        Bet {
            race: next_race.name.clone(),
            nick: nick.to_lowercase(),
            p1: args[0].to_lowercase(),
            p2: args[1].to_lowercase(),
            p3: args[2].to_lowercase(),
            fl: args[3].to_lowercase(),
        },
        |b: &&Bet| {
            b.race.to_lowercase() == next_race.name.to_lowercase()
                && b.nick.to_lowercase() == nick.to_lowercase()
        },
    ) {
        Ok(()) => format!(
            "Your bet for the {} was successfully updated.",
            next_race.name
        ),
        Err(_) => String::from("Problem updating your bet."),
    }
}

pub async fn points(options: &HashMap<String, String>, db: Arc<Mutex<Database>>) -> String {
    let bets: Vec<Bet> = match db.lock().await.select("bets", |_| true) {
        Ok(bets_result) => match bets_result {
            Some(bets) => bets,
            None => return String::from("Could not find any bets."),
        },
        Err(_) => return String::from("Could not find any bets."),
    };
    let results: Vec<Bet> = match db.lock().await.select("results", |_| true) {
        Ok(bets_result) => match bets_result {
            Some(bets) => bets,
            None => return String::from("Could not find any results."),
        },
        Err(_) => return String::from("Could not find any results."),
    };

    let bets_scored = score_bets(bets, results, ScoringSystem::from_options(options));

    bets_scored
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