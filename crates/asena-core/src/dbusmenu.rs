use std::collections::HashMap;
use zbus::zvariant::OwnedValue;
pub type Layout = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);
#[derive(Clone, Debug)]
pub struct Node {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub visible: bool,
    pub separator: bool,
    pub checked: Option<bool>,
    pub children: Vec<Node>,
}
pub fn parse(layout: Layout) -> Node {
    node(layout, 0, &mut 0)
}
fn node((id, properties, children): Layout, depth: usize, count: &mut usize) -> Node {
    *count += 1;
    let text = |key: &str| {
        properties
            .get(key)
            .and_then(|v| <&str>::try_from(v).ok())
            .unwrap_or("")
            .chars()
            .take(4096)
            .collect::<String>()
    };
    let boolean = |key: &str| {
        properties
            .get(key)
            .and_then(|v| bool::try_from(v).ok())
            .unwrap_or(true)
    };
    let toggle = text("toggle-type");
    let state = properties
        .get("toggle-state")
        .and_then(|v| i32::try_from(v).ok());
    let children = if depth < 4 {
        children
            .into_iter()
            .take(128)
            .filter_map(|value| {
                if *count >= 128 {
                    return None;
                }
                Layout::try_from(value)
                    .ok()
                    .map(|layout| node(layout, depth + 1, count))
            })
            .collect()
    } else {
        Vec::new()
    };
    Node {
        id,
        label: text("label"),
        enabled: boolean("enabled"),
        visible: boolean("visible"),
        separator: text("type") == "separator",
        checked: if toggle == "checkmark" || toggle == "radio" {
            state.filter(|value| *value >= 0).map(|value| value != 0)
        } else {
            None
        },
        children,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::Str;
    #[test]
    fn malformed_children_are_ignored_and_defaults_are_safe() {
        let properties = HashMap::from([
            ("label".into(), OwnedValue::from(Str::from("Open"))),
            ("enabled".into(), false.into()),
        ]);
        let parsed = parse((4, properties, vec![true.into()]));
        assert_eq!(parsed.label, "Open");
        assert!(!parsed.enabled);
        assert!(parsed.visible);
        assert!(parsed.children.is_empty());
    }
}
