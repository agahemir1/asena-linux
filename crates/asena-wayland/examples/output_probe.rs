//! Prints the output heads. With `NAME SCALE [test]`, tests or applies a scale.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let bridge = asena_wayland::output::start()?;
    async_io::block_on(async {
        let heads = loop {
            match bridge.events.recv().await? {
                asena_wayland::output::Event::Available(false) => {
                    return Err("Output management is unavailable".into())
                }
                asena_wayland::output::Event::Available(true) => {}
                asena_wayland::output::Event::Heads(heads) => break heads,
            }
        };
        for head in &heads {
            println!(
                "{} enabled={} mode={:?} position={},{} transform={} scale={} modes={} make={:?} model={:?} serial={:?}",
                head.name,
                head.enabled,
                head.mode.map(|mode| (mode.width, mode.height, mode.refresh)),
                head.x,
                head.y,
                head.transform,
                head.scale,
                head.modes.len(),
                head.make,
                head.model,
                head.serial
            );
        }
        let [name, scale, rest @ ..] = arguments.as_slice() else {
            return Ok::<(), Box<dyn std::error::Error>>(());
        };
        let head = heads
            .iter()
            .find(|head| head.name == *name)
            .ok_or("Unknown output")?;
        let (reply, outcome) = async_channel::bounded(1);
        bridge
            .requests
            .send(asena_wayland::output::Request {
                heads: vec![asena_wayland::output::HeadConfig {
                    name: head.name.clone(),
                    enabled: true,
                    mode: head.mode,
                    x: head.x,
                    y: head.y,
                    transform: head.transform,
                    scale: scale.parse()?,
                }],
                test: rest.first().is_some_and(|word| word == "test"),
                reply,
            })
            .await?;
        println!("outcome={:?}", outcome.recv().await?);
        Ok(())
    })
}
