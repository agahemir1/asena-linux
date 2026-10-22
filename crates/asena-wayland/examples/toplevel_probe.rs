//! Run only against a disposable test window with app_id asena-probe.
use asena_wayland::toplevel::{self, Event, Request};
use std::{cell::Cell, rc::Rc, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bridge = toplevel::start()?;
    let loop_ = glib::MainLoop::new(None, false);
    let success = Rc::new(Cell::new(false));
    let result = success.clone();
    let active_loop = loop_.clone();
    glib::spawn_future_local(async move {
        let mut stage = 0;
        let mut target = 0;
        while let Ok(event) = bridge.events.recv().await {
            println!("{event:?}");
            match event {
                Event::Updated(window) if window.app_id == "asena-probe" => {
                    target = window.id;
                    if stage == 0 {
                        let _ = bridge.requests.send(Request::Minimize(target)).await;
                        stage = 1;
                    } else if stage == 1 && window.minimized {
                        let _ = bridge.requests.send(Request::Activate(target)).await;
                        stage = 2;
                    } else if stage == 2 && window.activated && !window.minimized {
                        let _ = bridge.requests.send(Request::Close(target)).await;
                        stage = 3;
                    }
                }
                Event::Closed(id) if id == target && stage == 3 => {
                    result.set(true);
                    active_loop.quit();
                    break;
                }
                _ => {}
            }
        }
    });
    let timeout_loop = loop_.clone();
    glib::timeout_add_local_once(Duration::from_secs(15), move || timeout_loop.quit());
    loop_.run();
    if !success.get() {
        return Err("Toplevel minimize/activate/close probe did not complete".into());
    }
    Ok(())
}
