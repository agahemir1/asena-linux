//! Stable identities; method contracts are added with their implementing phase.
pub const SHELL_ID: &str = "org.asenalinux.Shell";
pub const SHELL_PATH: &str = "/org/asenalinux/Shell";
pub const DAEMON_ID: &str = "org.asenalinux.Daemon";
pub const DAEMON_PATH: &str = "/org/asenalinux/Daemon";
pub const LOCK_ID: &str = "org.asenalinux.Lock";
pub const LOCK_PATH: &str = "/org/asenalinux/Lock";
pub const SETTINGS_ID: &str = "org.asenalinux.Settings";
pub const SETTINGS_PATH: &str = "/org/asenalinux/Settings";
#[zbus::proxy(
    interface = "org.asenalinux.Settings",
    default_service = "org.asenalinux.Settings",
    default_path = "/org/asenalinux/Settings"
)]
pub trait Settings {
    fn open_panel(&self, id: &str) -> zbus::Result<()>;
}
#[zbus::proxy(
    interface = "org.asenalinux.Daemon",
    default_service = "org.asenalinux.Daemon",
    default_path = "/org/asenalinux/Daemon"
)]
pub trait Daemon {
    fn idle(&self, scope: &str, action: &str) -> zbus::Result<()>;
    fn lock(&self, scope: &str) -> zbus::Result<()>;
    #[zbus(property)]
    fn lock_available(&self) -> zbus::Result<bool>;
    /// JSON `displays::Layout`.
    fn display_layout(&self) -> zbus::Result<String>;
    /// JSON list of `displays::Config`; returns the seconds left to confirm.
    fn apply_displays(&self, scope: &str, layout: &str) -> zbus::Result<u32>;
    fn confirm_displays(&self, scope: &str) -> zbus::Result<()>;
    fn revert_displays(&self, scope: &str) -> zbus::Result<()>;
    #[zbus(signal)]
    fn displays_changed(&self) -> zbus::Result<()>;
}
#[zbus::proxy(
    interface = "org.asenalinux.Lock",
    default_service = "org.asenalinux.Lock",
    default_path = "/org/asenalinux/Lock"
)]
pub trait Lock {
    fn set_sleep_pending(&self, scope: &str, pending: bool) -> zbus::Result<()>;
    fn wait_ready(&self, scope: &str) -> zbus::Result<()>;
    fn focus(&self) -> zbus::Result<()>;
    #[zbus(property)]
    fn ready(&self) -> zbus::Result<bool>;
}

#[zbus::proxy(
    interface = "org.asenalinux.Shell",
    default_service = "org.asenalinux.Shell",
    default_path = "/org/asenalinux/Shell"
)]
pub trait Shell {
    fn toggle_menu(&self) -> zbus::Result<()>;
    fn toggle_quick_settings(&self) -> zbus::Result<()>;
    fn toggle_notification_center(&self) -> zbus::Result<()>;
    fn show_osd(&self, kind: &str, value: f64) -> zbus::Result<()>;
    fn reload_style(&self) -> zbus::Result<()>;
    fn change_volume(&self, action: &str) -> zbus::Result<()>;
    fn change_brightness(&self, action: &str) -> zbus::Result<()>;
    fn toggle_microphone(&self) -> zbus::Result<()>;
}
/// Claim a service name without replacing or queueing behind another owner.
pub async fn claim_name(connection: &zbus::Connection, name: &str) -> zbus::Result<()> {
    let reply = connection
        .request_name_with_flags(name, zbus::fdo::RequestNameFlags::DoNotQueue.into())
        .await?;
    if matches!(
        reply,
        zbus::fdo::RequestNameReply::PrimaryOwner | zbus::fdo::RequestNameReply::AlreadyOwner
    ) {
        Ok(())
    } else {
        Err(zbus::Error::Failure(format!(
            "The bus name {name} already has an owner"
        )))
    }
}
