//! Shared semantic geometry. GTK anchors and popup placement derive from this model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanelEdge {
    #[default]
    Bottom,
    Top,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MenuLayout {
    #[default]
    CompactList,
    Classic,
    Grid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelLayout {
    pub edge: PanelEdge,
    pub thickness: i32,
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self {
            edge: PanelEdge::Bottom,
            thickness: 48,
        }
    }
}

impl PanelLayout {
    pub fn valid_dimensions(thickness: i32, icon: i32) -> bool {
        (32..=96).contains(&thickness) && (16..=64).contains(&icon) && icon + 8 <= thickness
    }
    pub fn from_settings(settings: &gio::Settings) -> Self {
        use gio::prelude::*;
        let edge = match settings.string("panel-edge").as_str() {
            "top" => PanelEdge::Top,
            "left" => PanelEdge::Left,
            "right" => PanelEdge::Right,
            _ => PanelEdge::Bottom,
        };
        Self {
            edge,
            thickness: settings.int("panel-thickness").clamp(32, 96),
        }
    }
    pub fn inward_edge(self) -> PanelEdge {
        match self.edge {
            PanelEdge::Bottom => PanelEdge::Top,
            PanelEdge::Top => PanelEdge::Bottom,
            PanelEdge::Left => PanelEdge::Right,
            PanelEdge::Right => PanelEdge::Left,
        }
    }

    pub fn is_horizontal(self) -> bool {
        matches!(self.edge, PanelEdge::Bottom | PanelEdge::Top)
    }

    /// [top, bottom, left, right], matching the shell's explicit edge mapping.
    pub fn anchors(self) -> [bool; 4] {
        match self.edge {
            PanelEdge::Bottom => [false, true, true, true],
            PanelEdge::Top => [true, false, true, true],
            PanelEdge::Left => [true, true, true, false],
            PanelEdge::Right => [true, true, false, true],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_reserve_only_the_selected_side() {
        for (edge, horizontal, expected) in [
            (PanelEdge::Bottom, true, [false, true, true, true]),
            (PanelEdge::Top, true, [true, false, true, true]),
            (PanelEdge::Left, false, [true, true, true, false]),
            (PanelEdge::Right, false, [true, true, false, true]),
        ] {
            let layout = PanelLayout {
                edge,
                thickness: 48,
            };
            assert_eq!(layout.is_horizontal(), horizontal);
            assert_eq!(layout.anchors(), expected);
            assert_ne!(layout.inward_edge(), edge);
            assert_eq!(
                PanelLayout {
                    edge: layout.inward_edge(),
                    thickness: 48
                }
                .inward_edge(),
                edge
            );
        }
        assert_eq!(PanelLayout::default().thickness, 48);
        assert_eq!(MenuLayout::default(), MenuLayout::CompactList);
    }
}
