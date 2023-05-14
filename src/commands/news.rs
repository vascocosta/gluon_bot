use chrono::{Duration, Utc};
use irc::client::prelude::Command;
use irc::client::Client;
use newsapi::api::NewsAPIClient;
use newsapi::constants::{Language, SortMethod};
use newsapi::payload::article::Articles;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

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

    let start = Utc::now() - Duration::hours(72);

    news.from(&start)
        .language(Language::English)
        .query(&search.join(" "))
        .sort_by(SortMethod::Popularity)
        .everything();

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

    for article in articles.articles.iter().take(3) {
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
