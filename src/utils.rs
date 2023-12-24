use regex::Regex;
use reqwest::{header::USER_AGENT, Client};
use scraper::{Html, Selector};
use std::error::Error;
use tokio::time::Duration;

pub async fn find_title(url: &str) -> Result<Option<String>, Box<dyn Error>> {
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
    let res = client
        .get(url)
        .header(
            USER_AGENT,
            "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/111.0",
        )
        .send()
        .await?;
    let body = res.text().await?;
    let document = Html::parse_document(&body);
    let selector = Selector::parse("title")?;

    match document.select(&selector).next() {
        Some(title) => Ok(Some(title.text().collect())),
        None => Ok(None),
    }
}

pub fn find_url(message: &str) -> Option<&str> {
    match Regex::new(r"https?://[^\s]+") {
        Ok(re) => re.find(message).map(|url| url.as_str()),
        Err(_) => None,
    }
}

pub fn upper_initials(text: &str) -> String {
    text.split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            let first = chars.next().unwrap_or_default().to_uppercase();
            let rest = chars.collect::<String>();

            format!("{first}{rest}")
        })
        .collect::<Vec<String>>()
        .join(" ")
}
