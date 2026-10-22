use crate::object_model::{ObjectModel, Properties};
#[derive(Clone, Debug)]
pub struct Adapter {
    pub path: String,
    pub name: String,
    pub powered: bool,
}
#[derive(Clone, Debug)]
pub struct Device {
    pub path: String,
    pub adapter: String,
    pub name: String,
    pub connected: bool,
}
pub fn boolean(properties: &Properties, key: &str) -> bool {
    properties
        .get(key)
        .and_then(|v| bool::try_from(v).ok())
        .unwrap_or(false)
}
pub fn string(properties: &Properties, key: &str) -> Option<String> {
    properties
        .get(key)
        .and_then(|v| <&str>::try_from(v).ok())
        .map(ToOwned::to_owned)
}
pub fn adapters(model: &ObjectModel) -> Vec<Adapter> {
    let mut result = model.with_objects(|objects| {
        objects
            .iter()
            .filter_map(|(path, interfaces)| {
                let properties = interfaces.get("org.bluez.Adapter1")?;
                Some(Adapter {
                    path: path.to_string(),
                    name: string(properties, "Alias")
                        .or_else(|| string(properties, "Name"))
                        .unwrap_or_default(),
                    powered: boolean(properties, "Powered"),
                })
            })
            .collect::<Vec<_>>()
    });
    result.sort_by(|a, b| a.path.cmp(&b.path));
    result
}
pub fn devices(model: &ObjectModel, adapter: &str) -> Vec<Device> {
    model.with_objects(|objects| {
        objects
            .iter()
            .filter_map(|(path, interfaces)| {
                let properties = interfaces.get("org.bluez.Device1")?;
                let owner = properties
                    .get("Adapter")
                    .and_then(|v| <&zbus::zvariant::ObjectPath>::try_from(v).ok())
                    .map(ToString::to_string)?;
                if owner != adapter || !boolean(properties, "Paired") {
                    return None;
                }
                Some(Device {
                    path: path.to_string(),
                    adapter: owner,
                    name: string(properties, "Alias")
                        .or_else(|| string(properties, "Name"))
                        .unwrap_or_default(),
                    connected: boolean(properties, "Connected"),
                })
            })
            .collect()
    })
}
