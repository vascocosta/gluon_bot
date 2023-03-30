use regex::Regex;
use reqwest::{header::USER_AGENT, Client};
use scraper::{Html, Selector};
use tokio::time::Duration;

pub async fn find_title(url: &str) -> Option<String> {
    let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
        Ok(client) => client,
        Err(_) => return None,
    };
    let res = match client.get(url).header(USER_AGENT, "curl").send().await {
        Ok(res) => res,
        Err(_) => return None,
    };
    let body = match res.text().await {
        Ok(body) => body,
        Err(_) => return None,
    };
    let document = Html::parse_document(&body);
    let selector = match Selector::parse("title") {
        Ok(selector) => selector,
        Err(_) => return None,
    };
    let title: String = match document.select(&selector).next() {
        Some(title) => title.text().collect(),
        None => return None,
    };

    Some(title)
}

pub fn find_url(message: &str) -> Option<&str> {
    match Regex::new(r#"https?://[^\s]+"#) {
        Ok(re) => re.find(message).map(|url| url.as_str()),
        Err(_) => None,
    }
}
