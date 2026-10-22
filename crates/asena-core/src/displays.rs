//! Display layout model shared by asena-daemon and Settings.
//!
//! Modes are physical pixels. Positions are logical pixels in the compositor's
//! global space: an output's logical size is its mode, rotated by the
//! transform, divided by the scale.
use crate::i18n::gettext;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Seconds the user has to keep a new layout before it is reverted.
pub const CONFIRM_SECONDS: u32 = 15;
pub const MIN_SCALE: f64 = 0.5;
pub const MAX_SCALE: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mode {
    pub width: i32,
    pub height: i32,
    /// Millihertz; zero when the compositor does not report it.
    pub refresh: i32,
    #[serde(default)]
    pub preferred: bool,
}

impl Mode {
    pub fn same(&self, other: &Mode) -> bool {
        self.width == other.width && self.height == other.height && self.refresh == other.refresh
    }
}

/// The part of an output a client may change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Connector name, such as `DP-1`.
    pub name: String,
    pub enabled: bool,
    pub mode: Option<Mode>,
    pub x: i32,
    pub y: i32,
    /// `wl_output.transform`: 0 normal, 1–3 rotated, 4–7 flipped.
    pub transform: u32,
    pub scale: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && other.x < self.x + self.width
            && self.y < other.y + other.height
            && other.y < self.y + self.height
    }

    /// Shares an edge segment of positive length; corners alone do not count.
    fn touches(&self, other: &Rect) -> bool {
        let vertical = (self.x + self.width == other.x || other.x + other.width == self.x)
            && self.y < other.y + other.height
            && other.y < self.y + self.height;
        let horizontal = (self.y + self.height == other.y || other.y + other.height == self.y)
            && self.x < other.x + other.width
            && other.x < self.x + self.width;
        vertical || horizontal
    }
}

impl Config {
    fn scale_valid(&self) -> bool {
        self.scale.is_finite() && (MIN_SCALE..=MAX_SCALE).contains(&self.scale)
    }

    /// Logical size, or None without a mode or with an invalid scale.
    pub fn logical_size(&self) -> Option<(i32, i32)> {
        let mode = self.mode?;
        if !self.scale_valid() || mode.width <= 0 || mode.height <= 0 {
            return None;
        }
        let (width, height) = if self.transform % 2 == 1 {
            (mode.height, mode.width)
        } else {
            (mode.width, mode.height)
        };
        // wlroots truncates the effective resolution the same way.
        Some((
            (f64::from(width) / self.scale) as i32,
            (f64::from(height) / self.scale) as i32,
        ))
    }

    pub fn rect(&self) -> Option<Rect> {
        let (width, height) = self.logical_size()?;
        Some(Rect {
            x: self.x,
            y: self.y,
            width,
            height,
        })
    }

    fn same_state(&self, other: &Config) -> bool {
        self.name == other.name
            && self.enabled == other.enabled
            && (!self.enabled
                || (self
                    .mode
                    .as_ref()
                    .map(|mode| (mode.width, mode.height, mode.refresh))
                    == other
                        .mode
                        .as_ref()
                        .map(|mode| (mode.width, mode.height, mode.refresh))
                    && self.x == other.x
                    && self.y == other.y
                    && self.transform == other.transform
                    && (self.scale - other.scale).abs() < 1e-6))
    }
}

/// Recognizes the same monitor on another connector or in a later session.
/// Without a serial number the connector tells identical models apart.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Identity {
    pub make: String,
    pub model: String,
    pub serial: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Output {
    pub description: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub modes: Vec<Mode>,
    pub state: Config,
}

impl Output {
    pub fn identity(&self) -> Identity {
        Identity {
            make: self.make.clone(),
            model: self.model.clone(),
            serial: self.serial.clone(),
            connector: self.serial.is_empty().then(|| self.state.name.clone()),
        }
    }

    pub fn title(&self) -> String {
        let title = [self.make.as_str(), self.model.as_str()]
            .into_iter()
            .filter(|part| !part.is_empty() && *part != "Unknown")
            .collect::<Vec<_>>()
            .join(" ");
        if !title.is_empty() {
            title
        } else if !self.description.is_empty() {
            self.description.clone()
        } else {
            self.state.name.clone()
        }
    }

    fn accepts(&self, mode: &Mode) -> bool {
        if self.modes.is_empty() {
            self.state
                .mode
                .as_ref()
                .is_some_and(|current| current.same(mode))
        } else {
            self.modes.iter().any(|known| known.same(mode))
        }
    }
}

/// What the daemon reports to Settings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub available: bool,
    pub pending: bool,
    pub outputs: Vec<Output>,
}

impl Layout {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok()
    }
}

/// The layout Settings sends to the daemon.
pub fn configs_to_json(configs: &[Config]) -> String {
    serde_json::to_string(configs).unwrap_or_default()
}

pub fn configs_from_json(text: &str) -> Option<Vec<Config>> {
    serde_json::from_str(text).ok()
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Invalid {
    #[error("The layout does not match the connected displays")]
    Outputs,
    #[error("At least one display must stay on")]
    NoneEnabled,
    #[error("{0} does not support this mode")]
    Mode(String),
    #[error("{0} has an unsupported scale or rotation")]
    Scale(String),
    #[error("Displays overlap")]
    Overlap,
    #[error("Every display must touch another one")]
    Detached,
}

impl Invalid {
    pub fn message(&self) -> String {
        match self {
            Self::Outputs => gettext("The connected displays changed. Review the layout again."),
            Self::NoneEnabled => gettext("At least one display must stay on."),
            Self::Mode(name) => {
                gettext("{name} does not support this resolution.").replace("{name}", name)
            }
            Self::Scale(name) => {
                gettext("{name} does not support this scale.").replace("{name}", name)
            }
            Self::Overlap => gettext("Displays must not overlap."),
            Self::Detached => gettext("Every display must touch another one."),
        }
    }
}

/// Checks a full layout: exactly the connected outputs, at least one on,
/// supported modes and scales, no overlap and no gaps.
pub fn validate(outputs: &[Output], configs: &[Config]) -> Result<(), Invalid> {
    let connected: BTreeSet<&str> = outputs
        .iter()
        .map(|output| output.state.name.as_str())
        .collect();
    let configured: BTreeSet<&str> = configs.iter().map(|config| config.name.as_str()).collect();
    if connected != configured || configs.len() != outputs.len() {
        return Err(Invalid::Outputs);
    }
    let mut rects = Vec::new();
    for config in configs.iter().filter(|config| config.enabled) {
        let Some(output) = outputs
            .iter()
            .find(|output| output.state.name == config.name)
        else {
            return Err(Invalid::Outputs);
        };
        let Some(mode) = config.mode.as_ref().filter(|mode| output.accepts(mode)) else {
            return Err(Invalid::Mode(output.title()));
        };
        if config.transform > 7 {
            return Err(Invalid::Scale(output.title()));
        }
        let Some(rect) = config.rect().filter(|_| mode.width > 0) else {
            return Err(Invalid::Scale(output.title()));
        };
        rects.push(rect);
    }
    if rects.is_empty() {
        return Err(Invalid::NoneEnabled);
    }
    for (index, rect) in rects.iter().enumerate() {
        if rects[index + 1..].iter().any(|other| rect.overlaps(other)) {
            return Err(Invalid::Overlap);
        }
    }
    let mut reached = vec![false; rects.len()];
    let mut queue = vec![0];
    reached[0] = true;
    while let Some(current) = queue.pop() {
        for next in 0..rects.len() {
            if !reached[next] && rects[current].touches(&rects[next]) {
                reached[next] = true;
                queue.push(next);
            }
        }
    }
    if reached.iter().all(|reached| *reached) {
        Ok(())
    } else {
        Err(Invalid::Detached)
    }
}

/// Moves the enabled outputs so the layout starts at 0,0.
pub fn normalize(configs: &mut [Config]) {
    let enabled = configs.iter().filter(|config| config.enabled);
    let (Some(x), Some(y)) = (
        enabled.clone().map(|config| config.x).min(),
        enabled.map(|config| config.y).min(),
    ) else {
        return;
    };
    for config in configs.iter_mut().filter(|config| config.enabled) {
        config.x -= x;
        config.y -= y;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Right,
    Left,
    Above,
    Below,
}

/// Where `other` lies relative to `primary`.
pub fn side_of(primary: &Rect, other: &Rect) -> Side {
    if other.x >= primary.x + primary.width {
        Side::Right
    } else if other.x + other.width <= primary.x {
        Side::Left
    } else if other.y + other.height <= primary.y {
        Side::Above
    } else {
        Side::Below
    }
}

/// Places every enabled output next to the primary one. Outputs on the same
/// side follow each other outward in list order; side by side outputs share
/// the top edge, stacked outputs the left edge. Unlisted outputs go right.
pub fn arrange(configs: &mut [Config], primary: &str, sides: &[(String, Side)]) {
    let primary = configs
        .iter()
        .position(|config| config.enabled && config.name == primary)
        .or_else(|| configs.iter().position(|config| config.enabled));
    let Some(primary) = primary else {
        return;
    };
    let Some((width, height)) = configs[primary].logical_size() else {
        return;
    };
    configs[primary].x = 0;
    configs[primary].y = 0;
    let (mut right, mut left, mut above, mut below) = (width, 0, 0, height);
    for (index, config) in configs.iter_mut().enumerate() {
        if index == primary || !config.enabled {
            continue;
        }
        let Some((width, height)) = config.logical_size() else {
            continue;
        };
        let side = sides
            .iter()
            .find(|(name, _)| *name == config.name)
            .map_or(Side::Right, |(_, side)| *side);
        match side {
            Side::Right => {
                (config.x, config.y) = (right, 0);
                right += width;
            }
            Side::Left => {
                left -= width;
                (config.x, config.y) = (left, 0);
            }
            Side::Above => {
                above -= height;
                (config.x, config.y) = (0, above);
            }
            Side::Below => {
                (config.x, config.y) = (0, below);
                below += height;
            }
        }
    }
    normalize(configs);
}

pub fn current(outputs: &[Output]) -> Vec<Config> {
    outputs.iter().map(|output| output.state.clone()).collect()
}

/// True when applying `configs` would change nothing.
pub fn unchanged(outputs: &[Output], configs: &[Config]) -> bool {
    configs.len() == outputs.len()
        && configs
            .iter()
            .all(|config| outputs.iter().any(|output| output.state.same_state(config)))
}

/// Settings remembered for each output, keyed by identity.
pub fn remembered(outputs: &[Output]) -> Vec<(Identity, Config)> {
    outputs
        .iter()
        .map(|output| (output.identity(), output.state.clone()))
        .collect()
}

/// Earlier settings for the outputs still connected, under their current
/// connector names. Outputs connected since keep their state. A layout that
/// no longer fits is laid out side by side; None when nothing would be on.
pub fn restore(previous: &[(Identity, Config)], outputs: &[Output]) -> Option<Vec<Config>> {
    let mut configs: Vec<Config> = outputs
        .iter()
        .map(|output| {
            let identity = output.identity();
            previous
                .iter()
                .find(|(known, _)| *known == identity)
                .map(|(_, config)| Config {
                    name: output.state.name.clone(),
                    ..config.clone()
                })
                .filter(|config| {
                    !config.enabled
                        || config
                            .mode
                            .as_ref()
                            .is_some_and(|mode| output.accepts(mode))
                })
                .unwrap_or_else(|| output.state.clone())
        })
        .collect();
    if !configs.iter().any(|config| config.enabled) {
        return None;
    }
    if validate(outputs, &configs).is_err() {
        let first = configs
            .iter()
            .find(|config| config.enabled)
            .map(|config| config.name.clone())
            .unwrap_or_default();
        arrange(&mut configs, &first, &[]);
    }
    validate(outputs, &configs).ok().map(|()| configs)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedOutput {
    pub identity: Identity,
    pub config: Config,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SavedLayout {
    pub outputs: Vec<SavedOutput>,
}

/// Confirmed layouts, one per set of monitors, most recent first.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Saved {
    pub version: u32,
    pub layouts: Vec<SavedLayout>,
}

impl Saved {
    pub const VERSION: u32 = 1;
    pub const LIMIT: usize = 16;

    /// An unreadable or newer file counts as empty; it is replaced on the next save.
    pub fn parse(text: &str) -> Self {
        serde_json::from_str::<Self>(text)
            .ok()
            .filter(|saved| saved.version == Self::VERSION)
            .unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    fn hardware(outputs: &[Output]) -> BTreeSet<Identity> {
        outputs.iter().map(Output::identity).collect()
    }

    pub fn remember(&mut self, outputs: &[Output], configs: &[Config]) {
        let hardware = Self::hardware(outputs);
        self.version = Self::VERSION;
        self.layouts.retain(|layout| {
            layout
                .outputs
                .iter()
                .map(|saved| saved.identity.clone())
                .collect::<BTreeSet<_>>()
                != hardware
        });
        let layout = SavedLayout {
            outputs: outputs
                .iter()
                .filter_map(|output| {
                    let config = configs
                        .iter()
                        .find(|config| config.name == output.state.name)?;
                    Some(SavedOutput {
                        identity: output.identity(),
                        config: config.clone(),
                    })
                })
                .collect(),
        };
        self.layouts.insert(0, layout);
        self.layouts.truncate(Self::LIMIT);
    }

    /// The confirmed layout for exactly these monitors, if it still applies.
    pub fn layout_for(&self, outputs: &[Output]) -> Option<Vec<Config>> {
        let hardware = Self::hardware(outputs);
        let layout = self.layouts.iter().find(|layout| {
            layout
                .outputs
                .iter()
                .map(|saved| saved.identity.clone())
                .collect::<BTreeSet<_>>()
                == hardware
        })?;
        let configs: Vec<Config> = outputs
            .iter()
            .map(|output| {
                let identity = output.identity();
                layout
                    .outputs
                    .iter()
                    .find(|saved| saved.identity == identity)
                    .map(|saved| Config {
                        name: output.state.name.clone(),
                        ..saved.config.clone()
                    })
            })
            .collect::<Option<_>>()?;
        validate(outputs, &configs).ok().map(|()| configs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(width: i32, height: i32) -> Mode {
        Mode {
            width,
            height,
            refresh: 60_000,
            preferred: false,
        }
    }

    fn output(name: &str, serial: &str, width: i32, height: i32, x: i32) -> Output {
        Output {
            description: String::new(),
            make: "Asena".into(),
            model: "Panel".into(),
            serial: serial.into(),
            modes: vec![mode(width, height), mode(1280, 720)],
            state: Config {
                name: name.into(),
                enabled: true,
                mode: Some(mode(width, height)),
                x,
                y: 0,
                transform: 0,
                scale: 1.0,
            },
        }
    }

    fn pair() -> Vec<Output> {
        vec![
            output("DP-1", "A1", 1920, 1080, 0),
            output("HDMI-A-1", "B2", 1280, 720, 1920),
        ]
    }

    #[test]
    fn identity_prefers_the_serial_over_the_connector() {
        let mut first = output("DP-1", "A1", 1920, 1080, 0);
        let mut moved = first.clone();
        moved.state.name = "DP-2".into();
        assert_eq!(first.identity(), moved.identity());
        first.serial.clear();
        moved.serial.clear();
        assert_ne!(first.identity(), moved.identity());
    }

    #[test]
    fn logical_size_rotates_and_scales() {
        let mut config = output("DP-1", "A1", 3840, 2160, 0).state;
        config.scale = 2.0;
        assert_eq!(config.logical_size(), Some((1920, 1080)));
        config.transform = 1;
        assert_eq!(config.logical_size(), Some((1080, 1920)));
        config.scale = 1.5;
        config.transform = 0;
        assert_eq!(config.logical_size(), Some((2560, 1440)));
        config.scale = 0.0;
        assert_eq!(config.logical_size(), None);
    }

    #[test]
    fn valid_side_by_side_layout_passes() {
        let outputs = pair();
        assert_eq!(validate(&outputs, &current(&outputs)), Ok(()));
    }

    #[test]
    fn layout_must_cover_exactly_the_connected_outputs() {
        let outputs = pair();
        let mut configs = current(&outputs);
        configs.pop();
        assert_eq!(validate(&outputs, &configs), Err(Invalid::Outputs));
        let mut configs = current(&outputs);
        configs[1].name = "DP-1".into();
        assert_eq!(validate(&outputs, &configs), Err(Invalid::Outputs));
    }

    #[test]
    fn turning_every_display_off_is_rejected() {
        let outputs = pair();
        let mut configs = current(&outputs);
        for config in &mut configs {
            config.enabled = false;
        }
        assert_eq!(validate(&outputs, &configs), Err(Invalid::NoneEnabled));
        configs[1].enabled = true;
        configs[1].x = 0;
        assert_eq!(validate(&outputs, &configs), Ok(()));
    }

    #[test]
    fn unknown_mode_and_scale_are_rejected() {
        let outputs = pair();
        let mut configs = current(&outputs);
        configs[0].mode = Some(mode(800, 600));
        assert!(matches!(
            validate(&outputs, &configs),
            Err(Invalid::Mode(_))
        ));
        let mut configs = current(&outputs);
        configs[0].scale = 8.0;
        assert!(matches!(
            validate(&outputs, &configs),
            Err(Invalid::Scale(_))
        ));
        configs[0].scale = f64::NAN;
        assert!(matches!(
            validate(&outputs, &configs),
            Err(Invalid::Scale(_))
        ));
    }

    #[test]
    fn overlap_and_gaps_are_rejected() {
        let outputs = pair();
        let mut configs = current(&outputs);
        configs[1].x = 1000;
        assert_eq!(validate(&outputs, &configs), Err(Invalid::Overlap));
        configs[1].x = 2000;
        assert_eq!(validate(&outputs, &configs), Err(Invalid::Detached));
        // Touching only at a corner leaves a gap.
        configs[1].x = 1920;
        configs[1].y = 1080;
        assert_eq!(validate(&outputs, &configs), Err(Invalid::Detached));
    }

    #[test]
    fn arrange_places_outputs_around_the_primary() {
        let outputs = pair();
        let mut configs = current(&outputs);
        arrange(&mut configs, "DP-1", &[("HDMI-A-1".into(), Side::Left)]);
        assert_eq!((configs[1].x, configs[1].y), (0, 0));
        assert_eq!((configs[0].x, configs[0].y), (1280, 0));
        arrange(&mut configs, "DP-1", &[("HDMI-A-1".into(), Side::Above)]);
        assert_eq!((configs[1].x, configs[1].y), (0, 0));
        assert_eq!((configs[0].x, configs[0].y), (0, 720));
        arrange(&mut configs, "DP-1", &[("HDMI-A-1".into(), Side::Below)]);
        assert_eq!((configs[0].x, configs[0].y), (0, 0));
        assert_eq!((configs[1].x, configs[1].y), (0, 1080));
        assert_eq!(validate(&outputs, &configs), Ok(()));
    }

    #[test]
    fn outputs_on_one_side_follow_each_other() {
        let mut outputs = pair();
        outputs.push(output("DP-2", "C3", 1920, 1080, 3200));
        let mut configs = current(&outputs);
        configs[1].scale = 2.0;
        arrange(&mut configs, "DP-1", &[]);
        assert_eq!(configs[1].x, 1920);
        assert_eq!(configs[2].x, 1920 + 640);
        assert_eq!(validate(&outputs, &configs), Ok(()));
        let primary = configs[0]
            .rect()
            .map(|rect| side_of(&rect, &configs[2].rect().unwrap_or(rect)));
        assert_eq!(primary, Some(Side::Right));
    }

    #[test]
    fn side_of_reads_back_every_side() {
        let outputs = pair();
        for side in [Side::Right, Side::Left, Side::Above, Side::Below] {
            let mut configs = current(&outputs);
            arrange(&mut configs, "DP-1", &[("HDMI-A-1".into(), side)]);
            let (Some(primary), Some(other)) = (configs[0].rect(), configs[1].rect()) else {
                panic!("rects");
            };
            assert_eq!(side_of(&primary, &other), side);
        }
    }

    #[test]
    fn restore_maps_by_identity_and_skips_missing_monitors() {
        let outputs = pair();
        let mut earlier = current(&outputs);
        earlier[1].scale = 2.0;
        earlier[1].x = 1920;
        let previous: Vec<_> = outputs
            .iter()
            .zip(&earlier)
            .map(|(output, config)| (output.identity(), config.clone()))
            .collect();
        // The second monitor now sits on another connector.
        let mut now = pair();
        now[1].state.name = "DP-3".into();
        let restored = restore(&previous, &now).unwrap_or_default();
        assert_eq!(restored[1].name, "DP-3");
        assert_eq!(restored[1].scale, 2.0);
        // The second monitor is gone: only the first is restored.
        let only = vec![outputs[0].clone()];
        assert_eq!(restore(&previous, &only), Some(vec![earlier[0].clone()]));
    }

    #[test]
    fn restore_lays_out_again_when_the_old_layout_no_longer_fits() {
        let outputs = pair();
        let previous = remembered(&outputs);
        let mut now = pair();
        // A new monitor appeared where the old layout puts another one.
        now.push(output("DP-2", "C3", 1920, 1080, 1920));
        let restored = restore(&previous, &now).unwrap_or_default();
        assert_eq!(validate(&now, &restored), Ok(()));
    }

    #[test]
    fn restore_never_turns_every_display_off() {
        let outputs = pair();
        let mut configs = current(&outputs);
        for config in &mut configs {
            config.enabled = false;
        }
        let previous: Vec<_> = outputs
            .iter()
            .zip(configs)
            .map(|(output, config)| (output.identity(), config))
            .collect();
        assert_eq!(restore(&previous, &outputs), None);
    }

    #[test]
    fn saved_layouts_apply_only_to_the_same_monitors() {
        let outputs = pair();
        let mut configs = current(&outputs);
        configs[1].scale = 2.0;
        let mut saved = Saved::default();
        saved.remember(&outputs, &configs);
        let reloaded = Saved::parse(&saved.to_json());
        assert_eq!(reloaded.layout_for(&outputs), Some(configs.clone()));
        let mut other = pair();
        other[1].serial = "Z9".into();
        assert_eq!(reloaded.layout_for(&other), None);
        // Remembering the same monitors again replaces the old entry.
        saved.remember(&outputs, &current(&outputs));
        assert_eq!(saved.layouts.len(), 1);
        assert_eq!(saved.layout_for(&outputs), Some(current(&outputs)));
    }

    #[test]
    fn saved_layouts_are_bounded_and_tolerate_bad_files() {
        let mut saved = Saved::default();
        for index in 0..20 {
            let outputs = vec![output("DP-1", &format!("S{index}"), 1920, 1080, 0)];
            saved.remember(&outputs, &current(&outputs));
        }
        assert_eq!(saved.layouts.len(), Saved::LIMIT);
        assert_eq!(Saved::parse("not json"), Saved::default());
        assert_eq!(
            Saved::parse(r#"{"version":9,"layouts":[]}"#),
            Saved::default()
        );
    }

    #[test]
    fn unchanged_ignores_disabled_output_details() {
        let mut outputs = pair();
        outputs[1].state.enabled = false;
        let mut configs = current(&outputs);
        assert!(unchanged(&outputs, &configs));
        // A disabled output keeps no mode or position that matters.
        configs[1].scale = 2.0;
        configs[1].x = 0;
        assert!(unchanged(&outputs, &configs));
        configs[0].scale = 1.25;
        assert!(!unchanged(&outputs, &configs));
    }

    #[test]
    fn layout_round_trips_through_json() {
        let layout = Layout {
            available: true,
            pending: false,
            outputs: pair(),
        };
        assert_eq!(Layout::from_json(&layout.to_json()), Some(layout.clone()));
        let configs = current(&layout.outputs);
        assert_eq!(configs_from_json(&configs_to_json(&configs)), Some(configs));
        assert_eq!(configs_from_json("[{}]"), None);
    }
}
