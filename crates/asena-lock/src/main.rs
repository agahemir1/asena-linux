use adw::prelude::*;
use asena_core::{i18n::gettext, lock::LockState};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};
use zeroize::Zeroizing;
mod auth;
mod bus;
mod content;
mod sleep;

struct Runtime {
    app: adw::Application,
    lock: gtk4_session_lock::Instance,
    state: RefCell<LockState>,
    views: RefCell<Vec<glib::WeakRef<content::Content>>>,
    events: async_channel::Sender<auth::Event>,
    reply: RefCell<Option<mpsc::SyncSender<Zeroizing<String>>>>,
    cancellation: RefCell<Option<Arc<AtomicBool>>>,
    user: asena_services::user::UserModel,
    username: String,
    prompt: RefCell<Option<(String, bool)>>,
    status: RefCell<String>,
    readiness: bus::Readiness,
    sleep_shown: Cell<bool>,
}
impl Runtime {
    fn each(&self, f: impl Fn(&content::Content)) {
        self.views.borrow_mut().retain(|weak| {
            if let Some(view) = weak.upgrade() {
                f(&view);
                true
            } else {
                false
            }
        });
    }
    fn clear(&self) {
        self.each(|view| {
            view.widgets().password.set_text("");
            view.widgets().answer.set_text("");
        });
    }
    fn message(&self, text: &str) {
        self.status.replace(text.to_owned());
        self.each(|view| {
            view.widgets().message.set_text(text);
            view.widgets().message.set_visible(!text.is_empty());
        });
    }
    fn controls(&self, enabled: bool) {
        let enabled = enabled && !self.readiness.held();
        self.each(|view| {
            let w = view.widgets();
            w.password.set_sensitive(enabled);
            w.answer.set_sensitive(enabled);
            w.submit.set_sensitive(enabled);
            w.cancel.set_sensitive(self.state.borrow().busy());
            w.cancel.set_visible(self.state.borrow().busy());
        });
    }
    fn focus(&self) {
        for window in self.app.windows() {
            if window.is_active() {
                if let Some(view) = window.child().and_downcast::<content::Content>() {
                    if view.widgets().password.is_visible() {
                        view.widgets().password.grab_focus();
                    } else {
                        view.widgets().answer.grab_focus();
                    }
                }
            }
        }
    }
    fn cancel(&self) {
        self.state.borrow_mut().cancel();
        if let Some(cancelled) = self.cancellation.borrow().as_ref() {
            cancelled.store(true, Ordering::SeqCst);
        }
        self.reply.borrow_mut().take();
        self.clear();
        self.controls(false);
        self.message(&gettext("Cancelling authentication…"));
    }
    fn submit(&self, view: &content::Content) {
        if self.readiness.held() {
            self.clear();
            return;
        }
        let w = view.widgets();
        let text = if w.password.is_visible() {
            w.password.text()
        } else {
            w.answer.text()
        };
        if text.len() > 4096 {
            self.clear();
            return;
        }
        let password = Zeroizing::new(text.to_string());
        drop(text);
        self.clear();
        self.message("");
        if let Some(reply) = self.reply.borrow_mut().take() {
            let _ = reply.try_send(password);
            self.controls(false);
            return;
        }
        let Some(ticket) = self.state.borrow_mut().begin() else {
            return;
        };
        self.controls(false);
        let cancelled = auth::start(ticket, self.username.clone(), password, self.events.clone());
        self.cancellation.replace(Some(cancelled));
    }
    fn surface(self: &Rc<Self>, monitor: &gtk::gdk::Monitor) {
        let view = content::Content::default();
        view.keyboard();
        let widgets = view.widgets();
        widgets.answer.set_max_length(4096);
        if let Some(text) = widgets.password.delegate().and_downcast::<gtk::Text>() {
            text.set_max_length(4096);
        }
        let (prompt, secret) = self
            .prompt
            .borrow()
            .clone()
            .unwrap_or_else(|| (gettext("Password"), true));
        widgets.prompt.set_text(&prompt);
        widgets.password.set_visible(secret);
        widgets.answer.set_visible(!secret);
        widgets.authentication.set_visible(false);
        widgets.submit.set_label(&gettext("Unlock"));
        widgets.cancel.set_label(&gettext("Cancel"));
        widgets.cancel.set_sensitive(self.state.borrow().busy());
        widgets.cancel.set_visible(self.state.borrow().busy());
        widgets.message.set_text(&self.status.borrow());
        widgets
            .message
            .set_visible(!self.status.borrow().is_empty());
        let enabled = self.state.borrow().ready()
            && (!self.state.borrow().busy() || self.reply.borrow().is_some());
        widgets.password.set_sensitive(enabled);
        widgets.answer.set_sensitive(enabled);
        widgets.submit.set_sensitive(enabled);
        self.user
            .bind_property("name", &*widgets.name, "label")
            .sync_create()
            .build();
        self.user
            .bind_property("name", &*widgets.avatar, "text")
            .sync_create()
            .build();
        let window = gtk::ApplicationWindow::builder()
            .application(&self.app)
            .title(gettext("Lock"))
            .child(&view)
            .build();
        window.add_css_class("asena-lock");
        window.set_default_widget(Some(&*widgets.submit));
        window.connect_close_request(|_| glib::Propagation::Stop);
        let weak = view.downgrade();
        window.connect_is_active_notify(move |window| {
            if let Some(view) = weak.upgrade() {
                view.keyboard();
                view.widgets()
                    .authentication
                    .set_visible(window.is_active());
                if !window.is_active() {
                    view.widgets().password.set_text("");
                    view.widgets().answer.set_text("");
                } else if view.widgets().password.is_visible() {
                    view.widgets().password.grab_focus();
                } else {
                    view.widgets().answer.grab_focus();
                }
            }
        });
        let weak = Rc::downgrade(self);
        let target = view.downgrade();
        widgets.submit.connect_clicked(move |_| {
            if let (Some(runtime), Some(view)) = (weak.upgrade(), target.upgrade()) {
                runtime.submit(&view);
            }
        });
        let weak = Rc::downgrade(self);
        widgets.cancel.connect_clicked(move |_| {
            if let Some(runtime) = weak.upgrade() {
                runtime.cancel();
            }
        });
        let keys = gtk::EventControllerKey::new();
        let weak = Rc::downgrade(self);
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                if let Some(runtime) = weak.upgrade() {
                    if runtime.state.borrow().busy() {
                        runtime.cancel();
                    } else {
                        runtime.clear();
                    }
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        window.add_controller(keys);
        self.views.borrow_mut().push(view.downgrade());
        self.clock();
        // C 1.3.0 maps and sizes this surface, including scale and monitor lifecycle.
        self.lock.assign_window_to_monitor(&window, monitor);
        tracing::info!(connector=?monitor.connector(),"Lock surface assigned");
    }
    fn clock(&self) {
        if let Ok(now) = glib::DateTime::now_local() {
            self.each(|view| {
                if let Ok(time) = now.format(&gettext("%H:%M")) {
                    view.widgets().clock.set_text(&time);
                }
                if let Ok(date) = now.format(&gettext("%A, %e %B")) {
                    view.widgets().date.set_text(&date);
                }
            });
        }
    }
    fn refresh_sleep(&self) {
        let held = self.readiness.held();
        let was_held = self.sleep_shown.replace(held);
        if held {
            self.state.borrow_mut().cancel();
            if let Some(cancelled) = self.cancellation.borrow().as_ref() {
                cancelled.store(true, Ordering::SeqCst);
            }
            self.reply.borrow_mut().take();
            self.clear();
            self.controls(false);
            self.message(&gettext("Unlock after resuming."));
        } else {
            if was_held {
                self.message("");
            }
            self.controls(
                self.state.borrow().ready()
                    && (!self.state.borrow().busy() || self.reply.borrow().is_some()),
            );
            if was_held {
                self.focus();
            }
        }
    }
    fn event(&self, event: auth::Event) {
        match event {
            auth::Event::Prompt {
                text,
                secret,
                reply,
            } => {
                if self
                    .cancellation
                    .borrow()
                    .as_ref()
                    .is_some_and(|cancelled| cancelled.load(Ordering::SeqCst))
                {
                    drop(reply);
                    return;
                }
                self.prompt.replace(Some((text.clone(), secret)));
                self.reply.replace(Some(reply));
                self.each(|view| {
                    let w = view.widgets();
                    w.prompt.set_text(&text);
                    w.password.set_visible(secret);
                    w.answer.set_visible(!secret);
                });
                self.controls(true);
                self.focus();
            }
            auth::Event::Message(text) => self.message(&text),
            auth::Event::Finished { ticket, success } => {
                let cancelled = self
                    .cancellation
                    .borrow()
                    .as_ref()
                    .is_some_and(|value| value.load(Ordering::SeqCst));
                self.prompt.borrow_mut().take();
                let accepted = self.state.borrow_mut().complete(ticket, success);
                self.reply.borrow_mut().take();
                self.cancellation.borrow_mut().take();
                self.clear();
                tracing::info!(accepted, "Authentication attempt completed");
                if accepted && self.state.borrow().may_unlock() {
                    if self.readiness.begin_unlock() {
                        self.lock.unlock();
                    } else {
                        self.state.borrow_mut().cancel();
                        self.refresh_sleep();
                    }
                } else {
                    self.each(|view| {
                        let w = view.widgets();
                        w.prompt.set_text(&gettext("Password"));
                        w.password.set_visible(true);
                        w.answer.set_visible(false);
                    });
                    self.message(&if cancelled {
                        gettext("Authentication cancelled.")
                    } else {
                        gettext("Authentication failed. Try again.")
                    });
                    self.controls(true);
                    if self.readiness.held() {
                        self.refresh_sleep();
                    }
                    self.focus();
                }
            }
        }
    }
}

fn activate(app: &adw::Application, success: Rc<Cell<bool>>) {
    let app = app.clone();
    let hold = app.hold();
    glib::spawn_future_local(async move {
        let (focus, requests) = async_channel::bounded(1);
        let readiness = bus::Readiness::default();
        let scope = gtk::gdk::Display::default().and_then(|display| {
            asena_core::session::for_display(std::path::Path::new(display.name().as_str())).ok()
        });
        let Some(scope) = scope else {
            tracing::error!("Could not identify the lock display");
            app.quit();
            return;
        };
        let connection = match zbus::connection::Builder::session().and_then(|builder| {
            builder.serve_at(
                asena_core::dbus::LOCK_PATH,
                bus::LockBus {
                    readiness: readiness.clone(),
                    focus,
                    scope: scope.clone(),
                },
            )
        }) {
            Ok(builder) => builder.build().await,
            Err(error) => Err(error),
        };
        let connection = match connection {
            Ok(connection) => connection,
            Err(error) => {
                tracing::error!(%error,"Could not connect the locker to D-Bus");
                app.quit();
                return;
            }
        };
        if asena_core::dbus::claim_name(&connection, asena_core::dbus::LOCK_ID)
            .await
            .is_err()
        {
            if let Ok(proxy) = asena_core::dbus::LockProxy::new(&connection).await {
                let result = futures_lite::future::race(proxy.wait_ready(&scope), async {
                    glib::timeout_future(Duration::from_secs(5)).await;
                    Err(zbus::Error::Failure(gettext(
                        "The session lock did not become ready.",
                    )))
                })
                .await;
                if result.is_ok() {
                    let _ = proxy.focus().await;
                    success.set(true);
                }
            }
            app.quit();
            return;
        }
        if let Err(error) = asena_widgets::init() {
            tracing::error!(%error,"Could not initialize lock resources");
            readiness.finish(false);
            app.quit();
            return;
        }
        let (events, receiver) = async_channel::unbounded();
        let user = asena_services::user::UserModel::default();
        user.start();
        let runtime = Rc::new(Runtime {
            app: app.clone(),
            lock: gtk4_session_lock::Instance::new(),
            state: RefCell::new(LockState::default()),
            views: RefCell::new(Vec::new()),
            events,
            reply: RefCell::new(None),
            cancellation: RefCell::new(None),
            user,
            prompt: RefCell::new(None),
            status: RefCell::new(String::new()),
            readiness: readiness.clone(),
            sleep_shown: Cell::new(false),
            username: glib::user_name().to_string_lossy().into_owned(),
        });
        let weak = Rc::downgrade(&runtime);
        runtime.lock.connect_local("monitor", false, move |values| {
            if let (Some(runtime), Some(monitor)) = (
                weak.upgrade(),
                values
                    .get(1)
                    .and_then(|value| value.get::<gtk::gdk::Monitor>().ok()),
            ) {
                runtime.surface(&monitor);
            }
            None
        });
        let weak = Rc::downgrade(&runtime);
        let ready = readiness.clone();
        runtime.lock.connect_locked(move |_| {
            if let Some(runtime) = weak.upgrade() {
                runtime.state.borrow_mut().compositor_locked();
                runtime.controls(true);
                runtime.focus();
                ready.finish(true);
                tracing::info!("Compositor acknowledged session lock");
            }
        });
        let failed = app.clone();
        let ready = readiness.clone();
        runtime.lock.connect_failed(move |_| {
            ready.finish(false);
            tracing::error!("Compositor rejected session lock");
            failed.quit();
        });
        let unlocked = app.clone();
        let result = success.clone();
        runtime.lock.connect_unlocked(move |_| {
            result.set(true);
            unlocked.quit();
        });
        let weak = Rc::downgrade(&runtime);
        let clock = glib::timeout_add_local(Duration::from_secs(1), move || {
            if let Some(runtime) = weak.upgrade() {
                runtime.clock();
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
        let weak = Rc::downgrade(&runtime);
        let focus_task = glib::spawn_future_local(async move {
            while let Ok(request) = requests.recv().await {
                if let Some(runtime) = weak.upgrade() {
                    runtime.refresh_sleep();
                    if matches!(request, bus::UiRequest::Focus) {
                        runtime.focus();
                    }
                } else {
                    break;
                }
            }
        });
        // lock() can synchronously emit failed; handlers must already exist.
        if !runtime.lock.lock() {
            readiness.finish(false);
            app.quit();
            clock.remove();
            focus_task.abort();
            return;
        }
        let weak = Rc::downgrade(&runtime);
        let event_task = glib::spawn_future_local(async move {
            while let Ok(event) = receiver.recv().await {
                if let Some(runtime) = weak.upgrade() {
                    runtime.event(event);
                } else {
                    break;
                }
            }
        });
        // The controller and connection must outlive every lock surface.
        let clock = RefCell::new(Some(clock));
        let watch_ready = readiness.clone();
        let watch_connection = connection.clone();
        let watch_runtime = Rc::downgrade(&runtime);
        let sleep_task = glib::spawn_future_local(async move {
            let result = sleep::watch(watch_ready, watch_connection, move || {
                if let Some(runtime) = watch_runtime.upgrade() {
                    runtime.refresh_sleep();
                }
            })
            .await;
            if let Err(error) = result {
                tracing::warn!(%error,"Sleep recovery monitoring is unavailable");
            }
        });
        app.connect_shutdown(move |_| {
            runtime.readiness.finish(false);
            runtime.clear();
            if let Some(cancelled) = runtime.cancellation.borrow().as_ref() {
                cancelled.store(true, Ordering::SeqCst);
            }
            runtime.reply.borrow_mut().take();
            event_task.abort();
            focus_task.abort();
            sleep_task.abort();
            if let Some(clock) = clock.borrow_mut().take() {
                clock.remove();
            }
            let _ = (&connection, &hold);
        });
    });
}
fn main() -> anyhow::Result<()> {
    asena_core::logging::init("asena-lock").map_err(|error| anyhow::anyhow!("{error}"))?;
    asena_core::i18n::init().map_err(|error| anyhow::anyhow!("{error}"))?;
    let app = adw::Application::builder()
        .application_id(asena_core::dbus::LOCK_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let success = Rc::new(Cell::new(false));
    let result = success.clone();
    app.connect_activate(move |app| activate(app, result.clone()));
    app.run();
    anyhow::ensure!(success.get(), gettext("The session could not be locked."));
    Ok(())
}
