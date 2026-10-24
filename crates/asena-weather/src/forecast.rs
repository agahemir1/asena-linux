//! Open-Meteo forecast: request URL, response parsing and WMO weather codes.
use asena_app::prelude::gettext;
use serde::{Deserialize, Serialize};

/// The data is CC BY 4.0; the window credits it with this link.
pub const ATTRIBUTION_URL: &str = "https://open-meteo.com/";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Units {
    #[default]
    Metric,
    Imperial,
}

impl Units {
    pub fn id(self) -> &'static str {
        match self {
            Self::Metric => "metric",
            Self::Imperial => "imperial",
        }
    }

    pub fn from_id(id: &str) -> Self {
        if id == "imperial" {
            Self::Imperial
        } else {
            Self::Metric
        }
    }

    pub fn wind(self, speed: f64) -> String {
        let speed = speed.round() as i64;
        match self {
            Self::Metric => gettext("{speed} km/h"),
            Self::Imperial => gettext("{speed} mph"),
        }
        .replace("{speed}", &speed.to_string())
    }
}

/// MET-style coordinates: four decimals are about 10 m, more only identifies the user better.
pub fn forecast_url(latitude: f64, longitude: f64, units: Units) -> String {
    let mut url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={latitude:.4}&longitude={longitude:.4}\
         &current=temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,is_day,\
         wind_speed_10m,wind_direction_10m,pressure_msl\
         &hourly=temperature_2m,weather_code,precipitation_probability,is_day\
         &daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max,\
         sunrise,sunset&timezone=auto&forecast_days=7"
    );
    if units == Units::Imperial {
        url.push_str("&temperature_unit=fahrenheit&wind_speed_unit=mph");
    }
    url
}

#[derive(Deserialize)]
struct Response {
    current: CurrentRaw,
    hourly: HourlyRaw,
    daily: DailyRaw,
}

#[derive(Deserialize)]
struct CurrentRaw {
    time: String,
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: f64,
    weather_code: u16,
    is_day: u8,
    wind_speed_10m: f64,
    wind_direction_10m: f64,
    pressure_msl: f64,
}

#[derive(Deserialize)]
struct HourlyRaw {
    time: Vec<String>,
    temperature_2m: Vec<Option<f64>>,
    weather_code: Vec<Option<u16>>,
    precipitation_probability: Vec<Option<f64>>,
    is_day: Vec<Option<u8>>,
}

#[derive(Deserialize)]
struct DailyRaw {
    time: Vec<String>,
    weather_code: Vec<Option<u16>>,
    temperature_2m_max: Vec<Option<f64>>,
    temperature_2m_min: Vec<Option<f64>>,
    precipitation_probability_max: Vec<Option<f64>>,
    sunrise: Vec<Option<String>>,
    sunset: Vec<Option<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Current {
    pub temperature: f64,
    pub feels_like: f64,
    pub humidity: f64,
    pub code: u16,
    pub is_day: bool,
    pub wind_speed: f64,
    pub wind_direction: f64,
    pub pressure: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hour {
    /// Local time at the place, `HH:MM`.
    pub time: String,
    pub temperature: f64,
    pub code: u16,
    pub precipitation: Option<f64>,
    pub is_day: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Day {
    /// `YYYY-MM-DD` at the place.
    pub date: String,
    pub code: u16,
    pub max: f64,
    pub min: f64,
    pub precipitation: Option<f64>,
    pub sunrise: Option<String>,
    pub sunset: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Forecast {
    pub current: Current,
    /// The next 24 hours, starting with the current one.
    pub hours: Vec<Hour>,
    pub days: Vec<Day>,
}

/// `2026-10-23T14:05` → `14:05`.
fn clock(time: &str) -> String {
    time.split_once('T')
        .map(|(_, clock)| clock.chars().take(5).collect())
        .unwrap_or_default()
}

pub fn parse(json: &str) -> Result<Forecast, serde_json::Error> {
    let raw: Response = serde_json::from_str(json)?;
    let current = Current {
        temperature: raw.current.temperature_2m,
        feels_like: raw.current.apparent_temperature,
        humidity: raw.current.relative_humidity_2m,
        code: raw.current.weather_code,
        is_day: raw.current.is_day != 0,
        wind_speed: raw.current.wind_speed_10m,
        wind_direction: raw.current.wind_direction_10m,
        pressure: raw.current.pressure_msl,
    };
    // Local ISO times compare in order; the current hour is the first one
    // not before the observation time truncated to the hour.
    let this_hour = format!("{}:00", raw.current.time.get(..13).unwrap_or_default());
    let hourly = raw.hourly;
    let hours = hourly
        .time
        .iter()
        .enumerate()
        .filter(|(_, time)| time.as_str() >= this_hour.as_str())
        .filter_map(|(index, time)| {
            Some(Hour {
                time: clock(time),
                temperature: (*hourly.temperature_2m.get(index)?)?,
                code: (*hourly.weather_code.get(index)?)?,
                precipitation: hourly
                    .precipitation_probability
                    .get(index)
                    .copied()
                    .flatten(),
                is_day: hourly.is_day.get(index).copied().flatten().unwrap_or(1) != 0,
            })
        })
        .take(24)
        .collect();
    let daily = raw.daily;
    let days = daily
        .time
        .iter()
        .enumerate()
        .filter_map(|(index, date)| {
            Some(Day {
                date: date.clone(),
                code: (*daily.weather_code.get(index)?)?,
                max: (*daily.temperature_2m_max.get(index)?)?,
                min: (*daily.temperature_2m_min.get(index)?)?,
                precipitation: daily
                    .precipitation_probability_max
                    .get(index)
                    .copied()
                    .flatten(),
                sunrise: daily
                    .sunrise
                    .get(index)
                    .cloned()
                    .flatten()
                    .map(|t| clock(&t)),
                sunset: daily
                    .sunset
                    .get(index)
                    .cloned()
                    .flatten()
                    .map(|t| clock(&t)),
            })
        })
        .collect();
    Ok(Forecast {
        current,
        hours,
        days,
    })
}

pub fn temperature(value: f64) -> String {
    format!("{}°", value.round() as i64)
}

/// Condition text and symbolic icon for a WMO weather interpretation code.
pub fn condition(code: u16, is_day: bool) -> (String, &'static str) {
    let clear = if is_day {
        "weather-clear-symbolic"
    } else {
        "weather-clear-night-symbolic"
    };
    let few = if is_day {
        "weather-few-clouds-symbolic"
    } else {
        "weather-few-clouds-night-symbolic"
    };
    let (text, icon) = match code {
        0 => (gettext("Clear"), clear),
        1 => (gettext("Mostly clear"), few),
        2 => (gettext("Partly cloudy"), few),
        3 => (gettext("Overcast"), "weather-overcast-symbolic"),
        45 | 48 => (gettext("Fog"), "weather-fog-symbolic"),
        51 | 53 | 55 => (gettext("Drizzle"), "weather-showers-scattered-symbolic"),
        56 | 57 => (
            gettext("Freezing drizzle"),
            "weather-showers-scattered-symbolic",
        ),
        61 => (gettext("Light rain"), "weather-showers-symbolic"),
        63 => (gettext("Rain"), "weather-showers-symbolic"),
        65 => (gettext("Heavy rain"), "weather-showers-symbolic"),
        66 | 67 => (gettext("Freezing rain"), "weather-showers-symbolic"),
        71 => (gettext("Light snow"), "weather-snow-symbolic"),
        73 => (gettext("Snow"), "weather-snow-symbolic"),
        75 => (gettext("Heavy snow"), "weather-snow-symbolic"),
        77 => (gettext("Snow grains"), "weather-snow-symbolic"),
        80 | 81 => (
            gettext("Rain showers"),
            "weather-showers-scattered-symbolic",
        ),
        82 => (
            gettext("Heavy rain showers"),
            "weather-showers-scattered-symbolic",
        ),
        85 | 86 => (gettext("Snow showers"), "weather-snow-symbolic"),
        95 => (gettext("Thunderstorm"), "weather-storm-symbolic"),
        96 | 99 => (gettext("Thunderstorm with hail"), "weather-storm-symbolic"),
        _ => (gettext("Unknown"), "weather-overcast-symbolic"),
    };
    (text, icon)
}

/// Eight-point compass direction the wind blows from.
pub fn compass(degrees: f64) -> String {
    let index = ((degrees.rem_euclid(360.0) + 22.5) / 45.0) as usize % 8;
    match index {
        0 => gettext("N"),
        1 => gettext("NE"),
        2 => gettext("E"),
        3 => gettext("SE"),
        4 => gettext("S"),
        5 => gettext("SW"),
        6 => gettext("W"),
        _ => gettext("NW"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "latitude": 41.0, "longitude": 29.0, "timezone": "Europe/Istanbul",
        "current": {"time": "2026-10-23T14:15", "interval": 900, "temperature_2m": 18.4,
            "apparent_temperature": 16.9, "relative_humidity_2m": 64, "weather_code": 2,
            "is_day": 1, "wind_speed_10m": 12.6, "wind_direction_10m": 315, "pressure_msl": 1016.2},
        "hourly": {"time": ["2026-10-23T12:00", "2026-10-23T13:00", "2026-10-23T14:00",
            "2026-10-23T15:00", "2026-10-23T16:00"],
            "temperature_2m": [16.0, 17.2, 18.1, null, 17.5],
            "weather_code": [1, 2, 2, 3, 61],
            "precipitation_probability": [0, 0, 5, 20, null],
            "is_day": [1, 1, 1, 1, 0]},
        "daily": {"time": ["2026-10-23", "2026-10-24"], "weather_code": [61, 0],
            "temperature_2m_max": [19.6, 21.0], "temperature_2m_min": [12.1, null],
            "precipitation_probability_max": [55, 0],
            "sunrise": ["2026-10-23T07:24", "2026-10-24T07:25"],
            "sunset": ["2026-10-23T18:17", null]}
    }"#;

    #[test]
    fn parses_current_conditions() -> Result<(), serde_json::Error> {
        let forecast = parse(SAMPLE)?;
        assert_eq!(forecast.current.code, 2);
        assert!(forecast.current.is_day);
        assert_eq!(forecast.current.humidity, 64.0);
        assert_eq!(temperature(forecast.current.temperature), "18°");
        Ok(())
    }

    #[test]
    fn hours_start_at_the_current_hour_and_skip_missing_values() -> Result<(), serde_json::Error> {
        let forecast = parse(SAMPLE)?;
        let times: Vec<&str> = forecast
            .hours
            .iter()
            .map(|hour| hour.time.as_str())
            .collect();
        assert_eq!(times, ["14:00", "16:00"]);
        assert_eq!(forecast.hours[1].precipitation, None);
        assert!(!forecast.hours[1].is_day);
        Ok(())
    }

    #[test]
    fn days_without_a_temperature_are_dropped() -> Result<(), serde_json::Error> {
        let forecast = parse(SAMPLE)?;
        assert_eq!(forecast.days.len(), 1);
        assert_eq!(forecast.days[0].sunrise.as_deref(), Some("07:24"));
        assert_eq!(forecast.days[0].precipitation, Some(55.0));
        Ok(())
    }

    #[test]
    fn invalid_answers_are_errors() {
        assert!(parse("{}").is_err());
        assert!(parse("<html>").is_err());
    }

    #[test]
    fn url_rounds_coordinates_and_selects_units() {
        let url = forecast_url(41.012_345_6, -8.6, Units::Imperial);
        assert!(url.starts_with("https://api.open-meteo.com/v1/forecast?latitude=41.0123&"));
        assert!(url.contains("longitude=-8.6000"));
        assert!(url.contains("temperature_unit=fahrenheit"));
        assert!(!forecast_url(0.0, 0.0, Units::Metric).contains("fahrenheit"));
    }

    #[test]
    fn rounding_and_directions() {
        assert_eq!(temperature(-0.4), "0°");
        assert_eq!(temperature(-2.6), "-3°");
        assert_eq!(compass(0.0), "N");
        assert_eq!(compass(350.0), "N");
        assert_eq!(compass(315.0), "NW");
        assert_eq!(compass(-90.0), "W");
    }

    #[test]
    fn weather_codes_have_day_and_night_icons() {
        assert_eq!(condition(0, true).1, "weather-clear-symbolic");
        assert_eq!(condition(0, false).1, "weather-clear-night-symbolic");
        assert_eq!(condition(95, true).1, "weather-storm-symbolic");
        assert_eq!(condition(1234, true).0, "Unknown");
    }
}
