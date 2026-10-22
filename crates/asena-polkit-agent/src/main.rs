use adw::prelude::*;
use asena_core::i18n::gettext;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use zeroize::Zeroizing;
mod content;
mod worker;
struct Runtime {
    view: content::Content,
    window: adw::ApplicationWindow,
    commands: async_channel::Sender<worker::Command>,
    ticket: Cell<Option<u64>>,
    epoch: Cell<u64>,
    echo: Cell<bool>,
    retry: Cell<bool>,
    selecting: Cell<bool>,
    failed: Cell<bool>,
    cancelled: RefCell<Option<Arc<AtomicBool>>>,
}
impl Runtime {
    fn clear(&self) {
        let w = self.view.widgets();
        w.password.set_text("");
        w.answer.set_text("");
    }
    fn message(&self, text: &str) {
        let w = self.view.widgets();
        w.message.set_text(text);
        w.message.set_visible(!text.is_empty());
    }
    fn cancel(&self) {
        self.clear();
        if let Some(flag) = self.cancelled.borrow().as_ref() {
            flag.store(true, Ordering::SeqCst);
        }
        if let Some(ticket) = self.ticket.get() {
            let _ = self.commands.try_send(worker::Command::Cancel(ticket));
        }
        self.window.set_visible(false);
    }
    fn submit(&self) {
        let Some(ticket) = self.ticket.get() else {
            return;
        };
        self.view.widgets().submit.set_sensitive(false);
        if self.retry.replace(false) {
            let _ = self.commands.try_send(worker::Command::Retry(ticket));
            return;
        }
        let text = if self.echo.get() {
            self.view.widgets().answer.text()
        } else {
            self.view.widgets().password.text()
        };
        let answer = Zeroizing::new(text.to_string());
        self.clear();
        let _ = self
            .commands
            .try_send(worker::Command::Answer(ticket, self.epoch.get(), answer));
    }
    fn event(&self, event: worker::Event) {
        let w = self.view.widgets();
        match event {
            worker::Event::Request {
                ticket,
                message,
                identities,
                cancelled,
            } => {
                self.clear();
                self.ticket.set(Some(ticket));
                self.cancelled.replace(Some(cancelled));
                self.retry.set(false);
                self.selecting.set(true);
                let names: Vec<&str> = identities.iter().map(String::as_str).collect();
                w.identities.set_model(Some(&gtk::StringList::new(&names)));
                w.identities.set_selected(0);
                w.identities.set_visible(identities.len() > 1);
                self.selecting.set(false);
                w.description.set_text(&message);
                w.prompt.set_text(&gettext("Password"));
                w.password.set_visible(true);
                w.answer.set_visible(false);
                self.message("");
                w.cancel.set_label(&gettext("Cancel"));
                w.submit.set_label(&gettext("Authenticate"));
                w.submit.set_visible(true);
                w.submit.set_sensitive(false);
                self.window.present();
            }
            worker::Event::Prompt {
                ticket,
                epoch,
                text,
                echo,
            } if self.ticket.get() == Some(ticket) => {
                if self
                    .cancelled
                    .borrow()
                    .as_ref()
                    .is_some_and(|flag| flag.load(Ordering::SeqCst))
                {
                    return;
                }
                self.clear();
                self.epoch.set(epoch);
                self.echo.set(echo);
                self.retry.set(false);
                w.prompt.set_text(&if matches!(
                    text.as_str(),
                    "Password:" | "Password: " | "Password"
                ) {
                    gettext("Password")
                } else {
                    text
                });
                w.password.set_visible(!echo);
                w.answer.set_visible(echo);
                w.password.set_sensitive(true);
                w.answer.set_sensitive(true);
                w.submit.set_sensitive(true);
                w.submit.set_label(&gettext("Authenticate"));
                if echo {
                    w.answer.grab_focus();
                } else {
                    w.password.grab_focus();
                }
            }
            worker::Event::Message(ticket, text) if self.ticket.get() == Some(ticket) => {
                self.message(&text)
            }
            worker::Event::Retry(ticket) if self.ticket.get() == Some(ticket) => {
                self.clear();
                self.retry.set(true);
                self.message(&gettext("Authentication failed. Try again."));
                w.password.set_visible(false);
                w.answer.set_visible(false);
                w.submit.set_label(&gettext("Try again"));
                w.submit.set_sensitive(true);
                w.submit.grab_focus();
            }
            worker::Event::Finished(ticket, gained, cancelled)
                if self.ticket.get() == Some(ticket) =>
            {
                self.clear();
                self.ticket.set(None);
                self.cancelled.borrow_mut().take();
                if gained || cancelled {
                    self.window.set_visible(false);
                } else {
                    self.message(&gettext("Authentication failed. Try again."));
                    w.password.set_visible(false);
                    w.answer.set_visible(false);
                    w.submit.set_visible(false);
                    w.cancel.set_label(&gettext("Close"));
                }
            }
            worker::Event::Unavailable(error) => {
                tracing::error!(%error,"Authorization agent registration failed");
                self.failed.set(true);
                self.clear();
                self.message(&gettext("The authorization agent could not register."));
                w.description.set_text("");
                w.identities.set_visible(false);
                w.password.set_visible(false);
                w.answer.set_visible(false);
                w.submit.set_visible(false);
                w.cancel.set_label(&gettext("Close"));
                self.window.present();
            }
            _ => {}
        }
    }
}
fn activate(app: &adw::Application, ready: Rc<Cell<bool>>, started: &Cell<bool>) {
    if started.replace(true) {
        return;
    }
    if let Err(error) = asena_widgets::init() {
        tracing::error!(%error,"Authorization resources unavailable");
        app.quit();
        return;
    }
    let (events, receiver) = async_channel::unbounded();
    let commands = match worker::start(events) {
        Ok(commands) => commands,
        Err(error) => {
            tracing::error!(%error,"Authorization worker unavailable");
            app.quit();
            return;
        }
    };
    let view = content::Content::default();
    let w = view.widgets();
    w.cancel.set_label(&gettext("Cancel"));
    w.submit.set_label(&gettext("Authenticate"));
    if let Some(delegate) = w.password.delegate().and_downcast::<gtk::Text>() {
        delegate.set_max_length(4096);
    }
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(gettext("Authentication required"))
        .content(&view)
        .default_width(440)
        .default_height(360)
        .build();
    window.set_default_widget(Some(&*w.submit));
    let runtime = Rc::new(Runtime {
        view: view.clone(),
        window: window.clone(),
        commands: commands.clone(),
        ticket: Cell::new(None),
        epoch: Cell::new(0),
        echo: Cell::new(false),
        retry: Cell::new(false),
        selecting: Cell::new(false),
        failed: Cell::new(false),
        cancelled: RefCell::new(None),
    });
    let weak = Rc::downgrade(&runtime);
    w.submit.connect_clicked(move |_| {
        if let Some(runtime) = weak.upgrade() {
            runtime.submit();
        }
    });
    let weak = Rc::downgrade(&runtime);
    w.cancel.connect_clicked(move |_| {
        if let Some(runtime) = weak.upgrade() {
            if runtime.failed.get() {
                if let Some(app) = runtime.window.application() {
                    app.quit();
                }
            } else {
                runtime.cancel();
            }
        }
    });
    let weak = Rc::downgrade(&runtime);
    w.identities.connect_selected_notify(move |dropdown| {
        if let Some(runtime) = weak.upgrade() {
            if runtime.selecting.get() {
                return;
            }
            if let Some(ticket) = runtime.ticket.get() {
                runtime.clear();
                runtime.view.widgets().submit.set_sensitive(false);
                let _ = runtime.commands.try_send(worker::Command::Select(
                    ticket,
                    dropdown.selected() as usize,
                ));
            }
        }
    });
    let weak = Rc::downgrade(&runtime);
    window.connect_close_request(move |_| {
        if let Some(runtime) = weak.upgrade() {
            if runtime.failed.get() {
                if let Some(app) = runtime.window.application() {
                    app.quit();
                }
            } else {
                runtime.cancel();
            }
        }
        glib::Propagation::Stop
    });
    let key = gtk::EventControllerKey::new();
    let weak = Rc::downgrade(&runtime);
    key.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            if let Some(runtime) = weak.upgrade() {
                runtime.cancel();
            }
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(key);
    let hold = app.hold();
    let stopped = app.clone();
    let ui = runtime.clone();
    glib::spawn_future_local(async move {
        while let Ok(event) = receiver.recv().await {
            match event {
                worker::Event::Ready => {
                    ready.set(true);
                    tracing::info!("Authorization agent registered");
                }
                worker::Event::Stopped => {
                    stopped.quit();
                    break;
                }
                event => ui.event(event),
            }
        }
    });
    for signal in [2, 15] {
        let commands = commands.clone();
        let fallback = app.clone();
        glib::unix_signal_add_local(signal, move || {
            let _ = commands.try_send(worker::Command::Stop);
            let fallback = fallback.clone();
            glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                fallback.quit()
            });
            glib::ControlFlow::Continue
        });
    }
    app.connect_shutdown(move |_| {
        runtime.cancel();
        let _ = commands.try_send(worker::Command::Stop);
        let _ = &hold;
    });
}
fn main() -> anyhow::Result<()> {
    asena_core::logging::init("asena-polkit-agent").map_err(|error| anyhow::anyhow!("{error}"))?;
    asena_core::i18n::init().map_err(|error| anyhow::anyhow!("{error}"))?;
    anyhow::ensure!(
        std::env::var_os("ASENA_DEV").is_none()
            || std::env::var("ASENA_POLKIT_TEST_VM").as_deref() == Ok("1"),
        gettext("Authorization is unavailable in this development session.")
    );
    let app = adw::Application::builder()
        .application_id("org.asenalinux.PolkitAgent")
        .build();
    let ready = Rc::new(Cell::new(false));
    let result = ready.clone();
    let started = Cell::new(false);
    app.connect_activate(move |app| activate(app, ready.clone(), &started));
    app.run();
    anyhow::ensure!(
        result.get(),
        gettext("The authorization agent could not register.")
    );
    Ok(())
}
