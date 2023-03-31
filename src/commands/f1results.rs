use chrono::Utc;
use reqwest::{header::USER_AGENT, Client};
use scraper::{Html, Selector};
use tokio::time::Duration;

const EVENT: &str = "1143/australia"; // Hardcoded for now. Need to find a way to do it programmatically.

pub async fn f1results(args: &[String]) -> String {
    if args.len() != 1 {
        return String::from("Please provide a session. Ex: FP1, FP2, FP3, Qualifying, Race");
    }

    let year = Utc::now().format("%Y");
    let session = match args[0].to_lowercase().as_str() {
        "p1" | "fp1" => "practice-1",
        "fp2" => "practice-2",
        "fp3" => "practice-3",
        "quali" | "qualy" | "qualifying" => "qualifying",
        "race" => "race-result",
        _ => return String::from("Session must be one of: FP1, FP2, FP3, Qualifying, Race"),
    };
    let base_url = format!("https://www.formula1.com/en/results.html/{year}/races");
    let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
        Ok(client) => client,
        Err(_) => return String::from("Could not fetch data."),
    };
    let res = match client
        .get(format!("{base_url}/{EVENT}/{session}.html"))
        .header(USER_AGENT, "curl")
        .send()
        .await
    {
        Ok(res) => res,
        Err(_) => return String::from("Could not fetch data."),
    };
    let body = match res.text().await {
        Ok(body) => body,
        Err(_) => return String::from("Could not fetch data."),
    };
    let document = Html::parse_document(&body);
    let row_selector = match Selector::parse("table.resultsarchive-table tr") {
        Ok(row_selector) => row_selector,
        Err(_) => return String::from("Could not fetch data."),
    };
    let mut output: String = String::new();

    for row in document.select(&row_selector).take(7) {
        for (index, cell) in row.select(&Selector::parse("td").unwrap()).enumerate() {
            if index == 3 {
                let entry: String = cell
                    .text()
                    .collect::<String>()
                    .trim()
                    .replace("\n", "")
                    .replace("\r", "")
                    .split_whitespace()
                    .collect();
                output = format!("{}{} ", output, &entry[entry.len() - 3..]);
            } else if index == 0 || index == 2 || index == 4 {
                ()
            } else {
                output = format!(
                    "{}{} ",
                    output,
                    cell.text()
                        .collect::<String>()
                        .trim()
                        .replace("\n", "")
                        .replace("\r", "")
                );
            }
        }
        output = format!("{}\r\n", output);
    }

    format!("{}\r\nSource: {base_url}/{EVENT}/{session}.html", output)
}
