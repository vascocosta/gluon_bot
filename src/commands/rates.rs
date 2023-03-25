use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const CURRENCY_API_URL: &str = "https://api.currencyapi.com/v3/latest";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Currency {
    pub meta: Meta,
    pub data: Data,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    #[serde(rename = "last_updated_at")]
    pub last_updated_at: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    #[serde(rename = "AUD")]
    pub aud: Aud,
    #[serde(rename = "BTC")]
    pub btc: Btc,
    #[serde(rename = "CNY")]
    pub cny: Cny,
    #[serde(rename = "EUR")]
    pub eur: Eur,
    #[serde(rename = "GBP")]
    pub gbp: Gbp,
    #[serde(rename = "JPY")]
    pub jpy: Jpy,
    #[serde(rename = "RUB")]
    pub rub: Rub,
    #[serde(rename = "SAR")]
    pub sar: Sar,
    #[serde(rename = "USD")]
    pub usd: Usd,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Aud {
    pub code: String,
    pub value: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Btc {
    pub code: String,
    pub value: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cny {
    pub code: String,
    pub value: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Eur {
    pub code: String,
    pub value: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gbp {
    pub code: String,
    pub value: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Jpy {
    pub code: String,
    pub value: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rub {
    pub code: String,
    pub value: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sar {
    pub code: String,
    pub value: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usd {
    pub code: String,
    pub value: f64,
}

pub async fn rates(args: &[String], options: &HashMap<String, String>) -> String {
    let base_currency = match args.len() {
        0 => "EUR",
        _ => &args[0],
    };
    let currencies: Currency = match reqwest::get(format!(
        "{CURRENCY_API_URL}/?apikey={}&base_currency={}",
        match options.get("currency_api_key") {
            Some(currency_api_key) => currency_api_key,
            None => "",
        },
        base_currency,
    ))
    .await
    {
        Ok(response) => match response.json().await {
            Ok(currencies) => currencies,
            Err(_) => return String::from("Could not use base currency."),
        },
        Err(_) => return String::from("Could not fetch data."),
    };
    let updated = match currencies.meta.last_updated_at.parse() {
        Ok(updated) => updated,
        Err(_) => Utc::now(),
    };

    format!(
        "\x02CUR:\x02 {} \x02Updated:\x02 {}\r\n\
        {}: {}\r\n\
        {}: {}\r\n\
        {}: {}\r\n\
        {}: {}\r\n\
        {}: {}\r\n\
        {}: {}\r\n\
        {}: {}\r\n\
        {}: {}\r\n\
        {}: {}",
        base_currency.to_uppercase(),
        updated.format("%d-%m-%Y %H:%M"),
        currencies.data.aud.code,
        currencies.data.aud.value,
        currencies.data.btc.code,
        currencies.data.btc.value,
        currencies.data.cny.code,
        currencies.data.cny.value,
        currencies.data.eur.code,
        currencies.data.eur.value,
        currencies.data.jpy.code,
        currencies.data.jpy.value,
        currencies.data.rub.code,
        currencies.data.rub.value,
        currencies.data.sar.code,
        currencies.data.sar.value,
        currencies.data.gbp.code,
        currencies.data.gbp.value,
        currencies.data.usd.code,
        currencies.data.usd.value
    )
}
