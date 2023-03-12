use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const OMDB_API_URL: &str = "https://www.omdbapi.com/";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmDb {
    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "Year")]
    pub year: String,
    #[serde(rename = "Rated")]
    pub rated: String,
    #[serde(rename = "Released")]
    pub released: String,
    #[serde(rename = "Runtime")]
    pub runtime: String,
    #[serde(rename = "Genre")]
    pub genre: String,
    #[serde(rename = "Director")]
    pub director: String,
    #[serde(rename = "Writer")]
    pub writer: String,
    #[serde(rename = "Actors")]
    pub actors: String,
    #[serde(rename = "Plot")]
    pub plot: String,
    #[serde(rename = "Language")]
    pub language: String,
    #[serde(rename = "Country")]
    pub country: String,
    #[serde(rename = "Awards")]
    pub awards: String,
    #[serde(rename = "Poster")]
    pub poster: String,
    #[serde(rename = "Ratings")]
    pub ratings: Vec<Rating>,
    #[serde(rename = "Metascore")]
    pub metascore: String,
    pub imdb_rating: String,
    pub imdb_votes: String,
    #[serde(rename = "imdbID")]
    pub imdb_id: String,
    #[serde(rename = "Type")]
    pub type_field: String,
    #[serde(rename = "DVD")]
    #[serde(default)]
    pub dvd: String,
    #[serde(default)]
    pub total_seasons: String,
    #[serde(rename = "BoxOffice")]
    #[serde(default)]
    pub box_office: String,
    #[serde(rename = "Production")]
    #[serde(default)]
    pub production: String,
    #[serde(rename = "Website")]
    #[serde(default)]
    pub website: String,
    #[serde(rename = "Response")]
    pub response: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rating {
    #[serde(rename = "Source")]
    pub source: String,
    #[serde(rename = "Value")]
    pub value: String,
}

pub async fn omdb(args: &[String], options: &HashMap<String, String>) -> String {
    if args.len() < 1 {
        return String::from("Please provide a movie or series title.");
    }

    let omdb: OmDb = match reqwest::get(format!(
        "{OMDB_API_URL}/?apikey={}&t={}",
        match options.get("omdb_api_key") {
            Some(value) => value,
            None => "",
        },
        &args.join(" ")
    ))
    .await
    {
        Ok(response) => match response.json().await {
            Ok(omdb) => omdb,
            Err(_) => return String::from("Could not find movie or series."),
        },
        Err(_) => return String::from("Could not fetch data."),
    };

    format!(
        "Title: {} | Year: {} | Genre: {} | Director: {} | IMDB Rating: {}\r\n{}",
        omdb.title, omdb.year, omdb.genre, omdb.director, omdb.imdb_rating, omdb.plot
    )
}
