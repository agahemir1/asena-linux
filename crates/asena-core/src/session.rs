//! Route user-bus requests to the intended graphical session, never another one.
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("The graphical session could not be identified")]
pub struct UnknownSession;
pub fn current_scope() -> Result<String, UnknownSession> {
    for_display(&PathBuf::from(
        std::env::var_os("WAYLAND_DISPLAY").ok_or(UnknownSession)?,
    ))
}
pub fn for_display(display: &Path) -> Result<String, UnknownSession> {
    identity(
        &PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").ok_or(UnknownSession)?),
        display,
        &std::env::var_os("XDG_SESSION_ID").unwrap_or_default(),
    )
}
fn identity(runtime: &Path, display: &Path, session: &OsStr) -> Result<String, UnknownSession> {
    if !runtime.is_absolute() || display.as_os_str().is_empty() {
        return Err(UnknownSession);
    }
    let socket = if display.is_absolute() {
        display.to_path_buf()
    } else {
        runtime.join(display)
    };
    if socket.as_os_str().as_encoded_bytes().len() > 256 || session.as_encoded_bytes().len() > 128 {
        return Err(UnknownSession);
    }
    // Encode OS bytes losslessly. This is a routing identity, not a credential.
    fn hex(value: &OsStr) -> String {
        let digits = b"0123456789abcdef";
        value
            .as_encoded_bytes()
            .iter()
            .flat_map(|byte| {
                [
                    digits[(byte >> 4) as usize] as char,
                    digits[(byte & 15) as usize] as char,
                ]
            })
            .collect()
    }
    Ok(format!("v1:{}:{}", hex(socket.as_os_str()), hex(session)))
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    #[test]
    fn display_and_login_session_both_distinguish_lock_owners() {
        let root = Path::new("/run/user/1000");
        let display = Path::new("wayland-0");
        let first = identity(root, display, OsStr::new("c1"));
        assert_ne!(
            first,
            identity(root, Path::new("wayland-1"), OsStr::new("c1"))
        );
        assert_ne!(first, identity(root, display, OsStr::new("c2")));
        assert_eq!(
            first,
            identity(
                root,
                Path::new("/run/user/1000/wayland-0"),
                OsStr::new("c1")
            )
        );
    }
    #[test]
    fn invalid_roots_and_lossy_os_names_cannot_alias() {
        assert!(identity(
            Path::new("relative"),
            Path::new("wayland-0"),
            OsStr::new("")
        )
        .is_err());
        use std::os::unix::ffi::OsStringExt;
        let first = OsString::from_vec(vec![b'w', 0xfe]);
        let second = OsString::from_vec(vec![b'w', 0xff]);
        assert_ne!(
            identity(Path::new("/tmp"), Path::new(&first), OsStr::new("")),
            identity(Path::new("/tmp"), Path::new(&second), OsStr::new(""))
        );
    }
}
