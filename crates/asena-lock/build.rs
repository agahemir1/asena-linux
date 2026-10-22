fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    let status = std::process::Command::new("pkg-config")
        .args(["--atleast-version=1.3.0", "gtk4-layer-shell-0"])
        .status()?;
    if !status.success() {
        return Err("Asena Lock requires the approved gtk4-layer-shell C 1.3.0 backport. Run scripts/backport-layer-shell.sh, then source scripts/native-env.sh. Debian 1.0.4 cannot provide session lock.".into());
    }
    let output = std::process::Command::new("pkg-config")
        .args(["--variable=libdir", "gtk4-layer-shell-0"])
        .output()?;
    if !output.status.success() {
        return Err("Could not locate the native lock library".into());
    }
    let directory = std::str::from_utf8(&output.stdout)?.trim();
    if directory.is_empty() {
        return Err("Native lock library directory is empty".into());
    }
    // Put the approved prefix ahead of transitive GTK /usr/lib search paths.
    println!("cargo:rustc-link-search=native={directory}");
    println!("cargo:rustc-link-lib=gtk4-layer-shell");
    Ok(())
}
