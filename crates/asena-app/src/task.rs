//! Asynchronous task execution helper for Asena applications.

/// Spawns a closure on a worker thread and dispatches the result to the main GTK thread.
///
/// This ensures the UI thread is never blocked during IO, compression, or network calls.
pub fn spawn_background<T, F, C>(work: F, callback: C)
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
    C: FnOnce(Result<T, String>) + 'static,
{
    let (sender, receiver) = async_channel::bounded(1);

    gio::spawn_blocking(move || {
        let result = work();
        let _ = sender.send_blocking(result);
    });

    glib::spawn_future_local(async move {
        if let Ok(result) = receiver.recv().await {
            callback(result);
        }
    });
}
