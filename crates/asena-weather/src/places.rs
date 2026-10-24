//! Saved places and the Open-Meteo place search.
use crate::forecast::Units;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
    /// Region and country, for telling places with the same name apart.
    pub region: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub places: Vec<Place>,
    pub selected: usize,
    pub units: Units,
}

impl Saved {
    pub fn parse(text: &str) -> Self {
        let mut saved: Self = serde_json::from_str(text).unwrap_or_default();
        if saved.selected >= saved.places.len() {
            saved.selected = 0;
        }
        saved
    }

    pub fn current(&self) -> Option<&Place> {
        self.places.get(self.selected)
    }

    /// Adds a place unless it is already saved, and selects it.
    pub fn add(&mut self, place: Place) {
        let existing = self.places.iter().position(|saved| {
            (saved.latitude - place.latitude).abs() < 0.01
                && (saved.longitude - place.longitude).abs() < 0.01
        });
        self.selected = match existing {
            Some(index) => index,
            None => {
                self.places.push(place);
                self.places.len() - 1
            }
        };
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.places.len() {
            self.places.remove(index);
            if self.selected > index || self.selected >= self.places.len() {
                self.selected = self.selected.saturating_sub(1);
            }
        }
    }
}

/// `tr_TR.UTF-8` → `tr`; the search answers with names in this language.
pub fn language(locale: &str) -> String {
    let code: String = locale
        .chars()
        .take_while(|character| character.is_ascii_alphabetic())
        .collect();
    if code.len() == 2 || code.len() == 3 {
        code.to_ascii_lowercase()
    } else {
        "en".into()
    }
}

pub fn search_url(query: &str, language: &str) -> String {
    format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=10&language={language}&format=json",
        glib::Uri::escape_string(query.trim(), None, false)
    )
}

#[derive(Deserialize)]
struct SearchResponse {
    #[serde(default)]
    results: Vec<SearchResult>,
}

#[derive(Deserialize)]
struct SearchResult {
    name: String,
    latitude: f64,
    longitude: f64,
    #[serde(default)]
    admin1: Option<String>,
    #[serde(default)]
    country: Option<String>,
}

pub fn parse_search(json: &str) -> Result<Vec<Place>, serde_json::Error> {
    let response: SearchResponse = serde_json::from_str(json)?;
    Ok(response
        .results
        .into_iter()
        .map(|result| {
            let mut region: Vec<String> = Vec::new();
            for part in [result.admin1, result.country].into_iter().flatten() {
                if !part.is_empty() && part != result.name && !region.contains(&part) {
                    region.push(part);
                }
            }
            Place {
                name: result.name,
                region: region.join(", "),
                latitude: result.latitude,
                longitude: result.longitude,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(name: &str, latitude: f64) -> Place {
        Place {
            name: name.into(),
            region: String::new(),
            latitude,
            longitude: 29.0,
        }
    }

    #[test]
    fn search_results_name_their_region() -> Result<(), serde_json::Error> {
        let json = r#"{"results": [
            {"id": 1, "name": "İzmir", "latitude": 38.41, "longitude": 27.14,
             "admin1": "İzmir", "country": "Türkiye"},
            {"id": 2, "name": "Kars", "latitude": 40.6, "longitude": 43.1, "country": "Türkiye"}
        ]}"#;
        let places = parse_search(json)?;
        assert_eq!(places.len(), 2);
        assert_eq!(places[0].region, "Türkiye");
        assert_eq!(places[1].name, "Kars");
        assert!(parse_search(r#"{"generationtime_ms": 0.5}"#)?.is_empty());
        Ok(())
    }

    #[test]
    fn search_url_escapes_the_query() {
        let url = search_url(" Şanlıurfa merkez ", "tr");
        assert!(url.contains("name=%C5%9Eanl%C4%B1urfa%20merkez&"));
        assert!(url.ends_with("language=tr&format=json"));
    }

    #[test]
    fn language_comes_from_the_locale() {
        assert_eq!(language("tr_TR.UTF-8"), "tr");
        assert_eq!(language("en_US"), "en");
        assert_eq!(language("C"), "en");
    }

    #[test]
    fn adding_selects_and_does_not_duplicate() {
        let mut saved = Saved::default();
        saved.add(place("A", 40.0));
        saved.add(place("B", 41.0));
        saved.add(place("A again", 40.001));
        assert_eq!(saved.places.len(), 2);
        assert_eq!(saved.selected, 0);
    }

    #[test]
    fn removing_keeps_a_valid_selection() {
        let mut saved = Saved::default();
        saved.add(place("A", 40.0));
        saved.add(place("B", 41.0));
        saved.add(place("C", 42.0));
        saved.remove(0);
        assert_eq!(saved.current().map(|place| place.name.as_str()), Some("C"));
        saved.remove(1);
        assert_eq!(saved.current().map(|place| place.name.as_str()), Some("B"));
        saved.remove(0);
        assert!(saved.current().is_none());
    }

    #[test]
    fn broken_files_start_empty() {
        let saved = Saved::parse("not json");
        assert!(saved.places.is_empty());
        let saved = Saved::parse(r#"{"places": [], "selected": 4, "units": "Imperial"}"#);
        assert_eq!(saved.selected, 0);
        assert_eq!(saved.units, Units::Imperial);
    }
}
