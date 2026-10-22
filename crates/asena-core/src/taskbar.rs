//! Taskbar ordering derived from the application index and window model.
use crate::{
    apps::{AppIndex, DesktopEntry},
    text::fold,
};

#[derive(Debug, Clone)]
pub struct Group {
    pub key: String,
    pub app: Option<DesktopEntry>,
    pub app_id: String,
    pub pinned: bool,
    pub windows: Vec<u64>,
}

pub fn groups<'a>(
    index: &AppIndex,
    pinned: &[String],
    windows: impl IntoIterator<Item = (u64, &'a str)>,
) -> Vec<Group> {
    let mut groups = Vec::<Group>::new();
    for pin in pinned {
        let app = index.resolve(pin);
        let key = app.map(|app| app.id.clone()).unwrap_or_else(|| fold(pin));
        if groups.iter().any(|group| group.key == key) {
            continue;
        }
        groups.push(Group {
            key,
            app: app.cloned(),
            app_id: pin.clone(),
            pinned: true,
            windows: vec![],
        });
    }
    for (id, app_id) in windows {
        let app = index.resolve(app_id);
        let key = app.map(|app| app.id.clone()).unwrap_or_else(|| {
            if app_id.is_empty() {
                format!("window:{id}")
            } else {
                fold(app_id)
            }
        });
        if let Some(group) = groups.iter_mut().find(|group| group.key == key) {
            group.windows.push(id);
        } else {
            groups.push(Group {
                key,
                app: app.cloned(),
                app_id: app_id.into(),
                pinned: false,
                windows: vec![id],
            });
        }
    }
    groups
}

/// Milliseconds until the next wall-clock boundary, including subsecond time.
pub fn clock_delay_ms(second: u32, microsecond: u32, show_seconds: bool) -> u64 {
    let seconds = if show_seconds { 1 } else { 60 - second.min(59) };
    (u64::from(seconds) * 1000)
        .saturating_sub(u64::from(microsecond.min(999_999)) / 1000)
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_order_then_open_order_and_blank_ids_stay_separate() {
        let index = AppIndex::default();
        let groups = groups(
            &index,
            &["B".into(), "A".into(), "b".into()],
            [(1, "c"), (2, "a"), (3, "C"), (4, ""), (5, "")],
        );
        assert_eq!(
            groups
                .iter()
                .map(|group| group.key.as_str())
                .collect::<Vec<_>>(),
            ["b", "a", "c", "window:4", "window:5"]
        );
        assert_eq!(groups[2].windows, [1, 3]);
        assert_eq!(groups[1].windows, [2]);
    }
    #[test]
    fn clock_uses_next_boundary_instead_of_drifting_interval() {
        assert_eq!(clock_delay_ms(59, 750_000, false), 250);
        assert_eq!(clock_delay_ms(0, 0, false), 60_000);
        assert_eq!(clock_delay_ms(20, 125_000, true), 875);
    }
}
