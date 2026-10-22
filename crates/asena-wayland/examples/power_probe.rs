//! Run only inside the disposable locked VM session. Always attempt restoration.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("ASENA_LOCK_TEST_VM").as_deref() != Ok("1") {
        return Err("VM fixture flag required".into());
    }
    let outcome = std::thread::Builder::new()
        .name("asena-power-probe".into())
        .spawn(|| {
            let mut session = asena_wayland::power::Session::connect()?;
            let off = session.set_power(false);
            let on = session.set_power(true);
            off.and(on)
        })?
        .join()
        .map_err(|_| "Power worker panicked")?;
    outcome?;
    println!("Output power off/on acknowledgements passed.");
    Ok(())
}
