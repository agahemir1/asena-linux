fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    std::fs::create_dir_all(out.join("ui"))?;
    for name in ["lock", "greeter", "polkit", "settings"] {
        let temporary = out.join(format!("ui/{name}.ui.tmp"));
        let destination = out.join(format!("ui/{name}.ui"));
        let source = format!("../../data/ui/{name}.blp");
        let status = std::process::Command::new("blueprint-compiler")
            .args(["compile", "--output"])
            .arg(&temporary)
            .arg(&source)
            .status()?;
        if !status.success() {
            return Err("Blueprint compilation failed".into());
        }
        let compiled = std::fs::read(&temporary)?;
        if std::fs::read(&destination).ok().as_deref() != Some(compiled.as_slice()) {
            std::fs::write(&destination, compiled)?;
        }
        std::fs::remove_file(temporary)?;
        println!("cargo:rerun-if-changed={source}");
    }
    glib_build_tools::compile_resources(
        &[out.to_str().ok_or("Non-UTF8 build path")?, "../../data"],
        "../../data/asena.gresource.xml",
        "asena.gresource",
    );
    Ok(())
}
