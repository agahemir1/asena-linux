//! Minimal iCalendar (RFC 5545) reading and writing for local events.
//!
//! Supported: VEVENT with UID, SUMMARY, LOCATION, DESCRIPTION, DTSTART and
//! DTEND as dates, floating local times or UTC times. Recurrence rules are
//! not expanded; such events appear on their first date.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    pub hour: u32,
    pub minute: u32,
}

impl Time {
    /// Parses "9:05" or "09:05".
    pub fn parse(text: &str) -> Option<Self> {
        let (hour, minute) = text.trim().split_once(':')?;
        let hour = hour.trim().parse().ok()?;
        let minute = minute.trim().parse().ok()?;
        (hour < 24 && minute < 60).then_some(Self { hour, minute })
    }

    pub fn label(self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub uid: String,
    pub title: String,
    pub date: Date,
    /// `None` for all day events.
    pub start: Option<Time>,
    pub end: Option<Time>,
    pub location: String,
    pub notes: String,
}

pub fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

pub fn unescape(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') | Some('N') => result.push('\n'),
                Some(other) => result.push(other),
                None => {}
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Joins folded content lines (continuations start with a space or tab).
fn unfold(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in text.lines() {
        if let Some(rest) = raw.strip_prefix([' ', '\t']) {
            if let Some(last) = lines.last_mut() {
                last.push_str(rest);
                continue;
            }
        }
        lines.push(raw.trim_end_matches('\r').to_owned());
    }
    lines
}

/// Converts a UTC wall time to local time; supplied by the caller so the
/// parser stays pure and testable.
pub type UtcToLocal = dyn Fn(Date, Time) -> Option<(Date, Time)>;

fn parse_moment(value: &str, to_local: &UtcToLocal) -> Option<(Date, Option<Time>)> {
    let digits = |range: std::ops::Range<usize>| value.get(range)?.parse::<u32>().ok();
    let date = Date {
        year: value.get(0..4)?.parse().ok()?,
        month: digits(4..6)?,
        day: digits(6..8)?,
    };
    if !(1..=12).contains(&date.month) || !(1..=31).contains(&date.day) {
        return None;
    }
    if value.len() == 8 {
        return Some((date, None));
    }
    if value.get(8..9) != Some("T") {
        return None;
    }
    let time = Time {
        hour: digits(9..11)?,
        minute: digits(11..13)?,
    };
    if time.hour > 23 || time.minute > 59 {
        return None;
    }
    if value.ends_with('Z') {
        let (date, time) = to_local(date, time)?;
        return Some((date, Some(time)));
    }
    Some((date, Some(time)))
}

pub fn parse(text: &str, to_local: &UtcToLocal) -> Vec<Event> {
    let mut events = Vec::new();
    let mut current: Option<Event> = None;
    let mut index = 0;
    for line in unfold(text) {
        let Some((head, value)) = line.split_once(':') else {
            continue;
        };
        let name = head.split(';').next().unwrap_or("").to_ascii_uppercase();
        match (name.as_str(), value) {
            ("BEGIN", "VEVENT") => {
                index += 1;
                current = Some(Event {
                    uid: format!("imported-{index}"),
                    title: String::new(),
                    date: Date {
                        year: 1970,
                        month: 1,
                        day: 1,
                    },
                    start: None,
                    end: None,
                    location: String::new(),
                    notes: String::new(),
                });
            }
            ("END", "VEVENT") => {
                if let Some(event) = current.take() {
                    if event.date.year != 1970 || !event.title.is_empty() {
                        events.push(event);
                    }
                }
            }
            (field, value) => {
                let Some(event) = current.as_mut() else {
                    continue;
                };
                match field {
                    "UID" => event.uid = unescape(value),
                    "SUMMARY" => event.title = unescape(value),
                    "LOCATION" => event.location = unescape(value),
                    "DESCRIPTION" => event.notes = unescape(value),
                    "DTSTART" => {
                        if let Some((date, time)) = parse_moment(value.trim(), to_local) {
                            event.date = date;
                            event.start = time;
                        }
                    }
                    "DTEND" => {
                        if let Some((_, time)) = parse_moment(value.trim(), to_local) {
                            event.end = time;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    events
}

fn fold_line(line: &str) -> String {
    // RFC 5545 limits lines to 75 octets; continue on the next line with a space.
    let mut result = String::new();
    let mut length = 0;
    for c in line.chars() {
        if length + c.len_utf8() > 74 {
            result.push_str("\r\n ");
            length = 1;
        }
        result.push(c);
        length += c.len_utf8();
    }
    result
}

pub fn serialize(events: &[Event]) -> String {
    let mut out =
        String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Asena Linux//Calendar//TR\r\n");
    for event in events {
        let date = format!(
            "{:04}{:02}{:02}",
            event.date.year, event.date.month, event.date.day
        );
        out.push_str("BEGIN:VEVENT\r\n");
        out.push_str(&fold_line(&format!("UID:{}", escape(&event.uid))));
        out.push_str("\r\n");
        match event.start {
            Some(start) => {
                out.push_str(&format!(
                    "DTSTART:{date}T{:02}{:02}00\r\n",
                    start.hour, start.minute
                ));
                let end = event.end.filter(|end| *end > start).unwrap_or(start);
                out.push_str(&format!(
                    "DTEND:{date}T{:02}{:02}00\r\n",
                    end.hour, end.minute
                ));
            }
            None => out.push_str(&format!("DTSTART;VALUE=DATE:{date}\r\n")),
        }
        for (name, value) in [
            ("SUMMARY", &event.title),
            ("LOCATION", &event.location),
            ("DESCRIPTION", &event.notes),
        ] {
            if !value.is_empty() {
                out.push_str(&fold_line(&format!("{name}:{}", escape(value))));
                out.push_str("\r\n");
            }
        }
        out.push_str("END:VEVENT\r\n");
    }
    out.push_str("END:VCALENDAR\r\n");
    out
}

/// Events on a day, all day events first, then by start time and title.
pub fn on_day(events: &[Event], date: Date) -> Vec<&Event> {
    let mut day: Vec<&Event> = events.iter().filter(|event| event.date == date).collect();
    day.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.title.cmp(&b.title)));
    day
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(date: Date, time: Time) -> Option<(Date, Time)> {
        Some((
            date,
            Time {
                hour: (time.hour + 3) % 24,
                minute: time.minute,
            },
        ))
    }

    fn event(title: &str, start: Option<Time>) -> Event {
        Event {
            uid: format!("uid-{title}"),
            title: title.into(),
            date: Date {
                year: 2026,
                month: 10,
                day: 21,
            },
            start,
            end: start.map(|time| Time {
                hour: time.hour + 1,
                minute: time.minute,
            }),
            location: "Kadıköy; İstanbul".into(),
            notes: "Satır 1\nSatır 2".into(),
        }
    }

    #[test]
    fn round_trip_keeps_fields_and_escapes() {
        let events = vec![
            event(
                "Toplantı, proje",
                Some(Time {
                    hour: 9,
                    minute: 30,
                }),
            ),
            event("Bayram", None),
        ];
        let text = serialize(&events);
        assert!(text.contains("SUMMARY:Toplantı\\, proje"));
        assert_eq!(parse(&text, &identity), events);
    }

    #[test]
    fn parses_folded_lines_dates_and_utc() {
        let text = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nSUMMARY:Uzun bir \r\n başlık\r\nDTSTART:20261021T070000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:Gün\r\nDTSTART;VALUE=DATE:20261022\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let events = parse(text, &identity);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].title, "Uzun bir başlık");
        assert_eq!(
            events[0].start,
            Some(Time {
                hour: 10,
                minute: 0
            })
        );
        assert_eq!(
            events[1].date,
            Date {
                year: 2026,
                month: 10,
                day: 22
            }
        );
        assert_eq!(events[1].start, None);
    }

    #[test]
    fn day_ordering_and_time_parsing() {
        let events = vec![
            event(
                "B",
                Some(Time {
                    hour: 14,
                    minute: 0,
                }),
            ),
            event("A", None),
            event(
                "C",
                Some(Time {
                    hour: 8,
                    minute: 15,
                }),
            ),
        ];
        let titles: Vec<_> = on_day(
            &events,
            Date {
                year: 2026,
                month: 10,
                day: 21,
            },
        )
        .iter()
        .map(|event| event.title.as_str())
        .collect();
        assert_eq!(titles, vec!["A", "C", "B"]);
        assert_eq!(Time::parse("9:05"), Some(Time { hour: 9, minute: 5 }));
        assert_eq!(Time::parse("24:00"), None);
        assert_eq!(Time::parse("ab"), None);
        assert!(fold_line(&"ş".repeat(60)).contains("\r\n "));
    }
}
