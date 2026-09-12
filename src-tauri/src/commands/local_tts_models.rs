use crate::ai::local_tts::installer::{
    global_local_tts_installer, LocalTtsInstallErrorKind, LocalTtsInstallProgress,
};
use crate::ai::local_tts::manifest::{local_tts_model_manifest, LocalTtsPlatform};
use crate::ai::local_tts::runtime::LocalTtsRuntimeStatus;
use crate::ai::local_tts::storage::LocalTtsInstallState;
use crate::ai::local_tts::{validate_local_tts_voice, LOCAL_TTS_MODEL_IDS};
use crate::ai::types::TtsProvider;
use crate::app::state::AppState;
use crate::commands::character::VOICE_AUDITION_SCRIPT;
use crate::commands::speech::{
    invoke_standalone_speech_for_provider, schedule_standalone_completion,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Emitter, Runtime, State};

pub const LOCAL_TTS_MODEL_PROGRESS_EVENT: &str = "moose://local-tts/model-progress";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalTtsModelError {
    pub kind: LocalTtsInstallErrorKind,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalTtsModelDescriptor {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub expected_bytes: u64,
    pub installed_bytes: Option<u64>,
    pub license: String,
    pub install_state: LocalTtsInstallState,
    pub active: bool,
    pub error: Option<LocalTtsModelError>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LocalTtsDiagnostics {
    pub provider: TtsProvider,
    pub selected_model_id: String,
    pub selected_voice_id: String,
    pub install_state: LocalTtsInstallState,
    pub expected_bytes: u64,
    pub installed_bytes: Option<u64>,
    pub installer_error_category: Option<LocalTtsInstallErrorKind>,
    pub installer_error_retryable: Option<bool>,
    pub runtime: LocalTtsRuntimeStatus,
}

fn current_platform() -> Result<LocalTtsPlatform, String> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Ok(LocalTtsPlatform::LinuxX86_64);
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok(LocalTtsPlatform::MacosArm64);
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Ok(LocalTtsPlatform::MacosX86_64);
    }
    #[allow(unreachable_code)]
    Err("Local TTS is not supported on this platform.".to_string())
}

fn descriptor(model_id: &str, selected_model_id: &str) -> Result<LocalTtsModelDescriptor, String> {
    let platform = current_platform()?;
    let installer = global_local_tts_installer().map_err(|error| error.to_string())?;
    let manifest = local_tts_model_manifest(model_id).ok_or_else(|| {
        "The selected Local TTS model is not in the supported catalog.".to_string()
    })?;
    let status = installer
        .status(model_id, platform)
        .map_err(|error| error.to_string())?;
    let error = status.error.as_ref().map(|error| LocalTtsModelError {
        // Status-level failures are deliberately collapsed to the safe corrupt-install
        // category here. The composed diagnostics response below exposes the exact recorded
        // installer category when one exists for the selected model.
        kind: LocalTtsInstallErrorKind::CorruptInstall,
        message: error.message.clone(),
        retryable: error.retryable,
    });

    Ok(LocalTtsModelDescriptor {
        id: manifest.provider_model_id.to_string(),
        display_name: manifest.display_name.to_string(),
        version: manifest.version.to_string(),
        expected_bytes: status.expected_bytes,
        installed_bytes: status.installed_bytes,
        license: manifest.license.to_string(),
        install_state: status.install_state,
        active: selected_model_id == manifest.provider_model_id,
        error,
    })
}

#[tauri::command]
pub fn get_local_tts_models(
    state: State<'_, AppState>,
) -> Result<Vec<LocalTtsModelDescriptor>, String> {
    let selected_model_id = state.settings.read().local_tts_model.clone();
    LOCAL_TTS_MODEL_IDS
        .iter()
        .map(|model_id| descriptor(model_id, &selected_model_id))
        .collect()
}

#[tauri::command]
pub fn get_local_tts_diagnostics(
    state: State<'_, AppState>,
) -> Result<LocalTtsDiagnostics, String> {
    // Read provider/model/voice as one coherent settings snapshot. Installer/runtime state may
    // advance after this instant, but identities within one response can never come from different
    // settings revisions.
    let (provider, selected_model_id, selected_voice_id) = {
        let settings = state.settings.read();
        (
            settings.tts_provider.clone(),
            settings.local_tts_model.clone(),
            settings.local_tts_voice.clone(),
        )
    };

    let platform = current_platform()?;
    let installer = global_local_tts_installer().map_err(|error| error.to_string())?;
    let status = installer
        .status(&selected_model_id, platform)
        .map_err(|error| error.to_string())?;
    let recorded_error = installer.error_for_model(&selected_model_id);
    let (installer_error_category, installer_error_retryable) = if let Some(error) = recorded_error
    {
        (Some(error.kind), Some(error.retryable))
    } else if let Some(error) = status.error.as_ref() {
        // Storage-level corruption can exist without a recorded install operation. Keep that
        // category conservative and typed instead of forwarding an arbitrary error string.
        (
            Some(LocalTtsInstallErrorKind::CorruptInstall),
            Some(error.retryable),
        )
    } else {
        (None, None)
    };
    let runtime = state.local_tts_runtime.status(selected_model_id.clone());

    Ok(LocalTtsDiagnostics {
        provider,
        selected_model_id,
        selected_voice_id,
        install_state: status.install_state,
        expected_bytes: status.expected_bytes,
        installed_bytes: status.installed_bytes,
        installer_error_category,
        installer_error_retryable,
        runtime,
    })
}

#[tauri::command]
pub async fn install_local_tts_model<R: Runtime>(
    model_id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<LocalTtsModelDescriptor, String> {
    let platform = current_platform()?;
    let installer = global_local_tts_installer().map_err(|error| error.to_string())?;
    let progress_app = app.clone();
    let progress = Arc::new(move |progress: LocalTtsInstallProgress| {
        let _ = progress_app.emit(LOCAL_TTS_MODEL_PROGRESS_EVENT, progress);
    });
    installer
        .install(&model_id, platform, Some(progress))
        .await
        .map_err(|error| error.to_string())?;

    let selected_model_id = state.settings.read().local_tts_model.clone();
    descriptor(&model_id, &selected_model_id)
}

#[tauri::command]
pub fn cancel_local_tts_install(model_id: String) -> Result<bool, String> {
    let installer = global_local_tts_installer().map_err(|error| error.to_string())?;
    Ok(installer.cancel(&model_id))
}

#[tauri::command]
pub async fn delete_local_tts_model(
    model_id: String,
    state: State<'_, AppState>,
) -> Result<LocalTtsModelDescriptor, String> {
    let installer = global_local_tts_installer().map_err(|error| error.to_string())?;
    state
        .local_tts_runtime
        .delete_model(installer, model_id.clone())
        .await
        .map_err(|error| error.to_string())?;

    let selected_model_id = state.settings.read().local_tts_model.clone();
    descriptor(&model_id, &selected_model_id)
}

#[tauri::command]
pub async fn audition_tts_voice<R: Runtime>(
    provider: TtsProvider,
    voice_name: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    match provider {
        TtsProvider::Google => crate::ai::google::validate_tts_voice(&voice_name)?,
        TtsProvider::Local => {
            validate_local_tts_voice(&voice_name)?;
            let model_id = state.settings.read().local_tts_model.clone();
            // Local audition is a real runtime readiness check, not just a catalog label check.
            // The runtime performs the KTT-204 install verification before loading.
            state
                .local_tts_runtime
                .ensure_loaded(&model_id)
                .await
                .map_err(|error| error.to_string())?;
        }
    }

    let playback = invoke_standalone_speech_for_provider(
        state.inner(),
        &app,
        VOICE_AUDITION_SCRIPT,
        provider,
        Some(voice_name),
    )
    .await?;
    schedule_standalone_completion(state.character_state.clone(), app.clone(), playback);
    Ok(VOICE_AUDITION_SCRIPT.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::local_tts::runtime::{LocalTtsRuntimePhase, LocalTtsRuntimeStatus};

    #[test]
    fn diagnostics_serialization_exposes_safe_identity_without_sensitive_payload_slots() {
        const SENTINEL: &str = "KCR130_COMMAND_SENTINEL_2C14F7A9";
        let model_id = "KittenML/kitten-tts-mini-0.8".to_string();
        let diagnostics = LocalTtsDiagnostics {
            provider: TtsProvider::Local,
            selected_model_id: model_id.clone(),
            selected_voice_id: "Jasper".to_string(),
            install_state: LocalTtsInstallState::Installed,
            expected_bytes: 256,
            installed_bytes: Some(256),
            installer_error_category: None,
            installer_error_retryable: None,
            runtime: LocalTtsRuntimeStatus {
                selected_model_id: model_id.clone(),
                loaded_model_id: Some(model_id.clone()),
                loaded_revision: Some("test-revision".to_string()),
                runtime_compatibility_version: Some(1),
                phase: LocalTtsRuntimePhase::Ready,
                sample_rate_hz: Some(24_000),
                inference_thread_count: Some(2),
                last_model_load_duration_ms: Some(10),
                last_synthesis_duration_ms: Some(20),
                last_generated_audio_duration_ms: Some(100.0),
                last_real_time_factor: Some(0.2),
                last_error_category: None,
            },
        };

        let json = serde_json::to_string(&diagnostics).unwrap();
        assert!(json.contains(&model_id));
        assert!(json.contains("Jasper"));
        assert!(json.contains("\"phase\":\"ready\""));
        assert!(!json.contains(SENTINEL));
        assert!(!json.contains("utterance"));
        assert!(!json.contains("pcm"));
        assert!(!json.contains("credential"));
        assert!(!json.contains("path"));
    }
}
