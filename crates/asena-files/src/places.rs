//! Navigation history and location helpers for Asena Files.
use asena_core::text::fold;

/// Back and forward stacks of visited location URIs.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct History {
    back: Vec<String>,
    forward: Vec<String>,
}

impl History {
    /// Records leaving `current` for a new location.
    pub fn visit(&mut self, current: &str) {
        if self.back.last().map(String::as_str) != Some(current) {
            self.back.push(current.to_owned());
        }
        self.forward.clear();
    }

    pub fn back(&mut self, current: &str) -> Option<String> {
        let previous = self.back.pop()?;
        self.forward.push(current.to_owned());
        Some(previous)
    }

    pub fn forward(&mut self, current: &str) -> Option<String> {
        let next = self.forward.pop()?;
        self.back.push(current.to_owned());
        Some(next)
    }

    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }
}

/// Accent and case insensitive file name filter, Turkish I aware.
pub fn name_matches(name: &str, query: &str) -> bool {
    let query = fold(query.trim());
    query.is_empty() || fold(name).contains(&query)
}

/// Sort orders offered in the view menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortMode {
    #[default]
    Name,
    Modified,
    Size,
}

impl SortMode {
    pub fn from_key(key: &str) -> Self {
        match key {
            "modified" => Self::Modified,
            "size" => Self::Size,
            _ => Self::Name,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Modified => "modified",
            Self::Size => "size",
        }
    }
}

/// A file name the user typed must stay inside the current folder.
pub fn valid_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && name != "." && name != ".." && !name.contains('/') && !name.contains('\0')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_moves_between_stacks() {
        let mut history = History::default();
        history.visit("file:///home");
        history.visit("file:///home/a");
        assert_eq!(
            history.back("file:///home/a/b").as_deref(),
            Some("file:///home/a")
        );
        assert!(history.can_forward());
        assert_eq!(
            history.forward("file:///home/a").as_deref(),
            Some("file:///home/a/b")
        );
        history.visit("file:///tmp");
        assert!(!history.can_forward());
        assert!(history.can_back());
    }

    #[test]
    fn filter_folds_turkish_letters() {
        assert!(name_matches("İzmir Raporu.pdf", "izmir"));
        assert!(name_matches("Işık.txt", "ISIK"));
        assert!(name_matches("anything", "  "));
        assert!(!name_matches("Belgeler", "müzik"));
    }

    #[test]
    fn sort_keys_round_trip_and_names_are_checked() {
        for mode in [SortMode::Name, SortMode::Modified, SortMode::Size] {
            assert_eq!(SortMode::from_key(mode.key()), mode);
        }
        assert!(valid_name("Yeni klasör"));
        assert!(!valid_name("../x"));
        assert!(!valid_name(""));
    }
}
