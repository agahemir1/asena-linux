//! Bounded greetd framing. All socket IO and PAM waiting live on this worker.
use crate::sessions::Session;
use greetd_ipc::{AuthMessageType, Request, Response};
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use zeroize::{Zeroize, Zeroizing};
pub enum Answer {
    Text(Zeroizing<String>),
    Continue,
    Start,
    Cancel,
}
pub enum Event {
    Prompt(String, bool),
    Information(String),
    Authenticated,
    Failed,
    Disconnected,
    Started(String),
    Done,
}
pub struct Attempt {
    pub cancelled: Arc<AtomicBool>,
    pub answers: async_channel::Sender<Answer>,
}
impl Attempt {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        let _ = self.answers.try_send(Answer::Cancel);
    }
}
// greetd 0.10.3 joins these strings before /bin/sh -c. Preserve parsed argv as
// literal words at that boundary; never send the desktop file's raw Exec text.
fn transport_command(arguments: Vec<String>) -> Vec<String> {
    arguments
        .into_iter()
        .map(|arg| format!("'{}'", arg.replace('\'', "'\\''")))
        .collect()
}
fn exchange(stream: &mut UnixStream, mut request: Request) -> anyhow::Result<Response> {
    let bytes = serde_json::to_vec(&request);
    if let Request::PostAuthMessageResponse {
        response: Some(value),
    } = &mut request
    {
        value.zeroize();
    }
    let bytes = Zeroizing::new(bytes?);
    anyhow::ensure!(bytes.len() <= 65536, "Request frame too large");
    stream.write_all(&(bytes.len() as u32).to_ne_bytes())?;
    stream.write_all(&bytes)?;
    let mut size = [0; 4];
    stream.read_exact(&mut size)?;
    let size = u32::from_ne_bytes(size) as usize;
    anyhow::ensure!(size > 0 && size <= 65536, "Invalid response frame size");
    let mut bytes = Zeroizing::new(vec![0; size]);
    stream.read_exact(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}
pub fn start(
    socket: PathBuf,
    username: String,
    session: Session,
    events: async_channel::Sender<Event>,
) -> std::io::Result<Attempt> {
    let (answers, receiver) = async_channel::bounded(1);
    let cancelled = Arc::new(AtomicBool::new(false));
    let stop = cancelled.clone();
    std::thread::Builder::new()
        .name("asena-greetd".into())
        .spawn(move || {
            let result = run(socket, username, session, &events, receiver, &stop);
            if result.is_err() && !stop.load(Ordering::SeqCst) {
                let _ = events.send_blocking(Event::Disconnected);
            }
            let _ = events.send_blocking(Event::Done);
        })?;
    Ok(Attempt { cancelled, answers })
}
fn run(
    socket: PathBuf,
    username: String,
    session: Session,
    events: &async_channel::Sender<Event>,
    answers: async_channel::Receiver<Answer>,
    cancelled: &AtomicBool,
) -> anyhow::Result<()> {
    let mut stream = futures_lite::future::block_on(futures_lite::future::race(
        async {
            async_io::Async::<UnixStream>::connect(socket)
                .await?
                .into_inner()
        },
        async {
            async_io::Timer::after(Duration::from_secs(5)).await;
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "greetd connect timed out",
            ))
        },
    ))?;
    stream.set_nonblocking(false)?;
    if cancelled.load(Ordering::SeqCst) {
        return Ok(());
    }
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut response = exchange(
        &mut stream,
        Request::CreateSession {
            username: username.clone(),
        },
    )?;
    let mut rounds = 0;
    loop {
        if cancelled.load(Ordering::SeqCst) {
            let _ = exchange(&mut stream, Request::CancelSession);
            return Ok(());
        }
        rounds += 1;
        anyhow::ensure!(rounds <= 128, "Too many authentication messages");
        match response {
            Response::Success => {
                events.send_blocking(Event::Authenticated)?;
                if !matches!(answers.recv_blocking()?, Answer::Start)
                    || cancelled.load(Ordering::SeqCst)
                {
                    let _ = exchange(&mut stream, Request::CancelSession);
                    return Ok(());
                }
                // Recheck cancellation immediately before asking greetd to start.
                if cancelled.load(Ordering::SeqCst) {
                    let _ = exchange(&mut stream, Request::CancelSession);
                    return Ok(());
                }
                let response = exchange(
                    &mut stream,
                    Request::StartSession {
                        cmd: transport_command(session.command),
                        env: session.environment,
                    },
                )?;
                if matches!(response, Response::Success) {
                    events.send_blocking(Event::Started(username))?;
                } else {
                    events.send_blocking(Event::Failed)?;
                }
                return Ok(());
            }
            Response::Error { .. } => {
                events.send_blocking(Event::Failed)?;
                return Ok(());
            }
            Response::AuthMessage {
                auth_message_type,
                auth_message,
            } => {
                let input = matches!(
                    auth_message_type,
                    AuthMessageType::Visible | AuthMessageType::Secret
                );
                let secret = matches!(auth_message_type, AuthMessageType::Secret);
                events.send_blocking(if input {
                    Event::Prompt(auth_message, secret)
                } else {
                    Event::Information(auth_message)
                })?;
                let answer = answers.recv_blocking()?;
                if cancelled.load(Ordering::SeqCst) || matches!(answer, Answer::Cancel) {
                    let _ = exchange(&mut stream, Request::CancelSession);
                    return Ok(());
                }
                let response_text = match answer {
                    Answer::Text(mut value) => {
                        anyhow::ensure!(
                            input && value.len() <= 4096,
                            "Invalid authentication answer"
                        );
                        Some(std::mem::take(&mut *value))
                    }
                    Answer::Continue => {
                        anyhow::ensure!(!input, "Answer required");
                        None
                    }
                    Answer::Cancel => return Ok(()),
                    Answer::Start => anyhow::bail!("Unexpected start permission"),
                };
                response = exchange(
                    &mut stream,
                    Request::PostAuthMessageResponse {
                        response: response_text,
                    },
                )?;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(self.0.join("socket"));
            let _ = std::fs::remove_dir(&self.0);
        }
    }
    fn read_request(stream: &mut UnixStream) -> anyhow::Result<Request> {
        let mut len = [0; 4];
        stream.read_exact(&mut len)?;
        let len = u32::from_ne_bytes(len) as usize;
        anyhow::ensure!(len <= 65536, "Bound exceeded");
        let mut bytes = vec![0; len];
        stream.read_exact(&mut bytes)?;
        Ok(serde_json::from_slice(&bytes)?)
    }
    fn respond(stream: &mut UnixStream, response: Response) -> anyhow::Result<()> {
        let bytes = serde_json::to_vec(&response)?;
        stream.write_all(&(bytes.len() as u32).to_ne_bytes())?;
        stream.write_all(&bytes)?;
        Ok(())
    }
    fn conversation(commit: bool) -> anyhow::Result<()> {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let fixture = Fixture(
            std::env::temp_dir().join(format!("asena-greeter-{}-{unique}", std::process::id())),
        );
        std::fs::create_dir(&fixture.0)?;
        let socket = fixture.0.join("socket");
        let listener = std::os::unix::net::UnixListener::bind(&socket)?;
        let peer = std::thread::spawn(move || -> anyhow::Result<()> {
            let (mut stream, _) = listener.accept()?;
            stream.set_read_timeout(Some(Duration::from_secs(3)))?;
            anyhow::ensure!(
                matches!(read_request(&mut stream)?,Request::CreateSession{username} if username=="alice"),
                "Create expected"
            );
            respond(
                &mut stream,
                Response::AuthMessage {
                    auth_message_type: AuthMessageType::Secret,
                    auth_message: "Password".into(),
                },
            )?;
            anyhow::ensure!(
                matches!(read_request(&mut stream)?,Request::PostAuthMessageResponse{response:Some(answer)} if answer=="fixture-answer"),
                "Answer expected"
            );
            respond(
                &mut stream,
                Response::AuthMessage {
                    auth_message_type: AuthMessageType::Info,
                    auth_message: "Fixture information".into(),
                },
            )?;
            anyhow::ensure!(
                matches!(
                    read_request(&mut stream)?,
                    Request::PostAuthMessageResponse { response: None }
                ),
                "Information acknowledgement expected"
            );
            respond(&mut stream, Response::Success)?;
            let request = read_request(&mut stream)?;
            if commit {
                anyhow::ensure!(
                    matches!(request,Request::StartSession{cmd,env} if cmd==vec!["'/usr/bin/true'"]&&env==vec!["XDG_SESSION_TYPE=wayland"]),
                    "Selected argv/environment changed"
                );
            } else {
                anyhow::ensure!(
                    matches!(request, Request::CancelSession),
                    "Cancellation started a session"
                );
            }
            respond(&mut stream, Response::Success)?;
            Ok(())
        });
        let (sender, events) = async_channel::bounded(4);
        let attempt = start(
            socket,
            "alice".into(),
            Session {
                id: "fixture".into(),
                name: "Fixture".into(),
                command: vec!["/usr/bin/true".into()],
                environment: vec!["XDG_SESSION_TYPE=wayland".into()],
            },
            sender,
        )?;
        anyhow::ensure!(
            matches!(events.recv_blocking()?, Event::Prompt(_, true)),
            "Secret prompt expected"
        );
        attempt
            .answers
            .send_blocking(Answer::Text(Zeroizing::new("fixture-answer".into())))?;
        anyhow::ensure!(
            matches!(events.recv_blocking()?, Event::Information(_)),
            "Info expected"
        );
        attempt.answers.send_blocking(Answer::Continue)?;
        anyhow::ensure!(
            matches!(events.recv_blocking()?, Event::Authenticated),
            "Authentication acknowledgement expected"
        );
        attempt.answers.send_blocking(if commit {
            Answer::Start
        } else {
            Answer::Cancel
        })?;
        if commit {
            anyhow::ensure!(
                matches!(events.recv_blocking()?,Event::Started(name) if name=="alice"),
                "Start expected"
            );
        }
        anyhow::ensure!(
            matches!(events.recv_blocking()?, Event::Done),
            "Worker did not finish"
        );
        peer.join().map_err(|_| anyhow::anyhow!("Peer failed"))??;
        Ok(())
    }
    #[test]
    fn multiple_prompts_preserve_the_selected_session_and_wait_for_start_permission(
    ) -> anyhow::Result<()> {
        conversation(true)
    }
    #[test]
    fn cancellation_after_authentication_cannot_start_a_session() -> anyhow::Result<()> {
        conversation(false)
    }
    #[test]
    fn greetds_real_shell_boundary_preserves_literal_arguments() -> anyhow::Result<()> {
        let words = transport_command(vec![
            "/usr/bin/printf".into(),
            "%s\\n".into(),
            "space value".into(),
            "$(printf expanded)".into(),
            "a'b".into(),
        ]);
        let output = std::process::Command::new("/bin/sh")
            .args(["-c", &format!("exec {}", words.join(" "))])
            .output()?;
        assert!(output.status.success());
        assert_eq!(output.stdout, b"space value\n$(printf expanded)\na'b\n");
        Ok(())
    }
    #[test]
    fn an_oversized_frame_is_rejected_before_allocating_payload() -> anyhow::Result<()> {
        let (mut client, mut server) = UnixStream::pair()?;
        let peer = std::thread::spawn(move || -> std::io::Result<()> {
            let mut len = [0; 4];
            server.read_exact(&mut len)?;
            let mut payload = vec![0; u32::from_ne_bytes(len) as usize];
            server.read_exact(&mut payload)?;
            server.write_all(&u32::MAX.to_ne_bytes())
        });
        assert!(exchange(&mut client, Request::CancelSession).is_err());
        peer.join().map_err(|_| anyhow::anyhow!("Peer failed"))??;
        Ok(())
    }
}
