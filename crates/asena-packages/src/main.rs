//! asena-packages: the privileged package service of Asena Software (K21,
//! Phase 6F). It runs as root on the system bus, started by D-Bus activation,
//! and installs, removes and upgrades Debian packages through libapt-pkg for
//! callers that polkit authorized. With `--offline-update` it applies the
//! downloaded upgrades during system-update.target and restarts.
mod apt;
mod offline;
mod plans;
mod polkit;
mod service;

fn main() -> anyhow::Result<()> {
    asena_core::logging::init("asena-packages").map_err(|e| anyhow::anyhow!("{e}"))?;
    asena_core::i18n::init().map_err(|e| anyhow::anyhow!("{e}"))?;
    // Package maintainer scripts must never wait for an answer.
    std::env::set_var("DEBIAN_FRONTEND", "noninteractive");
    match std::env::args().nth(1).as_deref() {
        None => service::run(),
        Some("--offline-update") => offline::run(),
        Some(other) => anyhow::bail!("Unknown argument: {other}"),
    }
}
