use chrono::NaiveDateTime;
use regex::Regex;
use reqwest::{header::USER_AGENT, Client};
use scraper::{Html, Selector};
use serde::Deserialize;
use std::error::Error;
use tokio::time::Duration;

const TIMEOUT: u64 = 10;
const YOUTUBE_API_BASE: &str = "https://www.googleapis.com/youtube/v3";
const USER_AGENT_STRING: &str =
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/111.0";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoSnippet {
    title: String,
    published_at: Option<NaiveDateTime>,
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
    content_details: Option<VideoContentDetails>,
    statistics: Option<VideoStatistics>,
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
    let url = format!(
        "{}/videos?part=snippet&part=contentDetails&part=statistics&id={}&key={}",
        YOUTUBE_API_BASE, video_id, api_key
    );
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
        .ok_or("Could not fetch video data.")?;

    let not_available = String::from("N/A");
    let published_at = video.snippet.published_at.unwrap_or_default();
    let duration = if let Some(content_details) = &video.content_details {
        content_details.duration.as_ref().unwrap_or(&not_available)
    } else {
        &not_available
    };
    let view_count = if let Some(statistics) = &video.statistics {
        statistics.view_count.as_ref().unwrap_or(&not_available)
    } else {
        &not_available
    };
    let comment_count = if let Some(statistics) = &video.statistics {
        statistics.comment_count.as_ref().unwrap_or(&not_available)
    } else {
        &not_available
    };
    let like_count = if let Some(statistics) = &video.statistics {
        statistics.like_count.as_ref().unwrap_or(&not_available)
    } else {
        &not_available
    };

    Ok(Some(format!(
        "{}\r\nPublished: {} | Duration: {} | Views: {} | Comments: {} Likes: {}",
        video.snippet.title,
        published_at.format("%d/%m/%Y"),
        duration.chars().skip(2).collect::<String>().to_lowercase(),
        view_count,
        comment_count,
        like_count
    )))
}

pub fn extract_video_id(url: &str) -> Option<String> {
    let parsed_url = url::Url::parse(url).ok()?;

    match parsed_url.domain()?.to_lowercase().as_str() {
        "www.youtube.com" | "youtube.com" => {
            if parsed_url.path().to_lowercase().contains("/watch") {
                parsed_url.query_pairs().find_map(|(key, value)| {
                    if key == "v" {
                        Some(value.into_owned())
                    } else {
                        None
                    }
                })
            } else if ["/shorts", "/live"]
                .iter()
                .any(|&segment| parsed_url.path().to_lowercase().contains(segment))
            {
                parsed_url.path_segments()?.nth(1).map(|s| s.to_string())
            } else {
                None
            }
        }
        "www.youtu.be" | "youtu.be" => parsed_url.path_segments()?.next().map(|s| s.to_string()),
        _ => None,
    }
}
