# gluon_bot

General purpose IRC bot written in Rust.

## Features

* Concurrency/Multithreading
* Memory safety
* Performance
* Reliability
* Events (search, announce and notify)
* Games
* Plugins (language agnostic)
* Quotes
* RSS/Atom feeds
* Time zones (events on user's local time)
* Weather (requires an OpenWeatherMap key)

## Build

To build `gluon_bot` you need the `Rust toolchain` as well as these `dependencies`:

* chrono = "0.4.23"
* chrono-tz = "0.8.1"
* circular-queue = "0.2.6"
* csv = "1.2.0"
* feed-rs = "1.3.0"
* futures = "0.3.0"
* irc = "0.15.0"
* newsapi = "0.6.0"
* openweather_sdk = "0.1.2"
* rand = "0.8.5"
* regex = "1.7.3"
* reqwest = { version = "0.11.0", features = ["json"] }
* scraper = "0.15.0"
* serde = { version = "1.0", features = ["derive"] }
* serde_json = "1.0"
* tokio = { version = "1.25.0", features = ["full"] }

Follow these steps to fetch and compile the source of `gluon_bot` and its `dependencies`:

```
git clone https://github.com/vascocosta/gluon_bot.git

cd gluon_bot

cargo build --release
```

## Install

* Create a folder with the name of your bot
* Move the `gluon_bot` binary generated above into that folder
* Create a `data` subfolder
* Create a `config.toml` configuration file like the one below

## Configuration (samples)

### config.toml

```toml
nickname = "Vettel"
alt_nicks = ["Vettel_", "Vettel__"]
username = "gluonbot"
realname = "gluonbot"
server = "irc.quakenet.org"
port = 6667
use_tls = false
encoding = "UTF-8"
channels = ["#aviation", "#formula1", "#geeks", "#motorsport", "#simracing"]
user_info = "I'm a general purpose IRC bot."
version = "0.1.0"

[options]
currency_api_key = "{your_currency_api_key}"
database_path = "data/"
feed_refresh = "300"
first_open_hour = "5"
first_open_min = "30"
first_close_hour = "21"
first_close_min = "0"
news_api_key = "{your_news_api_key}"
news_articles = "3"
omdb_api_key = "{your_omdb_api_key}"
owm_api_key = "{your_owm_api_key}"
owm_api_language = "en"
owm_api_units = "metric"
plugins_path = "plugins"
prefix = "!"
```

### data/events.csv

```csv
[SpaceX],Falcon 9 Block 5,Starlink Group 3-5 Launch,2023-04-27 13:40:00 UTC,#geeks,space spacex,true
[SpaceX],Falcon Heavy,ViaSat-3 Americas Launch,2023-04-27 23:29:00 UTC,#geeks,space spacex,true
[SpaceX],Falcon 9 Block 5,O3b mPower 3 & 4 Launch,2023-04-28 21:12:00 UTC,#geeks,space spacex,true

[Formula 1],Azerbaijan GP,Practice 1,2023-04-28 09:30:00 UTC,#formula1,f1 formula1,true
[Formula 1],Azerbaijan GP,Qualifying,2023-04-28 13:00:00 UTC,#formula1,f1 formula1,true
[Formula 1],Azerbaijan GP,Sprint Shootout,2023-04-29 08:30:00 UTC,#formula1,f1 formula1,true
[Formula 1],Azerbaijan GP,Sprint,2023-04-29 13:30:00 UTC,#formula1,f1 formula1,true
[Formula 1],Azerbaijan GP,Race,2023-04-30 11:00:00 UTC,#formula1,f1 formula1,true
```