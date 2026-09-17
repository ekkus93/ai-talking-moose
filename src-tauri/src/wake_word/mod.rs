//! Authoritative Wake Word V1 subsystem boundary.
//!
//! Wake Word is an application capability, not a command-ASR provider.  The
//! implementation is being migrated behind this module so production callers
//! have one stable ownership boundary while the legacy `app`/`asr` paths remain
//! compatibility re-exports during remediation.

pub use crate::app::wake_word_settings as settings;
pub use crate::asr::wake_word_diagnostics as diagnostics;
pub use crate::asr::wake_word_handoff as handoff;
pub use crate::asr::wake_word_runtime as runtime;
pub use crate::asr::wake_word_sherpa as engine;
pub use crate::asr::wake_word_sherpa_manifest as manifest;

pub use diagnostics::WakeWordDiagnostics;
pub use engine::{NativeKwsSession, SherpaKwsConfig, SherpaKwsEngine};
pub use handoff::WakeAsrHandoff;
pub use runtime::{WakeWordRuntimeManager, WakeWordRuntimePhase, WakeWordRuntimeSnapshot};
pub use settings::WakeWordSettings;

#[cfg(test)]
mod architecture_tests {
    use super::*;

    #[test]
    fn canonical_boundary_exposes_one_runtime_and_engine_stack() {
        let manager = WakeWordRuntimeManager::new();
        assert_eq!(
            manager.snapshot(std::time::Instant::now()).phase,
            WakeWordRuntimePhase::Disabled
        );

        let _settings = WakeWordSettings::default();
        let _handoff = WakeAsrHandoff::new(Vec::new());
    }

    #[test]
    fn legacy_app_modules_are_compatibility_reexports_not_duplicate_owners() {
        let runtime_source = include_str!("../app/wake_word_runtime.rs");
        let engine_source = include_str!("../app/wake_word_engine.rs");
        assert!(!runtime_source.contains("struct WakeWordRuntimeManager"));
        assert!(!engine_source.contains("struct SherpaKwsEngine"));
        assert!(!engine_source.contains("trait SherpaKwsEngine"));
    }
}
