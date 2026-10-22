//! Pure helpers for world clocks, the stopwatch and the timer.
use asena_core::text::fold;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Zone {
    /// IANA identifier, for example `Europe/Istanbul`.
    pub id: String,
    /// Human readable city, for example `Istanbul`.
    pub city: String,
}

/// City part of an IANA identifier.
pub fn city_of(id: &str) -> String {
    id.rsplit('/').next().unwrap_or(id).replace('_', " ")
}

/// Reads `zone1970.tab`: country codes, coordinates, identifier, comment.
pub fn parse_zone_table(text: &str) -> Vec<Zone> {
    let mut zones: Vec<Zone> = text
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| line.split('\t').nth(2))
        .map(|id| Zone {
            id: id.to_owned(),
            city: city_of(id),
        })
        .collect();
    zones.sort_by(|a, b| a.city.cmp(&b.city));
    zones.dedup_by(|a, b| a.id == b.id);
    zones
}

pub fn zone_matches(zone: &Zone, query: &str) -> bool {
    let query = fold(query.trim());
    query.is_empty() || fold(&zone.city).contains(&query) || fold(&zone.id).contains(&query)
}

/// Difference to local time, e.g. "+3 h", "-1 h 30 min" or `same`.
pub fn offset_label(seconds: i64, same: &str, hours: &str, minutes: &str) -> String {
    if seconds == 0 {
        return same.to_owned();
    }
    let sign = if seconds > 0 { "+" } else { "-" };
    let total = seconds.unsigned_abs() / 60;
    let (h, m) = (total / 60, total % 60);
    match (h, m) {
        (0, m) => format!("{sign}{m} {minutes}"),
        (h, 0) => format!("{sign}{h} {hours}"),
        (h, m) => format!("{sign}{h} {hours} {m} {minutes}"),
    }
}

/// Stopwatch display: minutes, seconds and hundredths; hours when needed.
pub fn stopwatch_label(millis: u64) -> String {
    let hundredths = (millis / 10) % 100;
    let seconds = (millis / 1000) % 60;
    let minutes = (millis / 60_000) % 60;
    let hours = millis / 3_600_000;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02},{hundredths:02}")
    } else {
        format!("{minutes:02}:{seconds:02},{hundredths:02}")
    }
}

/// Timer display, rounding up so it reads 00:00:01 until the last second ends.
pub fn timer_label(millis: u64) -> String {
    let total = millis.div_ceil(1000);
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total / 60) % 60,
        total % 60
    )
}

/// Saved world clocks: one IANA identifier per line, unknown lines skipped.
pub fn parse_saved(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for line in text.lines().map(str::trim) {
        if !line.is_empty() && !line.starts_with('#') && !ids.iter().any(|id| id == line) {
            ids.push(line.to_owned());
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_table_parsing_and_search() {
        let zones = parse_zone_table(
            "# comment\nTR\t+4101+02858\tEurope/Istanbul\nUS\t+404251-0740023\tAmerica/New_York\tEastern\n",
        );
        assert_eq!(zones.len(), 2);
        assert_eq!(zones[0].city, "Istanbul");
        assert_eq!(zones[1].city, "New York");
        assert!(zone_matches(&zones[0], "İSTANBUL"));
        assert!(zone_matches(&zones[1], "america"));
        assert!(!zone_matches(&zones[1], "tokyo"));
    }

    #[test]
    fn labels() {
        assert_eq!(offset_label(0, "Same time", "h", "min"), "Same time");
        assert_eq!(offset_label(10_800, "", "h", "min"), "+3 h");
        assert_eq!(offset_label(-5_400, "", "h", "min"), "-1 h 30 min");
        assert_eq!(offset_label(1_800, "", "sa", "dk"), "+30 dk");
        assert_eq!(stopwatch_label(61_230), "01:01,23");
        assert_eq!(stopwatch_label(3_600_000), "1:00:00,00");
        assert_eq!(timer_label(1), "00:00:01");
        assert_eq!(timer_label(3_661_000), "01:01:01");
        assert_eq!(
            parse_saved("Europe/Istanbul\n\nEurope/Istanbul\nAsia/Tokyo"),
            vec!["Europe/Istanbul", "Asia/Tokyo"]
        );
    }
}
