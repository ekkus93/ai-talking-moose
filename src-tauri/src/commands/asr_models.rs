use crate::app::settings_policy::settings_runtime_lock;
use crate::app::state::AppState;
use crate::asr::whisper::{
    installer::{
        WhisperModelInstallCancellation, WhisperModelInstallErrorKind, WhisperModelInstallPhase,
        WhisperModelInstallProgress, WhisperModelInstallProgressCallback, WhisperModelInstaller,
    },
    manifest,
    manifest::{WHISPER_RUNTIME_UNBUILT_MESSAGE, WHISPER_SMALL_DISPLAY_NAME},
};
use crate::asr::{AsrMode, AsrModelDescriptor, AsrModelInstallState};
use crate::conversation::session::ConversationLifecycle;
use serde::Serialize;
use std::sync::Arc;
use tauri::{Emitter, Runtime, State};

const MODEL_PROGRESS_EVENT: &str = "moose://asr/model-progress";

struct WhisperInstallRegistration {
    slot: Arc<parking_lot::Mutex<Option<Arc<WhisperModelInstallCancellation>>>>,
    cancellation: Arc<WhisperModelInstallCancellation>,
}

impl Drop for WhisperInstallRegistration {
    fn drop(&mut self) {
        let mut active = self.slot.lock();
        if active
            .as_ref()
            .is_some_and(|registered| Arc::ptr_eq(registered, &self.cancellation))
        {
            *active = None;
        }
    }
}

fn cancel_active_whisper_install(
    slot: &parking_lot::Mutex<Option<Arc<WhisperModelInstallCancellation>>>,
) -> bool {
    if let Some(cancellation) = slot.lock().clone() {
        cancellation.cancel();
        true
    } else {
        false
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AsrModelProgressEvent {
    pub mode: AsrMode,
    pub install_state: AsrModelInstallState,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub current_file: Option<String>,
}

/// Verification failures must not all be reported as corrupted weights:
/// invalid bundled metadata is incompatible, while I/O/runtime failures are
/// operational errors that should not prompt an unnecessary model download.
fn whisper_verification_error_state(kind: WhisperModelInstallErrorKind) -> AsrModelInstallState {
    match kind {
        WhisperModelInstallErrorKind::CorruptInstall
        | WhisperModelInstallErrorKind::SizeMismatch
        | WhisperModelInstallErrorKind::Sha256Mismatch => AsrModelInstallState::Corrupt,
        WhisperModelInstallErrorKind::IncompatibleInstall
        | WhisperModelInstallErrorKind::InvalidManifest => AsrModelInstallState::Incompatible,
        WhisperModelInstallErrorKind::Io
        | WhisperModelInstallErrorKind::InsufficientDiskSpace
        | WhisperModelInstallErrorKind::Network
        | WhisperModelInstallErrorKind::Http
        | WhisperModelInstallErrorKind::Cancelled
        | WhisperModelInstallErrorKind::Promotion => AsrModelInstallState::Failed,
    }
}

/// Build a Whisper Small descriptor that reflects the real local install state
/// by querying the Whisper installer rather than assuming `NotInstalled`.
pub(super) fn whisper_descriptor(
    installer: &WhisperModelInstaller,
    active: bool,
) -> AsrModelDescriptor {
    let mut descriptor = manifest::model_descriptor(active);
    match installer.verify_installed() {
        Ok(Some(outcome)) => {
            descriptor.install_state = AsrModelInstallState::Installed;
            descriptor.installed_bytes = Some(outcome.installed_bytes);
            descriptor.revision = outcome.revision;
        }
        Ok(None) => {
            descriptor.install_state = AsrModelInstallState::NotInstalled;
        }
        Err(error) => {
            descriptor.install_state = whisper_verification_error_state(error.kind);
            descriptor.error_message = Some(error.message);
        }
    }
    descriptor
}

pub(super) async fn load_whisper_descriptor(
    installer: Arc<WhisperModelInstaller>,
    active: bool,
) -> Result<AsrModelDescriptor, String> {
    run_whisper_descriptor_blocking(move || whisper_descriptor(installer.as_ref(), active)).await
}

async fn run_whisper_descriptor_blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| "Whisper model verification worker terminated unexpectedly.".to_string())
}

#[cfg(test)]
mod blocking_verification_tests {
    use super::*;

    #[tokio::test]
    async fn whisper_descriptor_verification_runs_off_the_async_worker() {
        let temp = tempfile::TempDir::new().unwrap();
        let installer = Arc::new(WhisperModelInstaller::new(temp.path()).unwrap());
        let async_thread = std::thread::current().id();
        let verification_thread = run_whisper_descriptor_blocking(move || {
            let _descriptor = whisper_descriptor(installer.as_ref(), false);
            std::thread::current().id()
        })
        .await
        .unwrap();

        assert_ne!(verification_thread, async_thread);
    }
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

fn ensure_mode_supported(mode: AsrMode) -> Result<(), String> {
    match mode {
        AsrMode::WhisperSmall => Ok(()),
        AsrMode::GeminiLiveAudio => Err(
            "Gemini Live is a cloud audio mode and has no local ASR model to install or delete."
                .to_string(),
        ),
    }
}

fn ensure_model_mutation_allowed(state: &AppState, mode: AsrMode) -> Result<(), String> {
    if model_in_use(state, mode) {
        let display_name = WHISPER_SMALL_DISPLAY_NAME;
        return Err(format!(
            "{} is currently active. Stop the conversation before changing this model.",
            display_name
        ));
    }
    Ok(())
}

async fn acquire_model_mutation_guard(
    state: &AppState,
    mode: AsrMode,
) -> Result<tokio::sync::MutexGuard<'static, ()>, String> {
    // Conversation start takes this same guard before reading settings and keeps
    // it through graph activation. Holding it for the entire model mutation
    // makes the check-and-mutate decision atomic with respect to a new start.
    let guard = settings_runtime_lock().lock().await;
    ensure_model_mutation_allowed(state, mode)?;
    Ok(guard)
}

#[tauri::command]
pub async fn get_asr_models(state: State<'_, AppState>) -> Result<Vec<AsrModelDescriptor>, String> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let whisper_active = model_in_use(state.inner(), AsrMode::WhisperSmall);
        let whisper =
            load_whisper_descriptor(state.whisper_installer.clone(), whisper_active).await?;
        Ok(vec![whisper])
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = state;
        Ok(Vec::new())
    }
}

#[tauri::command]
pub async fn install_asr_model<R: Runtime>(
    mode: AsrMode,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<AsrModelDescriptor, String> {
    ensure_mode_supported(mode)?;
    let _settings_guard = acquire_model_mutation_guard(state.inner(), mode).await?;

    install_whisper(state.inner(), app).await
}

/// Installs the Whisper Small `ggml-small.bin` weight through the local Whisper
/// installer and streams `moose://asr/model-progress` events. Fails closed when the Whisper runtime is unbuilt.
async fn install_whisper<R: Runtime>(
    state: &AppState,
    app: tauri::AppHandle<R>,
) -> Result<AsrModelDescriptor, String> {
    if !cfg!(whisper_native_linked) {
        return Err(WHISPER_RUNTIME_UNBUILT_MESSAGE.to_string());
    }

    let progress_app = app.clone();
    let progress: WhisperModelInstallProgressCallback =
        Arc::new(move |progress: WhisperModelInstallProgress| {
            let install_state = match progress.phase {
                WhisperModelInstallPhase::Downloading => AsrModelInstallState::Downloading,
                WhisperModelInstallPhase::Verifying => AsrModelInstallState::Verifying,
            };
            let _ = progress_app.emit(
                MODEL_PROGRESS_EVENT,
                AsrModelProgressEvent {
                    mode: AsrMode::WhisperSmall,
                    install_state,
                    downloaded_bytes: progress.downloaded_bytes,
                    total_bytes: progress.total_bytes,
                    current_file: Some("ggml-small.bin".to_string()),
                },
            );
        });

    let cancellation = Arc::new(WhisperModelInstallCancellation::default());
    {
        let mut active = state.whisper_install_cancellation.lock();
        if active.is_some() {
            return Err("A Whisper model installation is already active.".to_string());
        }
        *active = Some(cancellation.clone());
    }
    let _registration = WhisperInstallRegistration {
        slot: state.whisper_install_cancellation.clone(),
        cancellation: cancellation.clone(),
    };
    let mut cancellation_for_install = cancellation.as_ref().clone();
    let outcome = state
        .whisper_installer
        .install(&mut cancellation_for_install, &Some(progress))
        .await
        .map_err(|error| error.message)?;

    let active = model_in_use(state, AsrMode::WhisperSmall);
    let mut descriptor = load_whisper_descriptor(state.whisper_installer.clone(), active).await?;
    descriptor.installed_bytes = Some(outcome.installed_bytes);
    descriptor.revision = outcome.revision;
    Ok(descriptor)
}

#[tauri::command]
pub fn cancel_whisper_asr_model_install(state: State<'_, AppState>) -> bool {
    cancel_active_whisper_install(&state.whisper_install_cancellation)
}

#[tauri::command]
pub async fn delete_asr_model(
    mode: AsrMode,
    state: State<'_, AppState>,
) -> Result<AsrModelDescriptor, String> {
    ensure_mode_supported(mode)?;
    let _settings_guard = acquire_model_mutation_guard(state.inner(), mode).await?;

    state
        .whisper_installer
        .delete()
        .await
        .map_err(|error| error.message)?;
    let active = model_in_use(state.inner(), mode);
    load_whisper_descriptor(state.whisper_installer.clone(), active).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whisper_install_cancellation_targets_only_the_registered_install() {
        let slot = Arc::new(parking_lot::Mutex::new(None));
        assert!(!cancel_active_whisper_install(&slot));

        let cancellation = Arc::new(WhisperModelInstallCancellation::default());
        *slot.lock() = Some(cancellation.clone());
        assert!(cancel_active_whisper_install(&slot));
        assert!(cancellation.is_cancelled());

        let registration = WhisperInstallRegistration {
            slot: slot.clone(),
            cancellation: cancellation.clone(),
        };
        drop(registration);
        assert!(!cancel_active_whisper_install(&slot));
    }

    #[test]
    fn whisper_descriptor_reports_missing_model_without_fabricating_installation() {
        let temp = tempfile::TempDir::new().unwrap();
        let installer = WhisperModelInstaller::new(temp.path()).unwrap();
        let descriptor = whisper_descriptor(&installer, false);
        assert_eq!(descriptor.install_state, AsrModelInstallState::NotInstalled);
        assert_eq!(descriptor.installed_bytes, None);
        assert_eq!(descriptor.revision, manifest::WHISPER_MODEL_REVISION);
        assert_eq!(descriptor.expected_bytes, manifest::WHISPER_MODEL_BYTES);
        assert!(!descriptor.active);
        // Unlinked CI/development builds must still disclose that native
        // Whisper is unavailable; the catalog must not fabricate readiness.
        assert_eq!(
            descriptor.error_message,
            manifest::model_descriptor(false).error_message
        );
    }

    #[test]
    fn whisper_catalog_distinguishes_corrupt_incompatible_and_operational_failures() {
        for kind in [
            WhisperModelInstallErrorKind::CorruptInstall,
            WhisperModelInstallErrorKind::SizeMismatch,
            WhisperModelInstallErrorKind::Sha256Mismatch,
        ] {
            assert_eq!(
                whisper_verification_error_state(kind),
                AsrModelInstallState::Corrupt
            );
        }
        for kind in [
            WhisperModelInstallErrorKind::IncompatibleInstall,
            WhisperModelInstallErrorKind::InvalidManifest,
        ] {
            assert_eq!(
                whisper_verification_error_state(kind),
                AsrModelInstallState::Incompatible
            );
        }
        for kind in [
            WhisperModelInstallErrorKind::Io,
            WhisperModelInstallErrorKind::InsufficientDiskSpace,
            WhisperModelInstallErrorKind::Network,
            WhisperModelInstallErrorKind::Http,
            WhisperModelInstallErrorKind::Cancelled,
            WhisperModelInstallErrorKind::Promotion,
        ] {
            assert_eq!(
                whisper_verification_error_state(kind),
                AsrModelInstallState::Failed
            );
        }
    }

    #[test]
    fn whisper_descriptor_reports_existing_truncated_model_as_corrupt() {
        let temp = tempfile::TempDir::new().unwrap();
        let installer = WhisperModelInstaller::new(temp.path()).unwrap();
        std::fs::write(installer.model_path(), b"truncated").unwrap();

        let descriptor = whisper_descriptor(&installer, false);
        assert_eq!(descriptor.install_state, AsrModelInstallState::Corrupt);
        assert!(descriptor.error_message.is_some());
    }

    #[test]
    fn active_or_connecting_local_model_is_blocked_from_mutation() {
        // Production Whisper mode must retain the same active-session and
        // in-flight-start protections as the legacy model selectors.
        assert!(model_is_in_use(
            Some(AsrMode::WhisperSmall),
            false,
            AsrMode::GeminiLiveAudio,
            AsrMode::WhisperSmall
        ));
        assert!(model_is_in_use(
            None,
            true,
            AsrMode::WhisperSmall,
            AsrMode::WhisperSmall
        ));
        assert!(!model_is_in_use(
            Some(AsrMode::GeminiLiveAudio),
            false,
            AsrMode::WhisperSmall,
            AsrMode::WhisperSmall
        ));
    }

    #[tokio::test]
    async fn model_mutation_and_conversation_start_share_one_runtime_guard() {
        use std::time::Duration;

        let state = AppState::new_for_tests().unwrap();

        // Simulate a conversation start already inside its settings snapshot /
        // graph-activation critical section. Model mutation must wait.
        let conversation_guard = settings_runtime_lock().lock().await;
        let mutation = acquire_model_mutation_guard(&state, AsrMode::WhisperSmall);
        tokio::pin!(mutation);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut mutation)
                .await
                .is_err()
        );
        drop(conversation_guard);
        let mutation_guard = tokio::time::timeout(Duration::from_secs(1), &mut mutation)
            .await
            .expect("model mutation should proceed after conversation guard releases")
            .unwrap();

        // And the reverse interleaving is blocked by the same lock: once a
        // mutation has passed its in-use check, a new conversation start cannot
        // enter its settings critical section until the mutation completes.
        let conversation_start = settings_runtime_lock().lock();
        tokio::pin!(conversation_start);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut conversation_start)
                .await
                .is_err()
        );
        drop(mutation_guard);
        let conversation_guard =
            tokio::time::timeout(Duration::from_secs(1), &mut conversation_start)
                .await
                .expect("conversation start should proceed after model mutation releases");
        drop(conversation_guard);
    }

    #[test]
    fn only_whisper_has_a_local_model_to_install_or_delete() {
        assert!(ensure_mode_supported(AsrMode::WhisperSmall).is_ok());
        let cloud_error = ensure_mode_supported(AsrMode::GeminiLiveAudio).unwrap_err();
        assert!(cloud_error.contains("no local ASR model"));
        assert!(!cloud_error.contains("Moonshine"));
    }

    #[test]
    fn whisper_descriptor_uses_pinned_manifest_metadata_and_configured_error() {
        let descriptor = manifest::model_descriptor(false);

        assert_eq!(descriptor.mode, AsrMode::WhisperSmall);
        assert_eq!(descriptor.install_state, AsrModelInstallState::NotInstalled);
        assert_eq!(descriptor.id, "whisper-small-ggml");
        assert_eq!(descriptor.expected_bytes, 487_601_967);
        assert!(!descriptor.active);
        if cfg!(whisper_native_linked) {
            assert!(descriptor.error_message.is_none());
        } else {
            assert!(descriptor
                .error_message
                .as_deref()
                .unwrap_or("")
                .contains("not yet built"));
        }
    }
}
