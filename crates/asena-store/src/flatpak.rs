//! Flatpak through libflatpak (K22). Applications go to the user installation,
//! which needs no administrator; the system installation is read for the
//! installed and update lists. Every function blocks and creates its own
//! installation objects, so callers run them with `gio::spawn_blocking`.
use libflatpak::prelude::*;

/// Flathub's remote description with its signing key (fingerprint
/// 6E5C 05D9 79C7 6DAF 93C0 8135 4184 DD4D 907A 7CAE), added only on request.
const FLATHUB: &[u8] = include_bytes!("../../../data/flatpak/flathub.flatpakrepo");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ref {
    pub reference: String,
    pub name: String,
    pub user: bool,
}

/// What a Flatpak application may reach outside its sandbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    Network,
    AllFiles,
    HomeFiles,
    Devices,
    Audio,
    /// X11 without Wayland: the application can watch other windows.
    LegacyDisplay,
    SessionBus,
    SystemBus,
}

fn installations() -> Vec<libflatpak::Installation> {
    let mut list = Vec::new();
    if let Ok(user) = libflatpak::Installation::new_user(gio::Cancellable::NONE) {
        list.push(user);
    }
    if let Ok(system) = libflatpak::Installation::new_system(gio::Cancellable::NONE) {
        list.push(system);
    }
    list
}

fn user() -> Result<libflatpak::Installation, glib::Error> {
    libflatpak::Installation::new_user(gio::Cancellable::NONE)
}

pub fn installed() -> Vec<Ref> {
    installations()
        .iter()
        .flat_map(|installation| {
            let user = installation.is_user();
            installation
                .list_installed_refs_by_kind(libflatpak::RefKind::App, gio::Cancellable::NONE)
                .unwrap_or_default()
                .into_iter()
                .filter_map(move |installed| {
                    Some(Ref {
                        reference: installed.format_ref()?.to_string(),
                        name: installed.name()?.to_string(),
                        user,
                    })
                })
        })
        .collect()
}

pub fn updates() -> Vec<Ref> {
    installations()
        .iter()
        .flat_map(|installation| {
            let user = installation.is_user();
            installation
                .list_installed_refs_for_update(gio::Cancellable::NONE)
                .unwrap_or_default()
                .into_iter()
                // Runtimes carry their own security fixes; locale and debug
                // extensions follow the ref they belong to.
                .filter(|installed| {
                    installed.kind() == libflatpak::RefKind::App
                        || installed.name().is_some_and(|name| {
                            !name.ends_with(".Locale") && !name.ends_with(".Debug")
                        })
                })
                .filter_map(move |installed| {
                    Some(Ref {
                        reference: installed.format_ref()?.to_string(),
                        name: installed
                            .appdata_name()
                            .or_else(|| installed.name())?
                            .to_string(),
                        user,
                    })
                })
        })
        .collect()
}

/// The AppStream catalog of one remote, as Flatpak keeps it on disk.
pub struct Catalog {
    pub remote: String,
    pub user: bool,
    pub directory: std::path::PathBuf,
}

pub fn catalogs() -> Vec<Catalog> {
    installations()
        .iter()
        .flat_map(|installation| {
            let user = installation.is_user();
            installation
                .list_remotes(gio::Cancellable::NONE)
                .unwrap_or_default()
                .into_iter()
                .filter(|remote| !remote.is_disabled())
                .filter_map(move |remote| {
                    Some(Catalog {
                        remote: remote.name()?.to_string(),
                        user,
                        directory: remote.appstream_dir(None)?.path()?,
                    })
                })
        })
        .collect()
}

fn installation(user_installation: bool) -> Result<libflatpak::Installation, glib::Error> {
    if user_installation {
        user()
    } else {
        libflatpak::Installation::new_system(gio::Cancellable::NONE)
    }
}

pub fn has_flathub() -> bool {
    installations().iter().any(|installation| {
        installation
            .remote_by_name("flathub", gio::Cancellable::NONE)
            .is_ok()
    })
}

/// Adds Flathub to the user installation and downloads its catalog.
pub fn enable_flathub() -> Result<(), glib::Error> {
    let installation = user()?;
    let remote = libflatpak::Remote::from_file("flathub", &glib::Bytes::from_static(FLATHUB))?;
    installation.add_remote(&remote, true, gio::Cancellable::NONE)?;
    installation.update_appstream_sync("flathub", None, gio::Cancellable::NONE)?;
    Ok(())
}

/// Refreshes the catalogs of every configured remote.
pub fn refresh_catalogs() {
    for installation in installations() {
        let remotes = installation
            .list_remotes(gio::Cancellable::NONE)
            .unwrap_or_default();
        for remote in remotes {
            if remote.is_disabled() {
                continue;
            }
            if let Some(name) = remote.name() {
                let _ = installation.update_appstream_sync(&name, None, gio::Cancellable::NONE);
            }
        }
    }
}

fn parse_ref(reference: &str) -> Option<(libflatpak::RefKind, &str, &str, &str)> {
    let mut parts = reference.split('/');
    let kind = match parts.next()? {
        "app" => libflatpak::RefKind::App,
        "runtime" => libflatpak::RefKind::Runtime,
        _ => return None,
    };
    Some((kind, parts.next()?, parts.next()?, parts.next()?))
}

/// The permissions an application asks for, read from its metadata on the remote.
pub fn permissions(
    remote: &str,
    reference: &str,
    user_installation: bool,
) -> Result<Vec<Permission>, glib::Error> {
    let (kind, name, arch, branch) = parse_ref(reference).ok_or_else(|| {
        glib::Error::new(
            gio::IOErrorEnum::InvalidArgument,
            "invalid Flatpak reference",
        )
    })?;
    let installation = installation(user_installation)?;
    let remote_ref = installation.fetch_remote_ref_sync(
        remote,
        kind,
        name,
        Some(arch),
        Some(branch),
        gio::Cancellable::NONE,
    )?;
    let metadata = remote_ref
        .metadata()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    Ok(summarize(&metadata))
}

/// Runs one transaction; `progress` receives whole percents.
fn run(
    installation: &libflatpak::Installation,
    add: impl FnOnce(&libflatpak::Transaction) -> Result<(), glib::Error>,
    progress: &async_channel::Sender<i32>,
    cancellable: &gio::Cancellable,
) -> Result<(), glib::Error> {
    let transaction = libflatpak::Transaction::for_installation(installation, Some(cancellable))?;
    transaction.set_no_interaction(true);
    add(&transaction)?;
    let progress = progress.clone();
    transaction.connect_new_operation(move |_, _, operation_progress| {
        let progress = progress.clone();
        operation_progress.connect_changed(move |operation_progress| {
            let _ = progress.try_send(operation_progress.progress());
        });
    });
    transaction.run(Some(cancellable))
}

/// Installs into the installation that has the remote configured; the
/// system installation asks polkit through Flatpak's own helper.
pub fn install(
    remote: &str,
    reference: &str,
    user_installation: bool,
    progress: &async_channel::Sender<i32>,
    cancellable: &gio::Cancellable,
) -> Result<(), glib::Error> {
    run(
        &installation(user_installation)?,
        |transaction| transaction.add_install(remote, reference, &[]),
        progress,
        cancellable,
    )
}

/// Installs the application a `.flatpakref` file describes, for the user.
pub fn install_ref_file(
    contents: &glib::Bytes,
    progress: &async_channel::Sender<i32>,
    cancellable: &gio::Cancellable,
) -> Result<(), glib::Error> {
    run(
        &user()?,
        |transaction| transaction.add_install_flatpakref(contents),
        progress,
        cancellable,
    )
}

/// `Title` (or `Name`) and `Url` of a `.flatpakref` file.
pub fn describe_ref_file(contents: &[u8]) -> Option<(String, String)> {
    let file = glib::KeyFile::new();
    file.load_from_bytes(&glib::Bytes::from(contents), glib::KeyFileFlags::NONE)
        .ok()?;
    let group = "Flatpak Ref";
    let title = file
        .locale_string(group, "Title", None)
        .or_else(|_| file.string(group, "Name"))
        .ok()?
        .to_string();
    let url = file
        .string(group, "Url")
        .map(|url| url.to_string())
        .unwrap_or_default();
    Some((title, url))
}

pub fn uninstall(
    reference: &str,
    user_installation: bool,
    progress: &async_channel::Sender<i32>,
    cancellable: &gio::Cancellable,
) -> Result<(), glib::Error> {
    let installation = installation(user_installation)?;
    run(
        &installation,
        |transaction| transaction.add_uninstall(reference),
        progress,
        cancellable,
    )
}

pub fn update(
    refs: &[Ref],
    progress: &async_channel::Sender<i32>,
    cancellable: &gio::Cancellable,
) -> Result<(), glib::Error> {
    for user_installation in [true, false] {
        let selected: Vec<&Ref> = refs
            .iter()
            .filter(|r| r.user == user_installation)
            .collect();
        if selected.is_empty() {
            continue;
        }
        run(
            &installation(user_installation)?,
            |transaction| {
                for item in &selected {
                    transaction.add_update(&item.reference, &[], None)?;
                }
                Ok(())
            },
            progress,
            cancellable,
        )?;
    }
    Ok(())
}

/// Reads the `[Context]` group of Flatpak metadata into the permissions
/// Software shows before an installation.
pub fn summarize(metadata: &str) -> Vec<Permission> {
    let mut in_context = false;
    let mut values = std::collections::HashMap::<&str, Vec<&str>>::new();
    let mut bus_groups = (false, false);
    for line in metadata.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_context = line == "[Context]";
            bus_groups.0 |= line == "[Session Bus Policy]";
            bus_groups.1 |= line == "[System Bus Policy]";
            continue;
        }
        if !in_context {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.insert(
                key.trim(),
                value
                    .split(';')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .collect(),
            );
        }
    }
    let has = |key: &str, item: &str| values.get(key).is_some_and(|items| items.contains(&item));
    let mut permissions = Vec::new();
    if has("shared", "network") {
        permissions.push(Permission::Network);
    }
    let filesystems = values.get("filesystems").cloned().unwrap_or_default();
    let plain = |item: &&str| item.split(':').next().unwrap_or(item).to_owned();
    if filesystems
        .iter()
        .map(plain)
        .any(|item| item == "host" || item == "host-os")
    {
        permissions.push(Permission::AllFiles);
    } else if filesystems
        .iter()
        .map(plain)
        .any(|item| item == "home" || item == "~")
    {
        permissions.push(Permission::HomeFiles);
    }
    if has("devices", "all") {
        permissions.push(Permission::Devices);
    }
    if has("sockets", "pulseaudio") {
        permissions.push(Permission::Audio);
    }
    if has("sockets", "x11") && !has("sockets", "wayland") {
        permissions.push(Permission::LegacyDisplay);
    }
    if has("sockets", "session-bus") || bus_groups.0 {
        permissions.push(Permission::SessionBus);
    }
    if has("sockets", "system-bus") || bus_groups.1 {
        permissions.push(Permission::SystemBus);
    }
    permissions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_come_from_the_context_group() {
        let metadata = "[Application]\nname=org.example.App\n\n[Context]\nshared=network;ipc;\n\
                        sockets=x11;pulseaudio;\ndevices=dri;\nfilesystems=xdg-download;home:ro;\n\n\
                        [Session Bus Policy]\norg.freedesktop.Notifications=talk\n";
        assert_eq!(
            summarize(metadata),
            [
                Permission::Network,
                Permission::HomeFiles,
                Permission::Audio,
                Permission::LegacyDisplay,
                Permission::SessionBus,
            ]
        );
    }

    #[test]
    fn host_access_and_wayland_apps() {
        let metadata =
            "[Context]\nsockets=wayland;fallback-x11;x11;\nfilesystems=host;\ndevices=all;\n";
        assert_eq!(
            summarize(metadata),
            [Permission::AllFiles, Permission::Devices]
        );
        assert!(summarize("[Application]\nname=x\n").is_empty());
    }

    #[test]
    fn references_split_into_their_parts() {
        let (kind, name, arch, branch) = parse_ref("app/org.gnome.Loupe/x86_64/stable")
            .unwrap_or((libflatpak::RefKind::Runtime, "", "", ""));
        assert_eq!(kind, libflatpak::RefKind::App);
        assert_eq!(
            (name, arch, branch),
            ("org.gnome.Loupe", "x86_64", "stable")
        );
        assert!(parse_ref("org.gnome.Loupe").is_none());
    }
}
