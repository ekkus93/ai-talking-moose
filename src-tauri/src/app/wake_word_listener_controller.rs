use super::wake_word_local_listener_thread::{WakeLocalListenerEvent, WakeLocalListenerHandle};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub(super) struct NativeWakeListenerConfig {
    pub(super) app_data_dir: PathBuf,
    pub(super) event_tx: mpsc::UnboundedSender<WakeLocalListenerEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeWakeListenerLifecyclePhase {
    Stopped,
    Starting,
    Running,
    Stopping,
}

enum NativeWakeListenerLifecycle {
    Stopped,
    Starting {
        generation: u64,
    },
    Running {
        generation: u64,
        handle: WakeLocalListenerHandle,
    },
    Stopping {
        generation: u64,
    },
}

struct NativeWakeListenerControllerInner {
    next_generation: u64,
    lifecycle: NativeWakeListenerLifecycle,
    pending_start_generation: Option<u64>,
    pending_restart_generation: Option<u64>,
    config: Option<NativeWakeListenerConfig>,
}

pub(super) enum NativeWakeListenerStopPlan {
    AlreadyStopped,
    AlreadyStopping,
    AwaitingStart,
    Supervise {
        generation: u64,
        handle: WakeLocalListenerHandle,
    },
}

impl Default for NativeWakeListenerControllerInner {
    fn default() -> Self {
        Self {
            next_generation: 0,
            lifecycle: NativeWakeListenerLifecycle::Stopped,
            pending_start_generation: None,
            pending_restart_generation: None,
            config: None,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct NativeWakeListenerController {
    inner: Arc<Mutex<NativeWakeListenerControllerInner>>,
}

impl NativeWakeListenerController {
    pub(super) fn configure(
        &self,
        app_data_dir: &Path,
        event_tx: &mpsc::UnboundedSender<WakeLocalListenerEvent>,
    ) {
        self.inner.lock().config = Some(NativeWakeListenerConfig {
            app_data_dir: app_data_dir.to_path_buf(),
            event_tx: event_tx.clone(),
        });
    }

    pub(super) fn config(&self) -> Option<NativeWakeListenerConfig> {
        self.inner.lock().config.clone()
    }

    pub(super) fn schedule_restart(&self) -> u64 {
        let mut inner = self.inner.lock();
        inner.next_generation = inner.next_generation.wrapping_add(1).max(1);
        let generation = inner.next_generation;
        inner.pending_restart_generation = Some(generation);
        generation
    }

    pub(super) fn restart_request_is_current(&self, generation: u64) -> bool {
        self.inner.lock().pending_restart_generation == Some(generation)
    }

    pub(super) fn cancel_restart_request(&self, generation: u64) {
        let mut inner = self.inner.lock();
        if inner.pending_restart_generation == Some(generation) {
            inner.pending_restart_generation = None;
        }
    }

    pub(super) fn cancel_all_restart_requests(&self) {
        self.inner.lock().pending_restart_generation = None;
    }

    pub(super) fn claim_scheduled_restart(&self, generation: u64) -> Option<u64> {
        let mut inner = self.inner.lock();
        if inner.pending_restart_generation != Some(generation)
            || !matches!(inner.lifecycle, NativeWakeListenerLifecycle::Stopped)
        {
            return None;
        }

        inner.pending_restart_generation = None;
        inner.pending_start_generation = Some(generation);
        inner.lifecycle = NativeWakeListenerLifecycle::Starting { generation };
        Some(generation)
    }

    pub(super) fn reserve_start(&self) -> Option<u64> {
        let mut inner = self.inner.lock();
        if !matches!(inner.lifecycle, NativeWakeListenerLifecycle::Stopped) {
            return None;
        }
        inner.next_generation = inner.next_generation.wrapping_add(1).max(1);
        let generation = inner.next_generation;
        // An explicit start supersedes any deferred restart request that has not claimed the
        // lifecycle yet.
        inner.pending_restart_generation = None;
        inner.pending_start_generation = Some(generation);
        inner.lifecycle = NativeWakeListenerLifecycle::Starting { generation };
        Some(generation)
    }

    pub(super) fn cancel_start(&self, generation: u64) {
        let mut inner = self.inner.lock();
        if inner.pending_start_generation != Some(generation) {
            return;
        }

        inner.pending_start_generation = None;
        if matches!(
            inner.lifecycle,
            NativeWakeListenerLifecycle::Starting { generation: current } if current == generation
        ) || matches!(
            inner.lifecycle,
            NativeWakeListenerLifecycle::Stopping { .. }
        ) {
            inner.lifecycle = NativeWakeListenerLifecycle::Stopped;
        }
    }

    pub(super) fn publish_start(
        &self,
        generation: u64,
        handle: WakeLocalListenerHandle,
    ) -> Result<(), (WakeLocalListenerHandle, Option<u64>)> {
        let mut inner = self.inner.lock();
        if matches!(
            inner.lifecycle,
            NativeWakeListenerLifecycle::Starting { generation: current } if current == generation
        ) {
            inner.pending_start_generation = None;
            inner.lifecycle = NativeWakeListenerLifecycle::Running { generation, handle };
            return Ok(());
        }

        let stopping_generation = if inner.pending_start_generation == Some(generation) {
            inner.pending_start_generation = None;
            match inner.lifecycle {
                NativeWakeListenerLifecycle::Stopping { generation } => Some(generation),
                _ => None,
            }
        } else {
            None
        };
        Err((handle, stopping_generation))
    }

    pub(super) fn begin_stop(&self) -> NativeWakeListenerStopPlan {
        let mut inner = self.inner.lock();
        match inner.lifecycle {
            NativeWakeListenerLifecycle::Stopped => {
                return NativeWakeListenerStopPlan::AlreadyStopped;
            }
            NativeWakeListenerLifecycle::Stopping { .. } => {
                return NativeWakeListenerStopPlan::AlreadyStopping;
            }
            NativeWakeListenerLifecycle::Starting { .. }
            | NativeWakeListenerLifecycle::Running { .. } => {}
        }

        inner.next_generation = inner.next_generation.wrapping_add(1).max(1);
        let stop_generation = inner.next_generation;
        let previous = std::mem::replace(
            &mut inner.lifecycle,
            NativeWakeListenerLifecycle::Stopping {
                generation: stop_generation,
            },
        );
        match previous {
            NativeWakeListenerLifecycle::Running { handle, .. } => {
                inner.pending_start_generation = None;
                NativeWakeListenerStopPlan::Supervise {
                    generation: stop_generation,
                    handle,
                }
            }
            NativeWakeListenerLifecycle::Starting { .. } => {
                NativeWakeListenerStopPlan::AwaitingStart
            }
            NativeWakeListenerLifecycle::Stopped | NativeWakeListenerLifecycle::Stopping { .. } => {
                unreachable!()
            }
        }
    }

    pub(super) fn finish_stop(&self, generation: u64) {
        let mut inner = self.inner.lock();
        if matches!(
            inner.lifecycle,
            NativeWakeListenerLifecycle::Stopping { generation: current } if current == generation
        ) {
            inner.pending_start_generation = None;
            inner.lifecycle = NativeWakeListenerLifecycle::Stopped;
        }
    }

    pub(super) fn phase(&self) -> NativeWakeListenerLifecyclePhase {
        let inner = self.inner.lock();
        match &inner.lifecycle {
            NativeWakeListenerLifecycle::Stopped => NativeWakeListenerLifecyclePhase::Stopped,
            NativeWakeListenerLifecycle::Starting { generation } => {
                let _ = generation;
                NativeWakeListenerLifecyclePhase::Starting
            }
            NativeWakeListenerLifecycle::Running { generation, .. } => {
                let _ = generation;
                NativeWakeListenerLifecyclePhase::Running
            }
            NativeWakeListenerLifecycle::Stopping { generation } => {
                let _ = generation;
                NativeWakeListenerLifecyclePhase::Stopping
            }
        }
    }

    pub(super) fn is_running(&self) -> bool {
        matches!(self.phase(), NativeWakeListenerLifecyclePhase::Running)
    }

    #[cfg(test)]
    pub(super) fn install_running_for_test(
        &self,
        generation: u64,
        handle: WakeLocalListenerHandle,
    ) {
        let mut inner = self.inner.lock();
        inner.next_generation = inner.next_generation.max(generation);
        inner.pending_start_generation = None;
        inner.lifecycle = NativeWakeListenerLifecycle::Running { generation, handle };
    }
}
