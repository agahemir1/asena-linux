use adw::prelude::*;
use asena_core::i18n::gettext;
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
};
use zeroize::Zeroizing;
mod accounts;
mod content;
mod ipc;
mod remember;
mod sessions;
#[derive(Clone, Copy)]
enum Input {
    None,
    Secret,
    Visible,
    Information,
}
struct Runtime {
    view: content::Content,
    window: adw::ApplicationWindow,
    socket: PathBuf,
    sessions: RefCell<Vec<sessions::Session>>,
    attempt: RefCell<Option<ipc::Attempt>>,
    input: Cell<Input>,
    cancelling: Cell<bool>,
    starting: Cell<bool>,
    keyboard: RefCell<Option<(gtk::gdk::Device, Vec<glib::SignalHandlerId>)>>,
}
impl Runtime {
    fn clear(&self) {
        let w = self.view.widgets();
        w.password.set_text("");
        w.answer.set_text("");
    }
    fn controls(&self, busy: bool) {
        let w = self.view.widgets();
        w.users.set_sensitive(!busy);
        w.username.set_sensitive(!busy);
        w.sessions.set_sensitive(!busy);
        w.power.set_sensitive(!busy);
        w.cancel.set_sensitive(!self.starting.get());
        w.submit
            .set_sensitive(!busy && !self.sessions.borrow().is_empty());
    }
    fn cancel(&self) {
        if self.starting.get() {
            return;
        }
        self.clear();
        self.input.set(Input::None);
        if let Some(attempt) = self.attempt.borrow().as_ref() {
            self.cancelling.set(true);
            attempt.cancel();
            self.controls(true);
            self.view
                .widgets()
                .message
                .set_text(&gettext("Authentication cancelled."));
        } else {
            self.view.widgets().username.grab_focus();
        }
    }
    fn submit(self: &Rc<Self>) {
        let w = self.view.widgets();
        if self.cancelling.get() || self.starting.get() {
            return;
        }
        if let Some(attempt) = self.attempt.borrow().as_ref() {
            let answer = match self.input.replace(Input::None) {
                Input::None => return,
                Input::Secret => ipc::Answer::Text(Zeroizing::new(w.password.text().to_string())),
                Input::Visible => ipc::Answer::Text(Zeroizing::new(w.answer.text().to_string())),
                Input::Information => ipc::Answer::Continue,
            };
            self.clear();
            w.submit.set_sensitive(false);
            w.password.set_sensitive(false);
            w.answer.set_sensitive(false);
            if attempt.answers.try_send(answer).is_err() {
                w.message
                    .set_text(&gettext("The login service is unavailable."));
            }
            return;
        }
        let username = w.username.text().trim().to_owned();
        if !remember::valid(&username) {
            w.message.set_text(&gettext("Enter a user name."));
            return;
        }
        let Some(session) = self
            .sessions
            .borrow()
            .get(w.sessions.selected() as usize)
            .cloned()
        else {
            return;
        };
        self.controls(true);
        self.clear();
        w.message.set_text("");
        w.prompt.set_text("");
        self.cancelling.set(false);
        let (sender, receiver) = async_channel::bounded(4);
        match ipc::start(self.socket.clone(), username, session, sender) {
            Ok(attempt) => {
                self.attempt.replace(Some(attempt));
                let weak = Rc::downgrade(self);
                glib::spawn_future_local(async move {
                    while let Ok(event) = receiver.recv().await {
                        let Some(runtime) = weak.upgrade() else {
                            break;
                        };
                        let done = matches!(event, ipc::Event::Done);
                        runtime.event(event).await;
                        if done {
                            break;
                        }
                    }
                });
            }
            Err(error) => {
                tracing::warn!(%error,"Greeter worker could not start");
                w.message
                    .set_text(&gettext("The login service is unavailable."));
                self.controls(false);
            }
        }
    }
    async fn event(&self, event: ipc::Event) {
        let w = self.view.widgets();
        if self.cancelling.get() && !matches!(event, ipc::Event::Done | ipc::Event::Started(_)) {
            return;
        }
        match event {
            ipc::Event::Prompt(text, secret) => {
                let text = if matches!(text.as_str(), "Password" | "Password:" | "Password: ") {
                    gettext("Password")
                } else {
                    text
                };
                self.clear();
                w.prompt.set_text(&text);
                w.password.set_visible(secret);
                w.answer.set_visible(!secret);
                w.password.set_sensitive(true);
                w.answer.set_sensitive(true);
                self.input.set(if secret {
                    Input::Secret
                } else {
                    Input::Visible
                });
                w.submit.set_label(&gettext("Continue"));
                w.submit.set_sensitive(true);
                if secret {
                    w.password.grab_focus();
                } else {
                    w.answer.grab_focus();
                }
                tracing::info!("Greeter authentication prompt ready");
            }
            ipc::Event::Information(text) => {
                self.clear();
                w.password.set_visible(false);
                w.answer.set_visible(false);
                w.prompt.set_text(&text);
                self.input.set(Input::Information);
                w.submit.set_label(&gettext("Continue"));
                w.submit.set_sensitive(true);
                w.submit.grab_focus();
            }
            ipc::Event::Authenticated => {
                self.starting.set(true);
                self.controls(true);
                self.clear();
                if let Some(attempt) = self.attempt.borrow().as_ref() {
                    let _ = attempt.answers.try_send(ipc::Answer::Start);
                }
            }
            ipc::Event::Failed => {
                w.message
                    .set_text(&gettext("Authentication failed. Try again."));
            }
            ipc::Event::Disconnected => {
                w.message
                    .set_text(&gettext("The login service is unavailable."));
            }
            ipc::Event::Started(username) => {
                // greetd starts the selected session only after this process exits.
                match futures_lite::future::race(
                    async {
                        gio::spawn_blocking(move || remember::write(&username))
                            .await
                            .map_err(|error| format!("{error:?}"))
                            .and_then(|result| result.map_err(|error| error.to_string()))
                    },
                    async {
                        glib::timeout_future(std::time::Duration::from_secs(2)).await;
                        Err("Last-user persistence timed out".into())
                    },
                )
                .await
                {
                    Ok(()) => {}
                    error => tracing::warn!(?error, "Could not save the last username"),
                }
                if let Some(application) = self.window.application() {
                    application.quit();
                }
            }
            ipc::Event::Done => {
                self.attempt.borrow_mut().take();
                self.starting.set(false);
                self.cancelling.set(false);
                self.input.set(Input::None);
                self.clear();
                w.password.set_visible(false);
                w.answer.set_visible(false);
                w.prompt.set_text("");
                w.submit.set_label(&gettext("Sign in"));
                self.controls(false);
                w.username.grab_focus();
            }
        }
    }
    fn keyboard(&self) {
        let Some(device) = gtk::gdk::Display::default()
            .and_then(|display| display.default_seat())
            .and_then(|seat| seat.keyboard())
        else {
            return;
        };
        let view = self.view.downgrade();
        let refresh = move |device: &gtk::gdk::Device| {
            if let Some(view) = view.upgrade() {
                let w = view.widgets();
                let names = device.layout_names();
                let index = usize::try_from(device.active_layout_index()).ok();
                let name = index.and_then(|index| names.get(index));
                w.keyboard.set_visible(name.is_some());
                w.keyboard
                    .set_text(name.map(|name| name.as_str()).unwrap_or_default());
                w.caps.set_text(&gettext("Caps Lock is on"));
                w.caps.set_visible(device.is_caps_locked());
            }
        };
        refresh(&device);
        let active = refresh.clone();
        let layouts = refresh.clone();
        let handlers = vec![
            device.connect_active_layout_index_notify(active),
            device.connect_layout_names_notify(layouts),
            device.connect_caps_lock_state_notify(refresh),
        ];
        self.keyboard.replace(Some((device, handlers)));
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.clear();
        if let Some(attempt) = self.attempt.get_mut().take() {
            attempt.cancel();
        }
        if let Some((device, handlers)) = self.keyboard.get_mut().take() {
            for handler in handlers {
                device.disconnect(handler);
            }
        }
    }
}
fn power(runtime: &Rc<Runtime>) {
    for (label, capability, method) in [
        (gettext("Restart"), "CanReboot", "Reboot"),
        (gettext("Power off"), "CanPowerOff", "PowerOff"),
    ] {
        let button = gtk::Button::with_label(&label);
        button.set_visible(false);
        runtime.view.widgets().power.append(&button);
        let weak = button.downgrade();
        glib::spawn_future_local(async move {
            if let Ok(connection) = zbus::Connection::system().await {
                if let Ok(proxy) = zbus::Proxy::new(
                    &connection,
                    "org.freedesktop.login1",
                    "/org/freedesktop/login1",
                    "org.freedesktop.login1.Manager",
                )
                .await
                {
                    if proxy
                        .call::<_, _, String>(capability, &())
                        .await
                        .is_ok_and(|answer| answer == "yes")
                    {
                        if let Some(button) = weak.upgrade() {
                            button.set_visible(true);
                        }
                    }
                }
            }
        });
        let weak = Rc::downgrade(runtime);
        button.connect_clicked(move |_| {
            let Some(runtime) = weak.upgrade() else {
                return;
            };
            if runtime.attempt.borrow().is_some() {
                return;
            }
            let dialog = adw::AlertDialog::new(Some(&label), None);
            dialog.add_response("cancel", &gettext("Cancel"));
            dialog.add_response("apply", &label);
            dialog.set_close_response("cancel");
            dialog.set_response_appearance("apply", adw::ResponseAppearance::Destructive);
            let weak = Rc::downgrade(&runtime);
            dialog.connect_response(None, move |_, response| {
                if response != "apply" {
                    return;
                }
                let weak = weak.clone();
                glib::spawn_future_local(async move {
                    let result: zbus::Result<()> = async {
                        let connection = zbus::Connection::system().await?;
                        zbus::Proxy::new(
                            &connection,
                            "org.freedesktop.login1",
                            "/org/freedesktop/login1",
                            "org.freedesktop.login1.Manager",
                        )
                        .await?
                        .call(method, &(false,))
                        .await
                    }
                    .await;
                    if result.is_err() {
                        if let Some(runtime) = weak.upgrade() {
                            runtime
                                .view
                                .widgets()
                                .message
                                .set_text(&gettext("The power request failed."));
                        }
                    }
                });
            });
            dialog.present(Some(&runtime.window));
        });
    }
}
fn activate(app: &adw::Application, socket: PathBuf) {
    if let Err(error) = asena_widgets::init() {
        tracing::error!(%error,"Greeter resources unavailable");
        app.quit();
        return;
    }
    let view = content::Content::default();
    let w = view.widgets();
    w.username.set_placeholder_text(Some(&gettext("User name")));
    w.username.set_activates_default(true);
    w.cancel.set_label(&gettext("Cancel"));
    w.submit.set_label(&gettext("Sign in"));
    w.submit.set_sensitive(false);
    w.username
        .update_property(&[gtk::accessible::Property::Label(&gettext("User name"))]);
    w.sessions
        .update_property(&[gtk::accessible::Property::Label(&gettext("Session"))]);
    if let Some(delegate) = w.password.delegate().and_downcast::<gtk::Text>() {
        delegate.set_max_length(4096);
    }
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(gettext("Sign in"))
        .content(&view)
        .default_width(800)
        .default_height(600)
        .build();
    window.add_css_class("asena-lock");
    window.set_default_widget(Some(&*w.submit));
    window.fullscreen();
    let runtime = Rc::new(Runtime {
        view: view.clone(),
        window: window.clone(),
        socket,
        sessions: RefCell::new(Vec::new()),
        attempt: RefCell::new(None),
        input: Cell::new(Input::None),
        cancelling: Cell::new(false),
        starting: Cell::new(false),
        keyboard: RefCell::new(None),
    });
    runtime.keyboard();
    power(&runtime);
    let weak = Rc::downgrade(&runtime);
    w.submit.connect_clicked(move |_| {
        if let Some(runtime) = weak.upgrade() {
            runtime.submit();
        }
    });
    let weak = Rc::downgrade(&runtime);
    w.cancel.connect_clicked(move |_| {
        if let Some(runtime) = weak.upgrade() {
            runtime.cancel();
        }
    });
    let avatar = w.avatar.clone();
    w.username
        .connect_changed(move |entry| avatar.set_text(Some(entry.text().as_str())));
    let weak = Rc::downgrade(&runtime);
    window.connect_close_request(move |_| {
        if let Some(runtime) = weak.upgrade() {
            if runtime.attempt.borrow().is_some() {
                runtime.cancel();
                return glib::Propagation::Stop;
            }
        }
        glib::Propagation::Proceed
    });
    let key = gtk::EventControllerKey::new();
    let weak = Rc::downgrade(&runtime);
    key.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            if let Some(runtime) = weak.upgrade() {
                runtime.cancel();
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.add_controller(key);
    let weak = Rc::downgrade(&runtime);
    glib::spawn_future_local(async move {
        let data = gio::spawn_blocking(|| (sessions::load(), remember::read())).await;
        let Some(runtime) = weak.upgrade() else {
            return;
        };
        let w = runtime.view.widgets();
        if let Ok((sessions, remembered)) = data {
            let names: Vec<&str> = sessions
                .iter()
                .map(|session| session.name.as_str())
                .collect();
            w.sessions.set_model(Some(&gtk::StringList::new(&names)));
            w.sessions.set_selected(
                sessions::preferred(&sessions)
                    .map(|index| index as u32)
                    .unwrap_or(gtk::INVALID_LIST_POSITION),
            );
            if let Some(username) = remembered {
                w.username.set_text(&username);
            }
            runtime.sessions.replace(sessions);
        }
        if runtime.sessions.borrow().is_empty() {
            w.message
                .set_text(&gettext("No Wayland session is available."));
        }
        runtime.controls(false);
        tracing::info!(
            available = runtime.sessions.borrow().len(),
            "Greeter sessions loaded"
        );
    });
    let weak = Rc::downgrade(&runtime);
    glib::spawn_future_local(async move {
        let Ok(users) = accounts::users().await else {
            return;
        };
        let Some(runtime) = weak.upgrade() else {
            return;
        };
        let w = runtime.view.widgets();
        if users.is_empty() {
            return;
        }
        let names: Vec<&str> = users.iter().map(|user| user.name.as_str()).collect();
        w.users.set_model(Some(&gtk::StringList::new(&names)));
        w.users.set_visible(true);
        w.users.set_selected(gtk::INVALID_LIST_POSITION);
        let entry = w.username.downgrade();
        w.users.connect_selected_notify(move |dropdown| {
            if let Some(user) = users.get(dropdown.selected() as usize) {
                if let Some(entry) = entry.upgrade() {
                    entry.set_text(&user.username);
                }
            }
        });
    });
    app.connect_shutdown(move |_| {
        runtime.clear();
        if let Some(attempt) = runtime.attempt.borrow().as_ref() {
            attempt.cancel();
        }
    });
    window.present();
    w.username.grab_focus();
}
fn main() -> anyhow::Result<()> {
    asena_core::logging::init("asena-greeter").map_err(|error| anyhow::anyhow!("{error}"))?;
    asena_core::i18n::init().map_err(|error| anyhow::anyhow!("{error}"))?;
    let socket = PathBuf::from(
        std::env::var_os("GREETD_SOCK")
            .ok_or_else(|| anyhow::anyhow!(gettext("Start the login screen through greetd.")))?,
    );
    anyhow::ensure!(
        socket.is_absolute(),
        gettext("The login service is unavailable.")
    );
    let app = adw::Application::builder()
        .application_id("org.asenalinux.Greeter")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| activate(app, socket.clone()));
    app.run();
    Ok(())
}
