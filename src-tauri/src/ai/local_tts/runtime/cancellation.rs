use super::LocalTtsRuntimeError;
use ort::session::RunOptions;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Default)]
struct LocalTtsRuntimeCancellationInner {
    cancelled: AtomicBool,
    run_options: Mutex<Option<Arc<RunOptions>>>,
}

#[derive(Clone, Default)]
pub(in crate::ai::local_tts) struct LocalTtsRuntimeCancellation {
    inner: Arc<LocalTtsRuntimeCancellationInner>,
}

impl LocalTtsRuntimeCancellation {
    pub(super) fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        let run_options = self.inner.run_options.lock().clone();
        if let Some(run_options) = run_options {
            let _ = run_options.terminate();
        }
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    pub(super) fn check_cancelled(&self) -> Result<(), LocalTtsRuntimeError> {
        if self.is_cancelled() {
            Err(LocalTtsRuntimeError::cancelled())
        } else {
            Ok(())
        }
    }

    pub(super) fn install_run_options(&self, run_options: Arc<RunOptions>) {
        *self.inner.run_options.lock() = Some(run_options.clone());
        if self.is_cancelled() {
            let _ = run_options.terminate();
        }
    }

    pub(super) fn clear_run_options(&self) {
        *self.inner.run_options.lock() = None;
    }

    #[cfg(test)]
    pub(super) fn has_run_options(&self) -> bool {
        self.inner.run_options.lock().is_some()
    }
}
