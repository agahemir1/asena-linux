//! PAM runs on one blocking worker. No credential crosses D-Bus or a log sink.
use pam_client::{Context, ConversationHandler, ErrorCode, Flag};
use std::{
    ffi::{CStr, CString},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};
use zeroize::Zeroizing;

pub enum Event {
    Prompt {
        text: String,
        secret: bool,
        reply: mpsc::SyncSender<Zeroizing<String>>,
    },
    Message(String),
    Finished {
        ticket: u64,
        success: bool,
    },
}
struct Conversation {
    initial: Option<Zeroizing<String>>,
    cancelled: Arc<AtomicBool>,
    events: async_channel::Sender<Event>,
}
impl Conversation {
    fn prompt(&mut self, text: &CStr, secret: bool) -> Result<CString, ErrorCode> {
        if self.cancelled.load(Ordering::SeqCst) {
            return Err(ErrorCode::CONV_ERR);
        }
        if secret {
            if let Some(mut initial) = self.initial.take() {
                return CString::new(std::mem::take(&mut *initial))
                    .map_err(|_| ErrorCode::CONV_ERR);
            }
        }
        let (reply, receiver) = mpsc::sync_channel(1);
        self.events
            .try_send(Event::Prompt {
                text: text.to_string_lossy().chars().take(1024).collect(),
                secret,
                reply,
            })
            .map_err(|_| ErrorCode::CONV_ERR)?;
        let mut answer = receiver
            .recv_timeout(Duration::from_secs(120))
            .map_err(|_| ErrorCode::CONV_ERR)?;
        if self.cancelled.load(Ordering::SeqCst) {
            return Err(ErrorCode::CONV_ERR);
        }
        CString::new(std::mem::take(&mut *answer)).map_err(|_| ErrorCode::CONV_ERR)
    }
}
impl ConversationHandler for Conversation {
    fn prompt_echo_on(&mut self, text: &CStr) -> Result<CString, ErrorCode> {
        self.prompt(text, false)
    }
    fn prompt_echo_off(&mut self, text: &CStr) -> Result<CString, ErrorCode> {
        self.prompt(text, true)
    }
    fn text_info(&mut self, text: &CStr) {
        let _ = self.events.try_send(Event::Message(
            text.to_string_lossy().chars().take(1024).collect(),
        ));
    }
    fn error_msg(&mut self, text: &CStr) {
        self.text_info(text);
    }
    fn radio_prompt(&mut self, _: &CStr) -> Result<bool, ErrorCode> {
        Err(ErrorCode::CONV_ERR)
    }
}
pub fn start(
    ticket: u64,
    username: String,
    password: Zeroizing<String>,
    events: async_channel::Sender<Event>,
) -> Arc<AtomicBool> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let pending = cancelled.clone();
    gio::spawn_blocking(move || {
        let conversation = Conversation {
            initial: Some(password),
            cancelled: pending.clone(),
            events: events.clone(),
        };
        let success = asena_services::lock_client::trusted_policy()
            && (|| {
                let mut context = Context::new("asena-lock", Some(&username), conversation)?;
                context.authenticate(Flag::DISALLOW_NULL_AUTHTOK)?;
                context.acct_mgmt(Flag::NONE)?;
                // A PAM module must not authenticate another account to unlock this session.
                Ok::<_, pam_client::Error>(
                    context.user()? == username && !pending.load(Ordering::SeqCst),
                )
            })()
            .unwrap_or(false);
        let _ = events.try_send(Event::Finished { ticket, success });
    });
    cancelled
}
