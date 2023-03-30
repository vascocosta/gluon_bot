use regex::Regex;
use reqwest::{header::USER_AGENT, Client};
use scraper::{Html, Selector};
use std::error::Error;
use tokio::time::Duration;

pub async fn find_title(url: &str) -> Result<Option<String>, Box<dyn Error>> {
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
    let res = client.get(url).header(USER_AGENT, "curl").send().await?;
    let body = res.text().await?;
    let document = Html::parse_document(&body);
    let selector = Selector::parse("title")?;

    match document.select(&selector).next() {
        Some(title) => Ok(Some(title.text().collect())),
        None => return Ok(None),
    }
}

pub fn find_url(message: &str) -> Option<&str> {
    match Regex::new(r#"https?://[^\s]+"#) {
        Ok(re) => re.find(message).map(|url| url.as_str()),
        Err(_) => None,
    }
}
