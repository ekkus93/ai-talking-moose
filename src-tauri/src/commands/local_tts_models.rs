use crate::ai::local_tts::installer::{
    global_local_tts_installer, LocalTtsInstallErrorKind, LocalTtsInstallProgress,
};
use crate::ai::local_tts::manifest::{local_tts_model_manifest, LocalTtsPlatform};
use crate::ai::local_tts::storage::LocalTtsInstallState;
use crate::ai::local_tts::LOCAL_TTS_MODEL_IDS;
use crate::app::state::AppState;
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

fn descriptor(
    model_id: &str,
    selected_model_id: &str,
) -> Result<LocalTtsModelDescriptor, String> {
    let platform = current_platform()?;
    let installer = global_local_tts_installer().map_err(|error| error.to_string())?;
    let manifest = local_tts_model_manifest(model_id)
        .ok_or_else(|| "The selected Local TTS model is not in the supported catalog.".to_string())?;
    let status = installer
        .status(model_id, platform)
        .map_err(|error| error.to_string())?;
    let error = status.error.as_ref().map(|error| LocalTtsModelError {
        // Status-level failures are deliberately collapsed to the safe corrupt-install
        // category here. KTT-600 diagnostics owns richer installer/runtime error composition.
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
