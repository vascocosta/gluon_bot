use std::collections::HashMap;

use futures::join;

const ERGAST_API_URL: &str = "http://ergast.com/api/f1/current";

mod wcc_models {
    use serde::{Deserialize, Serialize};

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Wcc {
        #[serde(rename = "MRData")]
        pub mrdata: Mrdata,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Mrdata {
        pub xmlns: String,
        pub series: String,
        pub url: String,
        pub limit: String,
        pub offset: String,
        pub total: String,
        #[serde(rename = "StandingsTable")]
        pub standings_table: StandingsTable,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StandingsTable {
        pub season: String,
        #[serde(rename = "StandingsLists")]
        pub standings_lists: Vec<StandingsList>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StandingsList {
        pub season: String,
        pub round: String,
        #[serde(rename = "ConstructorStandings")]
        pub constructor_standings: Vec<ConstructorStanding>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ConstructorStanding {
        pub position: String,
        pub position_text: String,
        pub points: String,
        pub wins: String,
        #[serde(rename = "Constructor")]
        pub constructor: Constructor,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Constructor {
        pub constructor_id: String,
        pub url: String,
        pub name: String,
        pub nationality: String,
    }
}

mod wdc_models {
    use serde::{Deserialize, Serialize};

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Wdc {
        #[serde(rename = "MRData")]
        pub mrdata: Mrdata,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Mrdata {
        pub xmlns: String,
        pub series: String,
        pub url: String,
        pub limit: String,
        pub offset: String,
        pub total: String,
        #[serde(rename = "StandingsTable")]
        pub standings_table: StandingsTable,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StandingsTable {
        pub season: String,
        #[serde(rename = "StandingsLists")]
        pub standings_lists: Vec<StandingsList>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StandingsList {
        pub season: String,
        pub round: String,
        #[serde(rename = "DriverStandings")]
        pub driver_standings: Vec<DriverStanding>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DriverStanding {
        pub position: String,
        pub position_text: String,
        pub points: String,
        pub wins: String,
        #[serde(rename = "Driver")]
        pub driver: Driver,
        #[serde(rename = "Constructors")]
        pub constructors: Vec<Constructor>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Driver {
        pub driver_id: String,
        pub permanent_number: String,
        pub code: String,
        pub url: String,
        pub given_name: String,
        pub family_name: String,
        pub date_of_birth: String,
        pub nationality: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Constructor {
        pub constructor_id: String,
        pub url: String,
        pub name: String,
        pub nationality: String,
    }
}

pub fn team_alias(name: &str) -> String {
    let aliases = HashMap::from([
        ("red bull", "RBR"),
        ("aston martin", "AMR"),
        ("alpine f1 team", "Alpine"),
        ("haas f1 team", "Haas"),
        ("alphatauri", "AT"),
    ]);

    match aliases.get(&*name.to_lowercase()) {
        Some(alias) => String::from(*alias),
        None => String::from(name),
    }
}

pub async fn f1standings() -> String {
    let wcc_task = async {
        let wcc: wcc_models::Wcc =
            match reqwest::get(format!("{}/constructorStandings.json", ERGAST_API_URL)).await {
                Ok(response) => match response.json().await {
                    Ok(wcc) => wcc,
                    Err(_) => return String::from("Could not decode data."),
                },
                Err(_) => return String::from("Could not fetch data."),
            };

        wcc.mrdata.standings_table.standings_lists[0]
            .constructor_standings
            .iter()
            .map(|s| {
                format!(
                    "{}. {} {}",
                    s.position,
                    team_alias(&s.constructor.name),
                    s.points
                )
            })
            .collect::<Vec<String>>()
            .join(" | ")
    };

    let wdc_task = async {
        let wdc: wdc_models::Wdc =
            match reqwest::get(format!("{}/driverStandings.json", ERGAST_API_URL)).await {
                Ok(response) => match response.json().await {
                    Ok(wcc) => wcc,
                    Err(_) => return String::from("Could not decode data."),
                },
                Err(_) => return String::from("Could not fetch data."),
            };

        wdc.mrdata.standings_table.standings_lists[0]
            .driver_standings
            .iter()
            .map(|s| {
                format!(
                    "{}. {} ({} wins) {}",
                    s.position, s.driver.code, s.wins, s.points
                )
            })
            .collect::<Vec<String>>()
            .join(" | ")
    };

    let (wcc, wdc) = join!(wcc_task, wdc_task);

    format!("WCC: {} \r\nWDC: {}", wcc, wdc)
}
