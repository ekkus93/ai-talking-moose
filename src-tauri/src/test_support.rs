use std::cell::Cell;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
pub(crate) struct CapturedLogs(Arc<Mutex<Vec<u8>>>);

impl CapturedLogs {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl Write for CapturedLogs {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for CapturedLogs {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

pub(crate) fn capture_logs<T>(run: impl FnOnce() -> T) -> (T, String) {
    let captured = CapturedLogs::default();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(captured.clone())
        .finish();
    let output = tracing::subscriber::with_default(subscriber, run);
    (output, captured.text())
}
thread_local! {
    static NETWORK_DENY_DEPTH: Cell<u32> = const { Cell::new(0) };
}

pub(crate) struct NetworkDenyGuard;

impl Drop for NetworkDenyGuard {
    fn drop(&mut self) {
        NETWORK_DENY_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

/// Deny production network boundaries on the current test executor thread.
///
/// Provider methods consult this immediately before their first HTTP/WebSocket
/// operation. The guard is test-only instrumentation of those real paths; it is
/// deliberately thread-local so unrelated parallel tests remain unaffected.
pub(crate) fn deny_network_for_scope() -> NetworkDenyGuard {
    NETWORK_DENY_DEPTH.with(|depth| depth.set(depth.get().saturating_add(1)));
    NetworkDenyGuard
}

pub(crate) fn network_denied() -> bool {
    NETWORK_DENY_DEPTH.with(|depth| depth.get() > 0)
}
