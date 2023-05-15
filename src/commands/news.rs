use irc::client::prelude::Command;
use irc::client::Client;
use newsapi::api::NewsAPIClient;
use newsapi::constants::{Country, SortMethod};
use newsapi::payload::article::Articles;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

fn country(name: &str) -> Option<Country> {
    match name.to_lowercase().as_str() {
        "at" | "austria" => Some(Country::Austria),
        "be" | "belgium" => Some(Country::Belgium),
        "fr" | "france" => Some(Country::France),
        "de" | "germany" => Some(Country::Germany),
        "it" | "italy" => Some(Country::Italy),
        "nl" | "netherlands" => Some(Country::Netherlands),
        "pt" | "portugal" => Some(Country::Portugal),
        "se" | "sweden" => Some(Country::Sweden),
        "uk" | "united kingdom" => Some(Country::UnitedKingdomofGreatBritainandNorthernIreland),
        "us" | "usa" | "unitted states" => Some(Country::UnitedStatesofAmerica),
        _ => None,
    }
}

pub async fn news(
    search: &[String],
    target: &str,
    client: Arc<Mutex<Client>>,
    options: &HashMap<String, String>,
) -> String {
    if search.is_empty() {
        return String::from("Please provide some search terms.");
    }

    let mut news = NewsAPIClient::new(match options.get("news_api_key") {
        Some(api_key) => String::from(api_key),
        None => String::from(""),
    });

    let country = match country(&search.join(" ")) {
        Some(country) => country,
        None => return String::from("Country not supported yet."),
    };

    news.country(country)
        .category(newsapi::constants::Category::General)
        .sort_by(SortMethod::PublishedAt)
        .top_headlines();

    let articles = match news.send_async::<Articles>().await {
        Ok(articles) => articles,
        Err(err) => {
            eprintln!("{err}");

            return String::from("Could not fetch news.");
        }
    };

    if articles.articles.is_empty() {
        return String::from("Could not find any news.");
    }

    for article in articles.articles.iter().take(4) {
        if client
            .lock()
            .await
            .send(Command::PRIVMSG(
                String::from(target),
                article.title.clone(),
            ))
            .is_err()
        {
            eprintln!("error");
        }
    }

    String::new()
}
