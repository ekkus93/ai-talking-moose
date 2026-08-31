use crate::ai::local::{
    global_local_model_installer, LocalModelDescriptor, LocalModelDiagnostics,
    LocalModelInstallProgress, LocalModelInstallProgressCallback,
};
use crate::app::state::AppState;
use std::sync::Arc;
use tauri::{Emitter, Runtime, State};

const LOCAL_MODEL_PROGRESS_EVENT: &str = "moose://local-llm/model-progress";

fn safe_installer_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn descriptor_for_selected(
    state: &AppState,
    model_id: &str,
) -> Result<LocalModelDescriptor, String> {
    let selected = state.settings.read().local_text_model.clone();
    global_local_model_installer()
        .map_err(safe_installer_error)?
        .descriptors(&selected)
        .into_iter()
        .find(|descriptor| descriptor.id == model_id)
        .ok_or_else(|| "The selected local model is not in the supported catalog.".to_string())
}

#[tauri::command]
pub async fn get_local_llm_models(
    state: State<'_, AppState>,
) -> Result<Vec<LocalModelDescriptor>, String> {
    let selected = state.settings.read().local_text_model.clone();
    Ok(global_local_model_installer()
        .map_err(safe_installer_error)?
        .descriptors(&selected))
}

#[tauri::command]
pub async fn get_local_llm_diagnostics() -> Result<LocalModelDiagnostics, String> {
    Ok(global_local_model_installer()
        .map_err(safe_installer_error)?
        .diagnostics())
}

#[tauri::command]
pub async fn install_local_llm_model<R: Runtime>(
    model_id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<LocalModelDescriptor, String> {
    let installer = global_local_model_installer().map_err(safe_installer_error)?;
    let progress_app = app.clone();
    let progress: LocalModelInstallProgressCallback =
        Arc::new(move |progress: LocalModelInstallProgress| {
            let _ = progress_app.emit(LOCAL_MODEL_PROGRESS_EVENT, progress);
        });
    installer
        .install(&model_id, Some(progress))
        .await
        .map_err(safe_installer_error)?;
    descriptor_for_selected(state.inner(), &model_id)
}

#[tauri::command]
pub async fn cancel_local_llm_install(model_id: String) -> Result<bool, String> {
    Ok(global_local_model_installer()
        .map_err(safe_installer_error)?
        .cancel(&model_id))
}

#[tauri::command]
pub async fn delete_local_llm_model(
    model_id: String,
    state: State<'_, AppState>,
) -> Result<LocalModelDescriptor, String> {
    let installer = global_local_model_installer().map_err(safe_installer_error)?;
    installer.delete(&model_id).map_err(safe_installer_error)?;
    // Selection is deliberately preserved. A selected-but-deleted model remains selected and
    // becomes NotInstalled; no other local model or cloud provider is substituted implicitly.
    descriptor_for_selected(state.inner(), &model_id)
}
