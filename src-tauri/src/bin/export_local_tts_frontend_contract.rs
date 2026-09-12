use serde::Serialize;
use talking_moose_lib::ai::local_tts::installer::LocalTtsInstallErrorKind;
use talking_moose_lib::ai::local_tts::runtime::{
    LocalTtsRuntimeErrorKind, LocalTtsRuntimePhase, LocalTtsRuntimeStatus,
};
use talking_moose_lib::ai::local_tts::storage::LocalTtsInstallState;
use talking_moose_lib::ai::types::TtsProvider;
use talking_moose_lib::commands::LocalTtsDiagnostics;

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct LocalTtsFrontendContract {
    local_tts_diagnostics: LocalTtsDiagnostics,
    local_tts_runtime_status: LocalTtsRuntimeStatus,
}

fn representative_diagnostics() -> LocalTtsDiagnostics {
    let model_id = "KittenML/kitten-tts-mini-0.8".to_string();
    LocalTtsDiagnostics {
        provider: TtsProvider::Local,
        selected_model_id: model_id.clone(),
        selected_voice_id: "Jasper".to_string(),
        install_state: LocalTtsInstallState::Installed,
        expected_bytes: 256,
        installed_bytes: Some(256),
        installer_error_category: Some(LocalTtsInstallErrorKind::Network),
        installer_error_retryable: Some(true),
        runtime: LocalTtsRuntimeStatus {
            selected_model_id: model_id.clone(),
            loaded_model_id: Some(model_id),
            loaded_revision: Some("contract-local-tts-revision".to_string()),
            runtime_compatibility_version: Some(1),
            phase: LocalTtsRuntimePhase::Ready,
            sample_rate_hz: Some(24_000),
            inference_thread_count: Some(2),
            last_model_load_duration_ms: Some(125),
            last_synthesis_duration_ms: Some(50),
            last_generated_audio_duration_ms: Some(500.0),
            last_real_time_factor: Some(0.1),
            last_error_category: Some(LocalTtsRuntimeErrorKind::Inference),
        },
    }
}

fn main() {
    let diagnostics = representative_diagnostics();
    let contract = LocalTtsFrontendContract {
        local_tts_runtime_status: diagnostics.runtime.clone(),
        local_tts_diagnostics: diagnostics,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&contract)
            .expect("Local TTS frontend contract must serialize")
    );
}
