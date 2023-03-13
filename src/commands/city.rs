use crate::database::{CsvRecord, Database};
use std::sync::Arc;
use tokio::sync::Mutex;

struct City {
    city: String,
    city_ascii: String,
    lat: String,
    lon: String,
    country: String,
    iso2: String,
    iso3: String,
    admin_name: String,
    capital: String,
    population: String,
    id: String,
}

impl CsvRecord for City {
    fn from_fields(fields: &[String]) -> Self {
        Self {
            city: fields[0].clone(),
            city_ascii: fields[1].clone(),
            lat: fields[2].clone(),
            lon: fields[3].clone(),
            country: fields[4].clone(),
            iso2: fields[5].clone(),
            iso3: fields[6].clone(),
            admin_name: fields[7].clone(),
            capital: fields[8].clone(),
            population: fields[9].clone(),
            id: fields[10].clone(),
        }
    }

    fn to_fields(&self) -> Vec<String> {
        vec![
            self.city.clone(),
            self.city_ascii.clone(),
            self.lat.clone(),
            self.lon.clone(),
            self.country.clone(),
            self.iso2.clone(),
            self.iso3.clone(),
            self.admin_name.clone(),
            self.capital.clone(),
            self.population.clone(),
            self.id.clone(),
        ]
    }
}

pub async fn city(args: &[String], db: Arc<Mutex<Database>>) -> String {
    if args.len() < 1 {
        return String::from("Please provide a city");
    }

    let cities: Vec<City> = match db.lock().await.select("cities", |c: &City| {
        c.city.to_lowercase() == args.join(" ").to_lowercase()
    }) {
        Ok(cities_result) => match cities_result {
            Some(cities) => cities,
            None => return String::from("Could not find city."),
        },
        Err(_) => return String::from("Could not find city."),
    };

    if cities.len() > 0 {
        let n = std::cmp::min(cities.len(), 5);

        let output: String = cities
            .iter()
            .take(n)
            .map(|cities| {
                format!(
                    "City: {} | Country: {} | Lat: {} Lon: {} | Population: {}\r\n",
                    cities.city, cities.country, cities.lat, cities.lon, cities.population
                )
            })
            .collect();

        return output;
    } else {
        return String::from("Could not find city.");
    }
}
