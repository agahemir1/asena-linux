#[cfg(not(target_os = "linux"))]
compile_error!("Asena must be built on Linux");

use std::{os::unix::process::CommandExt, process::Command};

fn main() -> anyhow::Result<()> {
    // Cargo owns the executable; the embedded POSIX script owns session startup.
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(include_str!("../../../scripts/asena-session"))
        .arg("asena-session")
        .args(std::env::args_os().skip(1));
    if std::env::args_os().any(|arg| arg == "--dev") {
        command.env(
            "ASENA_SOURCE_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../.."),
        );
    }
    Err(command.exec().into())
}
