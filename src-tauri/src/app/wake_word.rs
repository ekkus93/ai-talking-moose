//! Authoritative Wake Word V1 subsystem facade.
//!
//! WWR-020 consolidates ownership here while retaining the strongest existing
//! component implementations in their original files. Production code must
//! import Wake Word components through this module rather than through the
//! underlying implementation paths.

// These re-exports establish the canonical boundary before later remediation
// slices wire the runtime into production. Until then, non-test builds do not
// consume every facade component.
#![allow(unused_imports)]

pub(crate) use crate::wake_word_policy as policy;
pub(crate) use super::wake_word_engine as engine;
pub(crate) use super::wake_word_settings as settings;
pub(crate) use crate::asr::wake_word_diagnostics as diagnostics;
pub(crate) use crate::asr::wake_word_handoff as handoff;
pub(crate) use crate::asr::wake_word_runtime as runtime;
pub(crate) use crate::asr::wake_word_sherpa_manifest as manifest;

#[cfg(test)]
mod architecture_tests {
    #[test]
    fn legacy_duplicate_runtime_and_engine_modules_are_not_compiled() {
        let app_mod = include_str!("mod.rs");
        let asr_mod = include_str!("../asr/mod.rs");
        assert!(!app_mod
            .lines()
            .any(|line| line.contains("mod wake_word_runtime")));
        assert!(!asr_mod
            .lines()
            .any(|line| line.contains("mod wake_word_sherpa;")));
    }

    #[test]
    fn canonical_facade_exposes_one_manager_and_one_engine_policy() {
        let _manager = super::runtime::WakeWordRuntimeManager::new();
        let config = super::engine::SherpaKwsConfig::default();
        config.validate().unwrap();
        assert_eq!(config.threads, super::policy::V1_KWS_THREADS);
        assert_eq!(super::settings::DEFAULT_WAKE_PHRASE, "Hey, Moose");
        assert_eq!(super::diagnostics::WAKE_WORD_ENGINE_ID, "sherpa-onnx-kws");
        assert_eq!(
            super::handoff::WAKE_ASR_LIVE_HANDOFF_CAPACITY_SAMPLES,
            super::policy::V1_PRE_ROLL_SAMPLES
        );
        assert_eq!(
            super::manifest::SHERPA_KWS_REQUIRED_FILES,
            super::engine::V1_KWS_REQUIRED_MODEL_FILES
        );
        assert!(super::manifest::V1_SHERPA_KWS_MODEL_MANIFEST
            .validate()
            .is_err());
    }
}
