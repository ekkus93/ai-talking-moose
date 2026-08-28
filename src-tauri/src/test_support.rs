use std::cell::Cell;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

const LOG_CAPTURE_LIVENESS_MARKER: &str = "TALKING_MOOSE_TEST_LOG_CAPTURE_LIVE";
static LOG_CAPTURE_LOCK: Mutex<()> = Mutex::new(());

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
    // `tracing` caches callsite interest process-wide while `with_default` installs
    // the formatter only for the current thread. Parallel capture subscribers can
    // therefore make an otherwise-live callsite disappear from another test. Keep
    // every repository log-capture test behind one lock so the cache is rebuilt
    // against exactly one capture subscriber at a time. Recover poisoning so a
    // failed assertion does not cascade into unrelated privacy tests.
    let _capture_guard = LOG_CAPTURE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let captured = CapturedLogs::default();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(captured.clone())
        .finish();
    let output = tracing::subscriber::with_default(subscriber, || {
        tracing::trace!("TALKING_MOOSE_TEST_LOG_CAPTURE_LIVE");
        run()
    });
    let logs = captured.text();
    assert_log_capture_live(&logs);
    (output, logs)
}

pub(crate) fn assert_log_capture_live(logs: &str) {
    assert!(
        logs.contains(LOG_CAPTURE_LIVENESS_MARKER),
        "tracing capture was not live; privacy assertions would be vacuous"
    );
}

#[cfg(test)]
mod log_capture_tests {
    use super::{assert_log_capture_live, capture_logs};

    #[test]
    fn capture_logs_proves_its_liveness() {
        let (_, logs) = capture_logs(|| ());
        assert_log_capture_live(&logs);
    }

    #[test]
    fn liveness_assertion_rejects_an_empty_capture() {
        let result = std::panic::catch_unwind(|| assert_log_capture_live(""));
        assert!(result.is_err());
    }
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
