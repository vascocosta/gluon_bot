# gluon_bot

General purpose IRC bot written in Rust.

# Dependencies

* chrono = "0.4.23"
* chrono-tz = "0.8.1"
* circular-queue = "0.2.6"
* csv = "1.2.0"
* feed-rs = "1.3.0"
* futures = "0.3.0"
* irc = "0.15.0"
* openweathermap = "0.2.4"
* rand = "0.8.5"
* regex = "1.7.3"
* reqwest = { version = "0.11.0", features = ["json"] }
* scraper = "0.15.0"
* serde = { version = "1.0", features = ["derive"] }
* serde_json = "1.0"
* tokio = { version = "1.25.0", features = ["full"] }

# Build

```
git clone https://github.com/vascocosta/gluon_bot.git

cd gluon_bot

cargo build --release
```

# Install

* Create a folder with the name of your bot
* Move the gluon_bot binary generated above into that folder
* Create a "data" subfolder
* Create a "config.toml" configuration file

# Configuration (sample config.toml)

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
omdb_api_key = "{your_omdb_api_key}"
owm_api_key = "{your_owm_api_key}"
owm_api_language = "en"
owm_api_units = "metric"
plugins_path = "plugins"
prefix = "!"
```