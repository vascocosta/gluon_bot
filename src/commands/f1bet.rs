use crate::database::{CsvRecord, Database};
use itertools::Itertools;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

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
