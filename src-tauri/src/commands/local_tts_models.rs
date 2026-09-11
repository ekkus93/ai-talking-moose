use crate::ai::local_tts::installer::{
    global_local_tts_installer, LocalTtsInstallProgress, LocalTtsInstallProgressCallback,
};
use crate::ai::local_tts::manifest::{
    local_tts_model_manifest, LocalTtsModelManifest, LocalTtsPlatform,
};
use crate::ai::local_tts::storage::{LocalTtsInstallState, LocalTtsStatusError};
use crate::ai::local_tts::DEFAULT_LOCAL_TTS_MODEL_ID;
use crate::app::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Emitter, Runtime, State};

const LOCAL_TTS_PROGRESS_EVENT: &str = "moose://local-tts/model-progress";

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
    pub error: Option<LocalTtsStatusError>,
}

fn safe_installer_error(error: impl std::fmt::Display) -> String {
    error.to_string()
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

fn descriptor_for_manifest(
    manifest: &LocalTtsModelManifest,
    selected_model_id: &str,
) -> Result<LocalTtsModelDescriptor, String> {
    let status = global_local_tts_installer()
        .map_err(safe_installer_error)?
        .status(manifest.provider_model_id, current_platform()?)
        .map_err(safe_installer_error)?;
    Ok(LocalTtsModelDescriptor {
        id: manifest.provider_model_id.to_string(),
        display_name: manifest.display_name.to_string(),
        version: manifest.version.to_string(),
        expected_bytes: status.expected_bytes,
        installed_bytes: status.installed_bytes,
        license: manifest.license.to_string(),
        install_state: status.install_state,
        active: selected_model_id == manifest.provider_model_id,
        error: status.error,
    })
}

fn descriptor_for_selected(state: &AppState, model_id: &str) -> Result<LocalTtsModelDescriptor, String> {
    let selected_model_id = state.settings.read().local_tts_model.clone();
    let manifest = local_tts_model_manifest(model_id)
        .ok_or_else(|| "The selected Local TTS model is not in the supported catalog.".to_string())?;
    descriptor_for_manifest(manifest, &selected_model_id)
}

#[tauri::command]
pub async fn get_local_tts_models(
    state: State<'_, AppState>,
) -> Result<Vec<LocalTtsModelDescriptor>, String> {
    let selected_model_id = state.settings.read().local_tts_model.clone();
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID)
        .ok_or_else(|| "The bundled Local TTS catalog is invalid.".to_string())?;
    Ok(vec![descriptor_for_manifest(manifest, &selected_model_id)?])
}

#[tauri::command]
pub async fn install_local_tts_model<R: Runtime>(
    model_id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<LocalTtsModelDescriptor, String> {
    let installer = global_local_tts_installer().map_err(safe_installer_error)?;
    // A reinstall must not race a warm runtime that still owns the old artifact identity.
    state
        .local_tts_runtime
        .invalidate_model(&model_id)
        .await
        .map_err(safe_installer_error)?;
    let progress_app = app.clone();
    let progress: LocalTtsInstallProgressCallback = Arc::new(move |progress: LocalTtsInstallProgress| {
        let _ = progress_app.emit(LOCAL_TTS_PROGRESS_EVENT, progress);
    });
    installer
        .install(&model_id, current_platform()?, Some(progress))
        .await
        .map_err(safe_installer_error)?;
    descriptor_for_selected(state.inner(), &model_id)
}

#[tauri::command]
pub async fn cancel_local_tts_install(model_id: String) -> Result<bool, String> {
    Ok(global_local_tts_installer()
        .map_err(safe_installer_error)?
        .cancel(&model_id))
}

#[tauri::command]
pub async fn delete_local_tts_model(
    model_id: String,
    state: State<'_, AppState>,
) -> Result<LocalTtsModelDescriptor, String> {
    let installer = global_local_tts_installer().map_err(safe_installer_error)?;
    state
        .local_tts_runtime
        .delete_model(installer, model_id.clone())
        .await
        .map_err(safe_installer_error)?;
    // Preserve selection. A selected-but-deleted model remains selected and becomes not_installed;
    // no Google fallback or alternate model is chosen implicitly.
    descriptor_for_selected(state.inner(), &model_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_local_tts_model_has_user_facing_lifecycle_metadata() {
        let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
        assert!(!manifest.display_name.is_empty());
        assert!(!manifest.version.is_empty());
        assert!(!manifest.license.is_empty());
        assert!(!manifest.common_artifacts.is_empty());
    }
}
