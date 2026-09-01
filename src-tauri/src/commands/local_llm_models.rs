use crate::ai::local::{
    global_local_model_installer, LocalModelDescriptor, LocalModelDiagnostics,
    LocalModelInstallProgress, LocalModelInstallProgressCallback, LocalModelInstallState,
    LocalTextModel,
};
use crate::ai::traits::TextModel;
use crate::ai::types::TextRequest;
use crate::app::state::AppState;
use crate::commands::settings::ConnectionTestResult;
use std::sync::Arc;
use tauri::{Emitter, Runtime, State};

const LOCAL_MODEL_PROGRESS_EVENT: &str = "moose://local-llm/model-progress";
const LOCAL_MODEL_TEST_MAX_TOKENS: u32 = 32;

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

fn local_model_test_request() -> TextRequest {
    TextRequest {
        prompt: "Reply with one short sentence confirming local text generation is working."
            .to_string(),
        system_instruction: Some(
            "This is a bounded local model self-test. Return only a short visible answer."
                .to_string(),
        ),
        temperature: Some(0.2),
        max_tokens: Some(LOCAL_MODEL_TEST_MAX_TOKENS),
    }
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
    state
        .local_llm_runtime
        .delete_model(installer, model_id.clone())
        .await
        .map_err(safe_installer_error)?;
    // Selection is deliberately preserved. A selected-but-deleted model remains selected and
    // becomes NotInstalled; no other local model or cloud provider is substituted implicitly.
    descriptor_for_selected(state.inner(), &model_id)
}

#[tauri::command]
pub async fn test_local_llm_model(
    state: State<'_, AppState>,
) -> Result<ConnectionTestResult, String> {
    if state.conversation_mgr.is_active() {
        return Ok(ConnectionTestResult {
            success: false,
            message: "Stop the active voice conversation before testing the local text model."
                .to_string(),
        });
    }

    let model_id = state.settings.read().local_text_model.clone();
    let installer = global_local_model_installer().map_err(safe_installer_error)?;
    let descriptor = installer
        .descriptors(&model_id)
        .into_iter()
        .find(|descriptor| descriptor.id == model_id)
        .ok_or_else(|| "The selected local model is not in the supported catalog.".to_string())?;

    if descriptor.install_state != LocalModelInstallState::Installed {
        return Ok(ConnectionTestResult {
            success: false,
            message: format!("Install {} before testing it.", descriptor.display_name),
        });
    }

    let model = LocalTextModel::new(
        state.local_llm_runtime.clone(),
        Ok(installer),
        descriptor.id.clone(),
    );
    match model.generate(local_model_test_request()).await {
        Ok(_) => Ok(ConnectionTestResult {
            success: true,
            message: format!("{} generated a local response successfully.", descriptor.display_name),
        }),
        Err(error) => Ok(ConnectionTestResult {
            success: false,
            message: error.message,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_model_self_test_is_fixed_and_bounded() {
        let request = local_model_test_request();
        assert_eq!(request.temperature, Some(0.2));
        assert_eq!(request.max_tokens, Some(LOCAL_MODEL_TEST_MAX_TOKENS));
        assert!(request.system_instruction.is_some());
        assert!(!request.prompt.is_empty());
    }
}
