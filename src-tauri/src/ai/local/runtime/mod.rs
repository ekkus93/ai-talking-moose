//! Local llama.cpp runtime ownership boundary.
//!
//! Binding-specific llama.cpp objects belong below this module boundary. Application state,
//! commands, and frontend-facing IPC types own or observe `LocalRuntimeManager`, never raw
//! `llama-cpp-2` model/context/sampler values.

use parking_lot::RwLock;
use std::sync::Arc;

const DEFAULT_CONTEXT_SIZE: u32 = 4_096;
const MAX_DEFAULT_THREADS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalRuntimePhase {
    Ready,
    ShuttingDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalRuntimePolicy {
    context_size: u32,
    thread_count: usize,
}

impl LocalRuntimePolicy {
    fn for_available_parallelism(available_parallelism: usize) -> Self {
        let available_parallelism = available_parallelism.max(1);
        let thread_count = (available_parallelism / 2).max(1).min(MAX_DEFAULT_THREADS);
        Self {
            context_size: DEFAULT_CONTEXT_SIZE,
            thread_count,
        }
    }

    pub(crate) fn context_size(self) -> u32 {
        self.context_size
    }

    pub(crate) fn thread_count(self) -> usize {
        self.thread_count
    }
}

impl Default for LocalRuntimePolicy {
    fn default() -> Self {
        let available_parallelism = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1);
        Self::for_available_parallelism(available_parallelism)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalRuntimeSnapshot {
    pub(crate) phase: LocalRuntimePhase,
    pub(crate) policy: LocalRuntimePolicy,
}

#[derive(Debug)]
struct LocalRuntimeLifecycle {
    phase: LocalRuntimePhase,
}

#[derive(Debug)]
struct LocalRuntimeInner {
    policy: LocalRuntimePolicy,
    lifecycle: RwLock<LocalRuntimeLifecycle>,
}

/// Authoritative owner for the local inference runtime lifecycle.
///
/// The manager is intentionally cheap to clone only through `Arc`. Future P5 work hangs model
/// ownership, generation serialization, cancellation, and diagnostics off this single boundary.
/// Once shutdown starts the transition is irreversible for this manager instance.
#[derive(Debug)]
pub(crate) struct LocalRuntimeManager {
    inner: Arc<LocalRuntimeInner>,
}

impl LocalRuntimeManager {
    pub(crate) fn new() -> Self {
        Self::with_policy(LocalRuntimePolicy::default())
    }

    fn with_policy(policy: LocalRuntimePolicy) -> Self {
        Self {
            inner: Arc::new(LocalRuntimeInner {
                policy,
                lifecycle: RwLock::new(LocalRuntimeLifecycle {
                    phase: LocalRuntimePhase::Ready,
                }),
            }),
        }
    }

    pub(crate) fn snapshot(&self) -> LocalRuntimeSnapshot {
        LocalRuntimeSnapshot {
            phase: self.inner.lifecycle.read().phase,
            policy: self.inner.policy,
        }
    }

    pub(crate) fn begin_shutdown(&self) {
        self.inner.lifecycle.write().phase = LocalRuntimePhase::ShuttingDown;
    }
}

impl Default for LocalRuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_policy_is_cpu_conservative_and_bounded() {
        let one = LocalRuntimePolicy::for_available_parallelism(1);
        assert_eq!(one.context_size(), DEFAULT_CONTEXT_SIZE);
        assert_eq!(one.thread_count(), 1);

        assert_eq!(
            LocalRuntimePolicy::for_available_parallelism(4).thread_count(),
            2
        );
        assert_eq!(
            LocalRuntimePolicy::for_available_parallelism(16).thread_count(),
            MAX_DEFAULT_THREADS
        );
        assert_eq!(
            LocalRuntimePolicy::for_available_parallelism(128).thread_count(),
            MAX_DEFAULT_THREADS
        );
    }

    #[test]
    fn shutdown_transition_is_irreversible_and_idempotent() {
        let manager = LocalRuntimeManager::with_policy(
            LocalRuntimePolicy::for_available_parallelism(8),
        );
        assert_eq!(manager.snapshot().phase, LocalRuntimePhase::Ready);

        manager.begin_shutdown();
        manager.begin_shutdown();

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.phase, LocalRuntimePhase::ShuttingDown);
        assert_eq!(snapshot.policy.context_size(), DEFAULT_CONTEXT_SIZE);
        assert_eq!(snapshot.policy.thread_count(), 4);
    }
}
