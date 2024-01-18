use crate::database::{CsvRecord, Database};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
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
            notify: fields[6].parse().unwrap_or_default(),
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

pub struct ScoringSystem {
    base: i32,
    fboost: i32,
    fcorrect: i32,
    pboost: i32,
    pcorrect: i32,
}

impl ScoringSystem {
    pub fn from_options(options: &HashMap<String, String>) -> Self {
        ScoringSystem {
            base: match options.get("f1bet_base") {
                Some(base) => base.parse().unwrap_or(1),
                None => 1,
            },
            fboost: match options.get("f1bet_fboost") {
                Some(fboost) => fboost.parse().unwrap_or(10),
                None => 10,
            },
            fcorrect: match options.get("f1bet_fcorrect") {
                Some(fcorrect) => fcorrect.parse().unwrap_or(5),
                None => 5,
            },
            pboost: match options.get("f1bet_pboost") {
                Some(pboost) => pboost.parse().unwrap_or(5),
                None => 5,
            },
            pcorrect: match options.get("f1bet_pcorrect") {
                Some(pcorrect) => pcorrect.parse().unwrap_or(2),
                None => 2,
            },
        }
    }
}

#[derive(PartialEq, Deserialize, Serialize)]
pub struct Bet {
    pub race: String,
    pub nick: String,
    pub p1: String,
    pub p2: String,
    pub p3: String,
    pub p4: String,
    pub p5: String,
}

impl CsvRecord for Bet {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            race: fields[0].clone(),
            nick: fields[1].clone(),
            p1: fields[2].clone(),
            p2: fields[3].clone(),
            p3: fields[4].clone(),
            p4: fields[5].clone(),
            p5: fields[6].clone(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![
            self.race.clone(),
            self.nick.clone(),
            self.p1.clone(),
            self.p2.clone(),
            self.p3.clone(),
            self.p4.clone(),
            self.p5.clone(),
        ]
    }
}

async fn valid_drivers(drivers: &[String], db: Arc<Mutex<Database>>) -> bool {
    let valid_drivers: Vec<Driver> = match db.lock().await.select("drivers", |d: &Driver| {
        d.code.to_lowercase() == drivers[0].to_lowercase()
            || d.code.to_lowercase() == drivers[1].to_lowercase()
            || d.code.to_lowercase() == drivers[2].to_lowercase()
            || d.code.to_lowercase() == drivers[3].to_lowercase()
            || d.code.to_lowercase() == drivers[4].to_lowercase()
    }) {
        Ok(drivers_result) => match drivers_result {
            Some(drivers) => drivers,
            None => return false,
        },
        Err(_) => return false,
    };

    valid_drivers.len() == 5
}

async fn next_race(target: &str, db: Arc<Mutex<Database>>) -> Option<Event> {
    match db.lock().await.select("events", |e: &Event| {
        e.datetime > Utc::now()
            && e.channel.to_lowercase() == target.to_lowercase()
            && e.category.to_lowercase().contains("formula 1")
            && e.description.eq_ignore_ascii_case("race")
    }) {
        Ok(events_result) => match events_result {
            Some(events) => events
                .into_iter()
                .sorted_by(|a, b| a.datetime.cmp(&b.datetime))
                .next(),
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
    amount: usize,
) -> Option<String> {
    let user_bets: Vec<String> = bets
        .iter()
        .filter(|b| b.nick.to_lowercase() == nick.to_lowercase())
        .map(|b| {
            let bet = [
                b.p1.to_lowercase(),
                b.p2.to_lowercase(),
                b.p3.to_lowercase(),
                b.p4.to_lowercase(),
                b.p5.to_lowercase(),
            ];
            let results: Vec<_> = results
                .iter()
                .filter(|r| r.race.to_lowercase() == b.race.to_lowercase())
                .collect();

            if results.is_empty() {
                return (b.race.clone(), bet, 0);
            }

            let result = [
                results[0].p1.to_lowercase(),
                results[0].p2.to_lowercase(),
                results[0].p3.to_lowercase(),
                results[0].p4.to_lowercase(),
                results[0].p5.to_lowercase(),
            ];
            let zipped: Vec<(String, String)> = bet
                .iter()
                .zip(result.iter())
                .filter(|(b, _)| result.contains(b))
                .map(|(b, r)| (b.to_owned(), r.to_owned()))
                .collect();
            let base_score: i32 = zipped
                .iter()
                .enumerate()
                .map(|(i, (b, r))| {
                    if i <= 2 && b == r {
                        scoring_system.pcorrect
                    } else if i > 2 && b == r {
                        scoring_system.fcorrect
                    } else {
                        scoring_system.base
                    }
                })
                .sum();
            let podium_correct = bet[0..3]
                .iter()
                .zip(result[0..3].iter())
                .all(|(b, r)| b == r);
            let top_five_correct =
                base_score == (3 * scoring_system.pcorrect) + (2 * scoring_system.fcorrect);

            if top_five_correct {
                (b.race.clone(), bet, base_score + scoring_system.fboost)
            } else if podium_correct {
                (b.race.clone(), bet, base_score + scoring_system.pboost)
            } else {
                (b.race.clone(), bet, base_score)
            }
        })
        .rev()
        .take(amount)
        .map(|b| {
            format!(
                "{}: {} {} {} {} {} {}",
                b.0,
                b.1[0].to_uppercase(),
                b.1[1].to_uppercase(),
                b.1[2].to_uppercase(),
                b.1[3].to_uppercase(),
                b.1[4].to_uppercase(),
                b.2
            )
        })
        .collect();

    if user_bets.is_empty() {
        None
    } else {
        Some(user_bets.join("\r\n"))
    }
}

pub fn score_bets(
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
                            b.p4.to_lowercase(),
                            b.p5.to_lowercase(),
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
                            results[0].p4.to_lowercase(),
                            results[0].p5.to_lowercase(),
                        ];
                        let zipped: Vec<(String, String)> = bet
                            .iter()
                            .zip(result.iter())
                            .filter(|(b, _)| result.contains(b))
                            .map(|(b, r)| (b.to_owned(), r.to_owned()))
                            .collect();
                        let base_score: i32 = zipped
                            .iter()
                            .enumerate()
                            .map(|(i, (b, r))| {
                                if i <= 2 && b == r {
                                    scoring_system.pcorrect
                                } else if i > 2 && b == r {
                                    scoring_system.fcorrect
                                } else {
                                    scoring_system.base
                                }
                            })
                            .sum();
                        let podium_correct = bet[0..3]
                            .iter()
                            .zip(result[0..3].iter())
                            .all(|(b, r)| b == r);
                        let top_five_correct = base_score
                            == (3 * scoring_system.pcorrect) + (2 * scoring_system.fcorrect);

                        if top_five_correct {
                            base_score + scoring_system.fboost
                        } else if podium_correct {
                            base_score + scoring_system.pboost
                        } else {
                            base_score
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
    let next_race = match next_race(target, Arc::clone(&db)).await {
        Some(next_race) => next_race,
        None => return String::from("Could not find next race."),
    };

    if args.len() <= 1 {
        let bets: Vec<Bet> = match db.lock().await.select("bets", |b: &Bet| {
            b.nick.to_lowercase() == nick.to_lowercase()
        }) {
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

        if args.is_empty() {
            match bets.last() {
                Some(last_bet) => {
                    if last_bet.race.to_lowercase() != next_race.name.to_lowercase() {
                        return String::from("You haven't placed a bet for the current race yet.");
                    }
                }
                None => return String::from("You haven't placed a bet for the current race yet."),
            }

            match bets_log(nick, bets, results, ScoringSystem::from_options(options), 1) {
                Some(bets_log) => return bets_log,
                None => return String::from("Could not find any bets."),
            }
        }

        let arg = args.get(0).unwrap_or(&String::from("")).to_lowercase();

        match arg.as_str() {
            "log" => match bets_log(nick, bets, results, ScoringSystem::from_options(options), 3) {
                Some(bets_log) => return bets_log,
                None => return String::from("Could not find any bets."),
            },
            "history" => match bets_log(
                nick,
                bets,
                results,
                ScoringSystem::from_options(options),
                10,
            ) {
                Some(bets_log) => return bets_log,
                None => return String::from("Could not find any bets."),
            },
            "last_points" | "lastpoints" => return points(true, options, db).await,
            "points" | "wbc" => return points(false, options, db).await,
            _ => return String::from("Unknown sub command."),
        }
    }

    if args.len() != 5 {
        return String::from("The bet must contain 5 drivers: <1st> <2nd> <3rd> <4th> <5th>.");
    }

    if !valid_drivers(args, Arc::clone(&db)).await {
        return String::from("Invalid drivers.");
    }

    match db.lock().await.update(
        "bets",
        Bet {
            race: next_race.name.clone(),
            nick: nick.to_lowercase(),
            p1: args[0].to_lowercase(),
            p2: args[1].to_lowercase(),
            p3: args[2].to_lowercase(),
            p4: args[3].to_lowercase(),
            p5: args[4].to_lowercase(),
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

pub async fn points(
    last: bool,
    options: &HashMap<String, String>,
    db: Arc<Mutex<Database>>,
) -> String {
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

    let bets_scored = if last {
        let last_result = match results.last() {
            Some(last_result) => last_result,
            None => return String::from("Could not find the last result."),
        };
        score_bets(
            bets.into_iter()
                .filter(|b| b.race.to_lowercase() == last_result.race.to_lowercase())
                .collect::<Vec<Bet>>(),
            results,
            ScoringSystem::from_options(options),
        )
    } else {
        score_bets(bets, results, ScoringSystem::from_options(options))
    };

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
