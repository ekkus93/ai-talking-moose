use crate::app::state::AppState;
use crate::asr::moonshine::{
    model_manifest_info, MoonshineModelArchitecture, MoonshineModelInstallCancellation,
    MoonshineModelInstallErrorKind, MoonshineModelInstallPhase, MoonshineModelInstallProgress,
    MoonshineModelInstallProgressCallback, MoonshineModelInstaller,
};
use crate::asr::{AsrMode, AsrModelDescriptor, AsrModelInstallState};
use crate::conversation::session::ConversationLifecycle;
use serde::Serialize;
use std::sync::Arc;
use tauri::{Emitter, Runtime, State};

const MODEL_PROGRESS_EVENT: &str = "moose://asr/model-progress";

#[derive(Debug, Clone, Serialize)]
pub struct AsrModelProgressEvent {
    pub mode: AsrMode,
    pub install_state: AsrModelInstallState,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub current_file: Option<String>,
}

pub(super) fn architecture_for_mode(mode: AsrMode) -> Result<MoonshineModelArchitecture, String> {
    match mode {
        AsrMode::MoonshineTinyStreaming => Ok(MoonshineModelArchitecture::TinyStreaming),
        AsrMode::MoonshineSmallStreaming => Ok(MoonshineModelArchitecture::SmallStreaming),
        AsrMode::GeminiLiveAudio => {
            Err("Gemini Live cloud ASR does not use a local Moonshine model.".to_string())
        }
    }
}

fn mode_for_architecture(architecture: MoonshineModelArchitecture) -> AsrMode {
    match architecture {
        MoonshineModelArchitecture::TinyStreaming => AsrMode::MoonshineTinyStreaming,
        MoonshineModelArchitecture::SmallStreaming => AsrMode::MoonshineSmallStreaming,
    }
}

fn descriptor_for_architecture(
    installer: &MoonshineModelInstaller,
    architecture: MoonshineModelArchitecture,
    active: bool,
) -> AsrModelDescriptor {
    let info = model_manifest_info(architecture);
    let mode = mode_for_architecture(architecture);
    let (install_state, installed_bytes, error_message) =
        match installer.verify_installed(architecture) {
            Ok(Some(outcome)) => (
                AsrModelInstallState::Installed,
                Some(outcome.installed_bytes),
                None,
            ),
            Ok(None) => (AsrModelInstallState::NotInstalled, None, None),
            Err(error)
                if matches!(
                    error.kind,
                    MoonshineModelInstallErrorKind::CorruptInstall
                        | MoonshineModelInstallErrorKind::SizeMismatch
                        | MoonshineModelInstallErrorKind::Sha256Mismatch
                        | MoonshineModelInstallErrorKind::Crc32cMismatch
                ) =>
            {
                (AsrModelInstallState::Corrupt, None, Some(error.message))
            }
            Err(error)
                if matches!(
                    error.kind,
                    MoonshineModelInstallErrorKind::InvalidManifest
                        | MoonshineModelInstallErrorKind::UnsupportedArtifact
                ) =>
            {
                (
                    AsrModelInstallState::Incompatible,
                    None,
                    Some(error.message),
                )
            }
            Err(error) => (AsrModelInstallState::Failed, None, Some(error.message)),
        };

    AsrModelDescriptor {
        id: info.id.to_string(),
        display_name: info.display_name.to_string(),
        mode,
        install_state,
        revision: info.revision.to_string(),
        runtime_release: info.runtime_release.to_string(),
        installed_bytes,
        expected_bytes: info.expected_bytes,
        active,
        error_message,
    }
}

pub(super) async fn load_descriptor(
    installer: Arc<MoonshineModelInstaller>,
    architecture: MoonshineModelArchitecture,
    active: bool,
) -> Result<AsrModelDescriptor, String> {
    tokio::task::spawn_blocking(move || {
        descriptor_for_architecture(installer.as_ref(), architecture, active)
    })
    .await
    .map_err(|_| "Moonshine model verification worker terminated unexpectedly.".to_string())
}

fn model_is_in_use(
    active_mode: Option<AsrMode>,
    lifecycle_busy: bool,
    selected_mode: AsrMode,
    requested_mode: AsrMode,
) -> bool {
    active_mode == Some(requested_mode) || (lifecycle_busy && selected_mode == requested_mode)
}

pub(super) fn model_in_use(state: &AppState, mode: AsrMode) -> bool {
    let lifecycle_busy = !matches!(
        state.conversation_mgr.lifecycle(),
        ConversationLifecycle::Idle | ConversationLifecycle::Failed
    );
    model_is_in_use(
        state.conversation_mgr.active_asr_mode(),
        lifecycle_busy,
        state.settings.read().asr_mode,
        mode,
    )
}

fn ensure_model_mutation_allowed(state: &AppState, mode: AsrMode) -> Result<(), String> {
    if model_in_use(state, mode) {
        let architecture = architecture_for_mode(mode)?;
        let info = model_manifest_info(architecture);
        return Err(format!(
            "{} is currently active. Stop the conversation before changing this model.",
            info.display_name
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_asr_models(state: State<'_, AppState>) -> Result<Vec<AsrModelDescriptor>, String> {
    let tiny_active = model_in_use(state.inner(), AsrMode::MoonshineTinyStreaming);
    let small_active = model_in_use(state.inner(), AsrMode::MoonshineSmallStreaming);
    let tiny = load_descriptor(
        state.moonshine_installer.clone(),
        MoonshineModelArchitecture::TinyStreaming,
        tiny_active,
    );
    let small = load_descriptor(
        state.moonshine_installer.clone(),
        MoonshineModelArchitecture::SmallStreaming,
        small_active,
    );
    let (tiny, small) = tokio::try_join!(tiny, small)?;
    Ok(vec![tiny, small])
}

#[tauri::command]
pub async fn install_asr_model<R: Runtime>(
    mode: AsrMode,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<AsrModelDescriptor, String> {
    let architecture = architecture_for_mode(mode)?;
    ensure_model_mutation_allowed(state.inner(), mode)?;

    let progress_app = app.clone();
    let progress: MoonshineModelInstallProgressCallback =
        Arc::new(move |progress: MoonshineModelInstallProgress| {
            let install_state = match progress.phase {
                MoonshineModelInstallPhase::Downloading => AsrModelInstallState::Downloading,
                MoonshineModelInstallPhase::Verifying => AsrModelInstallState::Verifying,
            };
            let _ = progress_app.emit(
                MODEL_PROGRESS_EVENT,
                AsrModelProgressEvent {
                    mode,
                    install_state,
                    downloaded_bytes: progress.downloaded_bytes,
                    total_bytes: progress.total_bytes,
                    current_file: progress.current_file,
                },
            );
        });

    state
        .moonshine_installer
        .install_with_progress(
            architecture,
            &MoonshineModelInstallCancellation::default(),
            progress,
        )
        .await
        .map_err(|error| error.message)?;

    let active = model_in_use(state.inner(), mode);
    load_descriptor(state.moonshine_installer.clone(), architecture, active).await
}

#[tauri::command]
pub async fn delete_asr_model(
    mode: AsrMode,
    state: State<'_, AppState>,
) -> Result<AsrModelDescriptor, String> {
    let architecture = architecture_for_mode(mode)?;
    ensure_model_mutation_allowed(state.inner(), mode)?;
    state
        .moonshine_installer
        .delete_installed(architecture)
        .await
        .map_err(|error| error.message)?;
    let active = model_in_use(state.inner(), mode);
    load_descriptor(state.moonshine_installer.clone(), architecture, active).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_mode_has_no_local_model_architecture() {
        let error = architecture_for_mode(AsrMode::GeminiLiveAudio).unwrap_err();
        assert!(error.contains("does not use a local Moonshine model"));
    }

    #[test]
    fn local_modes_map_to_distinct_model_architectures() {
        assert_eq!(
            architecture_for_mode(AsrMode::MoonshineTinyStreaming).unwrap(),
            MoonshineModelArchitecture::TinyStreaming
        );
        assert_eq!(
            architecture_for_mode(AsrMode::MoonshineSmallStreaming).unwrap(),
            MoonshineModelArchitecture::SmallStreaming
        );
    }

    #[test]
    fn descriptor_uses_pinned_manifest_metadata_for_missing_model() {
        let temp = tempfile::TempDir::new().unwrap();
        let installer = MoonshineModelInstaller::new(temp.path()).unwrap();
        let descriptor = descriptor_for_architecture(
            &installer,
            MoonshineModelArchitecture::TinyStreaming,
            false,
        );

        assert_eq!(descriptor.install_state, AsrModelInstallState::NotInstalled);
        assert_eq!(descriptor.revision, "quantized_26_07_30");
        assert_eq!(descriptor.expected_bytes, 51_441_771);
        assert_eq!(descriptor.runtime_release, "v0.1.3");
        assert!(!descriptor.active);
    }

    #[test]
    fn active_or_connecting_local_model_is_blocked_from_mutation() {
        assert!(model_is_in_use(
            Some(AsrMode::MoonshineTinyStreaming),
            false,
            AsrMode::GeminiLiveAudio,
            AsrMode::MoonshineTinyStreaming
        ));
        assert!(model_is_in_use(
            None,
            true,
            AsrMode::MoonshineSmallStreaming,
            AsrMode::MoonshineSmallStreaming
        ));
        assert!(!model_is_in_use(
            Some(AsrMode::MoonshineTinyStreaming),
            true,
            AsrMode::MoonshineTinyStreaming,
            AsrMode::MoonshineSmallStreaming
        ));
    }
}
