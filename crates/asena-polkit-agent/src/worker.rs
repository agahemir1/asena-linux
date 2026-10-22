//! Native listener, registration and PAM helpers stay on one private GLib thread.
use asena_core::authorization::{Completion, Flow};
use gio::prelude::*;
use glib::subclass::prelude::*;
use polkit_agent::traits::ListenerExt;
use std::{
    cell::{Cell, RefCell},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use zeroize::Zeroizing;
pub enum Command {
    Select(u64, usize),
    Answer(u64, u64, Zeroizing<String>),
    Retry(u64),
    Cancel(u64),
    Stop,
}
pub enum Event {
    Ready,
    Unavailable(String),
    Request {
        ticket: u64,
        message: String,
        identities: Vec<String>,
        cancelled: Arc<AtomicBool>,
    },
    Prompt {
        ticket: u64,
        epoch: u64,
        text: String,
        echo: bool,
    },
    Message(u64, String),
    Retry(u64),
    Finished(u64, bool, bool),
    Stopped,
}
struct Request {
    ticket: u64,
    cookie: Zeroizing<String>,
    identities: Vec<polkit::UnixUser>,
    selected: usize,
    task: gio::LocalTask<bool>,
    flow: Flow,
    session: Option<polkit_agent::Session>,
    cancelled: Arc<AtomicBool>,
}
fn local(task: gio::Task<bool>) -> Result<gio::LocalTask<bool>, glib::Error> {
    // This fixed ListenerImpl::Message is bool. Moving a Send Task into its
    // thread-local view preserves that exact payload type and weakens Send.
    task.upcast::<glib::Object>()
        .downcast::<gio::LocalTask<bool>>()
        .map_err(|_| glib::Error::new(gio::IOErrorEnum::InvalidData, "Invalid authorization task"))
}
mod imp {
    use super::*;
    #[derive(Default)]
    pub struct Listener {
        pub(super) active: RefCell<Option<Request>>,
        pub(super) next: Cell<u64>,
        pub(super) events: RefCell<Option<async_channel::Sender<Event>>>,
        pub(super) commands: RefCell<Option<async_channel::Sender<Command>>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for Listener {
        const NAME: &'static str = "AsenaAuthorizationListener";
        type Type = super::Listener;
        type ParentType = polkit_agent::Listener;
    }
    impl ObjectImpl for Listener {}
    impl polkit_agent::subclass::ListenerImpl for Listener {
        type Message = bool;
        fn initiate_authentication(
            &self,
            _action_id: &str,
            message: &str,
            _icon_name: &str,
            _details: &polkit::Details,
            cookie: &str,
            identities: Vec<polkit::Identity>,
            cancellable: gio::Cancellable,
            task: gio::Task<bool>,
        ) {
            let Ok(task) = local(task) else {
                return;
            };
            if self.active.borrow().is_some()
                || cookie.len() > 4096
                || message.len() > 16384
                || identities.len() > 32
            {
                task.return_result(Err(glib::Error::new(
                    gio::IOErrorEnum::Busy,
                    "Authorization agent is busy",
                )));
                return;
            }
            let identities: Vec<_> = identities
                .into_iter()
                .filter_map(|identity| identity.downcast::<polkit::UnixUser>().ok())
                .collect();
            if identities.is_empty() {
                task.return_result(Err(glib::Error::new(
                    gio::IOErrorEnum::NotSupported,
                    "No supported authorization identity",
                )));
                return;
            }
            let ticket = self.next.get().wrapping_add(1);
            self.next.set(ticket);
            let cancelled = Arc::new(AtomicBool::new(false));
            let names = identities
                .iter()
                .map(|user| {
                    user.name()
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| user.uid().to_string())
                })
                .collect();
            self.active.replace(Some(Request {
                ticket,
                cookie: Zeroizing::new(cookie.to_owned()),
                identities,
                selected: 0,
                task,
                flow: Flow::default(),
                session: None,
                cancelled: cancelled.clone(),
            }));
            if let Some(commands) = self.commands.borrow().clone() {
                cancellable.connect_cancelled(move |_| {
                    let _ = commands.try_send(Command::Cancel(ticket));
                });
            }
            if cancellable.is_cancelled() {
                self.obj().finish(
                    ticket,
                    Err(glib::Error::new(
                        gio::IOErrorEnum::Cancelled,
                        "Authorization cancelled",
                    )),
                    false,
                );
                return;
            }
            self.obj().emit(Event::Request {
                ticket,
                message: message.to_owned(),
                identities: names,
                cancelled,
            });
            self.obj().start(ticket, 0);
        }
        fn initiate_authentication_finish(
            &self,
            result: Result<gio::Task<bool>, glib::Error>,
        ) -> Result<bool, glib::Error> {
            local(result?)?.propagate()
        }
    }
}
glib::wrapper! {pub struct Listener(ObjectSubclass<imp::Listener>) @extends polkit_agent::Listener;}
impl Listener {
    fn emit(&self, event: Event) {
        if let Some(events) = self.imp().events.borrow().as_ref() {
            let _ = events.try_send(event);
        }
    }
    fn finish(&self, ticket: u64, result: Result<bool, glib::Error>, gained: bool) {
        let request = {
            let mut active = self.imp().active.borrow_mut();
            if active
                .as_ref()
                .is_none_or(|request| request.ticket != ticket)
            {
                return;
            }
            active.take()
        };
        if let Some(mut request) = request {
            request.flow.cancel();
            request.cancelled.store(true, Ordering::SeqCst);
            if let Some(session) = request.session.take() {
                session.cancel();
            }
            let cancelled = result.is_err();
            request.task.return_result(result);
            self.emit(Event::Finished(ticket, gained, cancelled));
        }
    }
    fn start(&self, ticket: u64, index: usize) {
        let next = {
            let mut active = self.imp().active.borrow_mut();
            let Some(request) = active.as_mut() else {
                return;
            };
            if request.ticket != ticket || request.cancelled.load(Ordering::SeqCst) {
                return;
            }
            let Some(identity) = request.identities.get(index).cloned() else {
                return;
            };
            request.flow.start().map(|epoch| {
                request.selected = index;
                (
                    epoch,
                    identity,
                    request.cookie.clone(),
                    request.session.take(),
                )
            })
        };
        let Some((epoch, identity, cookie, previous)) = next else {
            self.finish(ticket, Ok(true), false);
            return;
        };
        if let Some(previous) = previous {
            previous.cancel();
        }
        let session = polkit_agent::Session::new(&identity, &cookie);
        let weak = self.downgrade();
        session.connect_request(move |_, text, echo| {
            if let Some(listener) = weak.upgrade() {
                let valid = listener
                    .imp()
                    .active
                    .borrow()
                    .as_ref()
                    .is_some_and(|request| {
                        request.ticket == ticket
                            && request.flow.current(epoch)
                            && !request.cancelled.load(Ordering::SeqCst)
                    });
                if valid {
                    listener.emit(Event::Prompt {
                        ticket,
                        epoch,
                        text: text.to_owned(),
                        echo,
                    });
                }
            }
        });
        let weak = self.downgrade();
        session.connect_show_error(move |_, text| {
            if let Some(listener) = weak.upgrade() {
                listener.message(ticket, epoch, text);
            }
        });
        let weak = self.downgrade();
        session.connect_show_info(move |_, text| {
            if let Some(listener) = weak.upgrade() {
                listener.message(ticket, epoch, text);
            }
        });
        let weak = self.downgrade();
        session.connect_completed(move |_, gained| {
            if let Some(listener) = weak.upgrade() {
                let outcome = {
                    let mut active = listener.imp().active.borrow_mut();
                    let Some(request) = active.as_mut() else {
                        return;
                    };
                    if request.ticket != ticket || request.cancelled.load(Ordering::SeqCst) {
                        return;
                    }
                    request.flow.complete(epoch, gained)
                };
                match outcome {
                    Completion::Ignore => {}
                    Completion::Retry => listener.emit(Event::Retry(ticket)),
                    Completion::Finished(gained) => listener.finish(ticket, Ok(true), gained),
                }
            }
        });
        if let Some(request) = self.imp().active.borrow_mut().as_mut() {
            request.session = Some(session.clone());
        }
        session.initiate();
    }
    fn message(&self, ticket: u64, epoch: u64, text: &str) {
        if self
            .imp()
            .active
            .borrow()
            .as_ref()
            .is_some_and(|request| request.ticket == ticket && request.flow.current(epoch))
        {
            self.emit(Event::Message(ticket, text.chars().take(4096).collect()));
        }
    }
    fn command(&self, command: Command) {
        match command {
            Command::Select(ticket, index) => self.start(ticket, index),
            Command::Retry(ticket) => {
                let index = self
                    .imp()
                    .active
                    .borrow()
                    .as_ref()
                    .filter(|request| request.ticket == ticket)
                    .map(|request| request.selected);
                if let Some(index) = index {
                    self.start(ticket, index);
                }
            }
            Command::Answer(ticket, epoch, answer) => {
                let session = self
                    .imp()
                    .active
                    .borrow()
                    .as_ref()
                    .filter(|request| {
                        request.ticket == ticket
                            && request.flow.current(epoch)
                            && !request.cancelled.load(Ordering::SeqCst)
                            && answer.len() <= 4096
                    })
                    .and_then(|request| request.session.clone());
                if let Some(session) = session {
                    session.response(&answer);
                }
            }
            Command::Cancel(ticket) => self.finish(
                ticket,
                Err(glib::Error::new(
                    gio::IOErrorEnum::Cancelled,
                    "Authorization cancelled",
                )),
                false,
            ),
            Command::Stop => {
                let ticket = self
                    .imp()
                    .active
                    .borrow()
                    .as_ref()
                    .map(|request| request.ticket);
                if let Some(ticket) = ticket {
                    self.finish(
                        ticket,
                        Err(glib::Error::new(
                            gio::IOErrorEnum::Cancelled,
                            "Agent stopped",
                        )),
                        false,
                    );
                }
            }
        }
    }
}
pub fn start(
    events: async_channel::Sender<Event>,
) -> std::io::Result<async_channel::Sender<Command>> {
    let (sender, receiver) = async_channel::unbounded();
    let commands = sender.clone();
    std::thread::Builder::new()
        .name("asena-polkit".into())
        .spawn(move || {
            let context = glib::MainContext::new();
            let result = context.with_thread_default(|| {
                let listener: Listener = glib::Object::new();
                listener.imp().events.replace(Some(events.clone()));
                listener.imp().commands.replace(Some(commands));
                let subject = if let Ok(id) = std::env::var("XDG_SESSION_ID") {
                    polkit::UnixSession::new(&id)
                } else {
                    polkit::UnixSession::new_for_process_sync(
                        std::process::id() as i32,
                        gio::Cancellable::NONE,
                    )?
                };
                let registration = listener.register(
                    polkit_agent::RegisterFlags::NONE,
                    &subject,
                    "/org/asenalinux/PolkitAgent",
                    gio::Cancellable::NONE,
                )?;
                let main_loop = glib::MainLoop::new(Some(&context), false);
                let finished = main_loop.clone();
                let event_stream = events.clone();
                context.spawn_local(async move {
                    let _ = event_stream.try_send(Event::Ready);
                    while let Ok(command) = receiver.recv().await {
                        let stopping = matches!(command, Command::Stop);
                        listener.command(command);
                        if stopping {
                            break;
                        }
                    }
                    drop(registration);
                    glib::timeout_future(std::time::Duration::from_millis(1)).await;
                    let _ = event_stream.try_send(Event::Stopped);
                    finished.quit();
                });
                main_loop.run();
                Ok::<_, glib::Error>(())
            });
            match result {
                Ok(Ok(())) => {}
                error => {
                    let _ = events.try_send(Event::Unavailable(format!("{error:?}")));
                }
            }
        })?;
    Ok(sender)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_error_callbacks_complete_and_release_the_listener() -> Result<(), glib::BoolError> {
        let context = glib::MainContext::new();
        context.with_thread_default(|| {
            let listener: Listener = glib::Object::new();
            let weak = listener.downgrade();
            let identity: polkit::Identity = polkit::UnixUser::new(1000).upcast();
            let oversized_cookie = "x".repeat(5000);
            let result = context.block_on(listener.initiate_authentication_future(
                "fixture",
                "fixture",
                "",
                &polkit::Details::new(),
                &oversized_cookie,
                &[identity],
            ));
            assert!(result.is_err());
            drop(listener);
            while context.pending() {
                context.iteration(false);
            }
            assert!(weak.upgrade().is_none());
        })
    }
}
