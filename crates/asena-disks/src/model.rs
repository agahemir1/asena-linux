//! Disks and volumes from the UDisks2 object tree.
use asena_services::object_model::{Objects, Properties};
use std::collections::HashMap;
use zbus::zvariant::{ObjectPath, Value};

pub const DRIVE: &str = "org.freedesktop.UDisks2.Drive";
pub const BLOCK: &str = "org.freedesktop.UDisks2.Block";
pub const PARTITION: &str = "org.freedesktop.UDisks2.Partition";
pub const TABLE: &str = "org.freedesktop.UDisks2.PartitionTable";
pub const FILESYSTEM: &str = "org.freedesktop.UDisks2.Filesystem";
pub const ATA: &str = "org.freedesktop.UDisks2.Drive.Ata";
pub const NVME: &str = "org.freedesktop.UDisks2.NVMe.Controller";
const LOOP: &str = "org.freedesktop.UDisks2.Loop";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    SolidState,
    Rotating,
    Removable,
    Optical,
}

impl Kind {
    pub fn icon(self) -> &'static str {
        match self {
            Self::SolidState | Self::Rotating => "drive-harddisk-symbolic",
            Self::Removable => "drive-removable-media-symbolic",
            Self::Optical => "drive-optical-symbolic",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Health {
    #[default]
    Unknown,
    Good,
    Failing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Volume {
    /// Block object path.
    pub path: String,
    pub device: String,
    pub label: String,
    pub filesystem: String,
    pub size: u64,
    pub number: Option<u32>,
    pub mount_points: Vec<String>,
    pub mountable: bool,
    /// Part of the running system; formatting is not offered.
    pub system: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Disk {
    /// Drive object path.
    pub path: String,
    pub name: String,
    pub size: u64,
    pub kind: Kind,
    pub ejectable: bool,
    pub can_power_off: bool,
    /// Block object of the whole disk.
    pub block: Option<String>,
    pub table: Option<String>,
    pub health: Health,
    /// Celsius.
    pub temperature: Option<f64>,
    pub power_on_hours: Option<u64>,
    pub volumes: Vec<Volume>,
}

impl Disk {
    pub fn system(&self) -> bool {
        self.volumes.iter().any(|volume| volume.system)
    }
}

fn string(properties: &Properties, key: &str) -> String {
    properties
        .get(key)
        .and_then(|value| <&str>::try_from(value).ok())
        .unwrap_or_default()
        .to_owned()
}

fn boolean(properties: &Properties, key: &str) -> bool {
    properties
        .get(key)
        .and_then(|value| bool::try_from(value).ok())
        .unwrap_or(false)
}

fn unsigned(properties: &Properties, key: &str) -> Option<u64> {
    let value = properties.get(key)?;
    u64::try_from(value)
        .ok()
        .or_else(|| u32::try_from(value).ok().map(u64::from))
        .or_else(|| u16::try_from(value).ok().map(u64::from))
}

fn path(properties: &Properties, key: &str) -> Option<String> {
    properties
        .get(key)
        .and_then(|value| <&ObjectPath>::try_from(value).ok())
        .map(|path| path.to_string())
        .filter(|path| path != "/")
}

/// UDisks sends paths as NUL-terminated byte arrays.
fn bytes(value: &Value) -> Option<String> {
    let Value::Array(array) = value else {
        return None;
    };
    let raw: Vec<u8> = array
        .iter()
        .filter_map(|byte| match byte {
            Value::U8(byte) => Some(*byte),
            _ => None,
        })
        .take_while(|byte| *byte != 0)
        .collect();
    Some(String::from_utf8_lossy(&raw).into_owned())
}

fn mount_points(properties: &Properties) -> Vec<String> {
    match properties.get("MountPoints").map(|value| &**value) {
        Some(Value::Array(points)) => points.iter().filter_map(bytes).collect(),
        _ => Vec::new(),
    }
}

/// Mounted at the root, boot or swap of the running system.
fn is_system(points: &[String]) -> bool {
    points.iter().any(|point| {
        point == "/" || point.starts_with("/boot") || point == "/home" || point == "/usr"
    })
}

pub fn disks(objects: &Objects) -> Vec<Disk> {
    let mut volumes: HashMap<String, Vec<Volume>> = HashMap::new();
    let mut whole: HashMap<String, (String, Option<String>)> = HashMap::new();
    for (object, interfaces) in objects {
        let Some(block) = interfaces.get(BLOCK) else {
            continue;
        };
        if interfaces.contains_key(LOOP) || boolean(block, "HintIgnore") {
            continue;
        }
        let Some(drive) = path(block, "Drive") else {
            continue;
        };
        if !interfaces.contains_key(PARTITION) {
            // The whole disk; it carries a partition table or a filesystem directly.
            whole.insert(
                drive.clone(),
                (
                    object.to_string(),
                    interfaces.contains_key(TABLE).then(|| object.to_string()),
                ),
            );
            if !interfaces.contains_key(FILESYSTEM) {
                continue;
            }
        }
        let points = interfaces
            .get(FILESYSTEM)
            .map(mount_points)
            .unwrap_or_default();
        let partition = interfaces.get(PARTITION);
        volumes.entry(drive).or_default().push(Volume {
            path: object.to_string(),
            device: block
                .get("PreferredDevice")
                .and_then(|value| bytes(value))
                .filter(|device| !device.is_empty())
                .or_else(|| block.get("Device").and_then(|value| bytes(value)))
                .unwrap_or_default(),
            label: string(block, "IdLabel"),
            filesystem: string(block, "IdType"),
            size: unsigned(block, "Size").unwrap_or(0),
            number: partition
                .and_then(|partition| unsigned(partition, "Number"))
                .map(|number| number as u32),
            system: is_system(&points) || boolean(block, "HintSystem"),
            mountable: interfaces.contains_key(FILESYSTEM),
            mount_points: points,
        });
    }
    let mut disks: Vec<Disk> = objects
        .iter()
        .filter_map(|(object, interfaces)| {
            let drive = interfaces.get(DRIVE)?;
            let object = object.to_string();
            let name = [string(drive, "Vendor"), string(drive, "Model")]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            let media = string(drive, "Media");
            let kind = if media.starts_with("optical") {
                Kind::Optical
            } else if boolean(drive, "Removable") || string(drive, "ConnectionBus") == "usb" {
                Kind::Removable
            } else if drive
                .get("RotationRate")
                .and_then(|value| i32::try_from(value).ok())
                == Some(0)
            {
                Kind::SolidState
            } else {
                Kind::Rotating
            };
            let (health, temperature, hours) = if let Some(ata) = interfaces.get(ATA) {
                let health = if !boolean(ata, "SmartSupported") || !boolean(ata, "SmartEnabled") {
                    Health::Unknown
                } else if boolean(ata, "SmartFailing") {
                    Health::Failing
                } else {
                    Health::Good
                };
                let kelvin = ata
                    .get("SmartTemperature")
                    .and_then(|value| f64::try_from(value).ok())
                    .filter(|kelvin| *kelvin > 0.0);
                (
                    health,
                    kelvin.map(|kelvin| kelvin - 273.15),
                    unsigned(ata, "SmartPowerOnSeconds").map(|seconds| seconds / 3600),
                )
            } else if let Some(nvme) = interfaces.get(NVME) {
                let warnings = match nvme.get("SmartCriticalWarning").map(|value| &**value) {
                    Some(Value::Array(warnings)) => warnings.len(),
                    _ => 0,
                };
                let updated = unsigned(nvme, "SmartUpdated").unwrap_or(0) > 0;
                let health = match (updated, warnings) {
                    (false, _) => Health::Unknown,
                    (true, 0) => Health::Good,
                    _ => Health::Failing,
                };
                (
                    health,
                    unsigned(nvme, "SmartTemperature")
                        .filter(|kelvin| *kelvin > 0)
                        .map(|kelvin| kelvin as f64 - 273.15),
                    unsigned(nvme, "SmartPowerOnHours"),
                )
            } else {
                (Health::Unknown, None, None)
            };
            let (block, table) = whole.get(&object).cloned().unzip();
            let mut volumes = volumes.remove(&object).unwrap_or_default();
            volumes.sort_by_key(|volume| volume.number.unwrap_or(0));
            Some(Disk {
                path: object,
                name,
                size: unsigned(drive, "Size").unwrap_or(0),
                kind,
                ejectable: boolean(drive, "Ejectable"),
                can_power_off: boolean(drive, "CanPowerOff"),
                block,
                table: table.flatten(),
                health,
                temperature,
                power_on_hours: hours,
                volumes,
            })
        })
        .collect();
    // Fixed disks first, then by name.
    disks.sort_by(|a, b| {
        (a.kind == Kind::Removable, &a.name).cmp(&(b.kind == Kind::Removable, &b.name))
    });
    disks
}

/// Filesystems the format dialog offers, with the UDisks type name.
pub const FORMATS: &[(&str, &str)] = &[
    ("exfat", "exFAT"),
    ("vfat", "FAT32"),
    ("ntfs", "NTFS"),
    ("ext4", "ext4"),
];

/// FAT labels are at most 11 characters and exFAT 15; others allow more.
pub fn label_limit(filesystem: &str) -> usize {
    match filesystem {
        "vfat" => 11,
        "exfat" => 15,
        "ntfs" => 32,
        _ => 16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::{OwnedObjectPath, OwnedValue};

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn value(value: Value<'_>) -> Result<OwnedValue> {
        Ok(value.try_to_owned()?)
    }

    fn device(text: &str) -> Result<OwnedValue> {
        let mut raw = text.as_bytes().to_vec();
        raw.push(0);
        value(Value::from(raw))
    }

    fn sample() -> Result<Objects> {
        let mut objects = Objects::new();
        let drive = "/org/freedesktop/UDisks2/drives/Kingston_USB";
        let drive_path = || value(Value::from(ObjectPath::try_from(drive)?));
        let mut drive_props = Properties::new();
        drive_props.insert("Vendor".into(), value(Value::from("Kingston"))?);
        drive_props.insert("Model".into(), value(Value::from("DataTraveler"))?);
        drive_props.insert("Size".into(), value(Value::from(16_000_000_000u64))?);
        drive_props.insert("Removable".into(), value(Value::from(true))?);
        drive_props.insert("CanPowerOff".into(), value(Value::from(true))?);
        objects.insert(
            OwnedObjectPath::try_from(drive)?,
            HashMap::from([(DRIVE.to_owned(), drive_props)]),
        );
        let mut disk = Properties::new();
        disk.insert("Drive".into(), drive_path()?);
        disk.insert("Device".into(), device("/dev/sdb")?);
        objects.insert(
            OwnedObjectPath::try_from("/org/freedesktop/UDisks2/block_devices/sdb")?,
            HashMap::from([
                (BLOCK.to_owned(), disk),
                (TABLE.to_owned(), Properties::new()),
            ]),
        );
        let mut part = Properties::new();
        part.insert("Drive".into(), drive_path()?);
        part.insert("Device".into(), device("/dev/sdb1")?);
        part.insert("IdType".into(), value(Value::from("exfat"))?);
        part.insert("IdLabel".into(), value(Value::from("USB"))?);
        part.insert("Size".into(), value(Value::from(15_000_000_000u64))?);
        let mut number = Properties::new();
        number.insert("Number".into(), value(Value::from(1u32))?);
        let mut point = b"/media/asena/USB".to_vec();
        point.push(0);
        let mut filesystem = Properties::new();
        filesystem.insert("MountPoints".into(), value(Value::from(vec![point]))?);
        objects.insert(
            OwnedObjectPath::try_from("/org/freedesktop/UDisks2/block_devices/sdb1")?,
            HashMap::from([
                (BLOCK.to_owned(), part),
                (PARTITION.to_owned(), number),
                (FILESYSTEM.to_owned(), filesystem),
            ]),
        );
        Ok(objects)
    }

    #[test]
    fn drives_collect_their_partitions() -> Result<()> {
        let disks = disks(&sample()?);
        assert_eq!(disks.len(), 1);
        let disk = &disks[0];
        assert_eq!(disk.name, "Kingston DataTraveler");
        assert_eq!(disk.kind, Kind::Removable);
        assert_eq!(
            disk.table.as_deref(),
            Some("/org/freedesktop/UDisks2/block_devices/sdb")
        );
        assert_eq!(disk.volumes.len(), 1);
        let volume = &disk.volumes[0];
        assert_eq!(volume.device, "/dev/sdb1");
        assert_eq!(volume.mount_points, ["/media/asena/USB"]);
        assert!(volume.mountable);
        assert!(!disk.system());
        Ok(())
    }

    #[test]
    fn the_running_system_is_recognized() {
        assert!(is_system(&["/".into()]));
        assert!(is_system(&["/boot/efi".into()]));
        assert!(!is_system(&["/media/asena/USB".into()]));
    }

    #[test]
    fn label_limits_follow_the_filesystem() {
        assert_eq!(label_limit("vfat"), 11);
        assert_eq!(label_limit("exfat"), 15);
    }
}
