use regex::Regex;
use reqwest::{header::USER_AGENT, Client};
use scraper::{Html, Selector};
use serde::Deserialize;
use std::error::Error;
use tokio::time::Duration;

const TIMEOUT: u64 = 10;
const USER_AGENT_STRING: &str =
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/111.0";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoSnippet {
    title: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoContentDetails {
    duration: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoStatistics {
    view_count: Option<String>,
    like_count: Option<String>,
    comment_count: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Video {
    snippet: VideoSnippet,
    content_details: VideoContentDetails,
    statistics: VideoStatistics,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiResponse {
    items: Vec<Video>,
}

pub async fn find_title(url: &str) -> Result<Option<String>, Box<dyn Error>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(TIMEOUT))
        .build()?;
    let res = client
        .get(url)
        .header(USER_AGENT, USER_AGENT_STRING)
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

pub async fn youtube_data(api_key: &str, video_id: &str) -> Result<Option<String>, Box<dyn Error>> {
    let url = format!("https://www.googleapis.com/youtube/v3/videos?part=snippet&part=contentDetails&part=statistics&id={}&key={}", video_id, api_key);

    let client = Client::builder()
        .timeout(Duration::from_secs(TIMEOUT))
        .build()?;
    let response = client
        .get(url)
        .header(USER_AGENT, USER_AGENT_STRING)
        .send()
        .await?;
    let api_response: ApiResponse = response.json().await?;

    let video = api_response
        .items
        .first()
        .ok_or("Could not fetch video data")?;

    let output = format!(
        "Title: {}\r\nDuration: {} | Views: {} | Comments: {} Likes: {}",
        video.snippet.title,
        video
            .content_details
            .duration
            .as_ref()
            .unwrap_or(&String::from("NA")),
        video
            .statistics
            .view_count
            .as_ref()
            .unwrap_or(&String::from("NA")),
        video
            .statistics
            .comment_count
            .as_ref()
            .unwrap_or(&String::from("NA")),
        video
            .statistics
            .like_count
            .as_ref()
            .unwrap_or(&String::from("NA")),
    );

    Ok(Some(output))
}

pub fn extract_video_id(url: &str) -> Option<String> {
    let parsed_url = url::Url::parse(url).ok()?;

    match parsed_url.domain()? {
        "www.youtube.com" | "youtube.com" => {
            if parsed_url.path().to_lowercase().contains("/watch") {
                parsed_url.query_pairs().find_map(|(key, value)| {
                    if key == "v" {
                        Some(value.into_owned())
                    } else {
                        None
                    }
                })
            } else if parsed_url.path().to_lowercase().contains("/shorts")
                || parsed_url.path().to_lowercase().contains("/live")
            {
                parsed_url.path_segments()?.nth(1).map(|s| s.to_string())
            } else {
                None
            }
        }
        "youtu.be" => parsed_url.path_segments()?.next().map(|s| s.to_string()),
        _ => None,
    }
}
