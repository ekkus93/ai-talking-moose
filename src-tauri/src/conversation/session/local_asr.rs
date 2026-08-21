use super::*;
use crate::asr::pipeline::{
    LocalAsrPipeline, LocalAsrPipelineEventCallback, LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
};
use crate::asr::types::LocalAsrRuntimeDiagnostics;
use crate::asr::{AsrError, AsrErrorKind};

#[derive(Default)]
pub(super) struct LocalAsrDiagnosticsStore {
    last: SyncMutex<Option<(AsrMode, LocalAsrRuntimeDiagnostics, u64)>>,
}

impl LocalAsrDiagnosticsStore {
    pub(super) fn get(&self, mode: AsrMode) -> Option<(LocalAsrRuntimeDiagnostics, u64)> {
        self.last
            .lock()
            .as_ref()
            .and_then(|(diagnostic_mode, diagnostics, dropped_chunks)| {
                if *diagnostic_mode == mode {
                    Some((diagnostics.clone(), *dropped_chunks))
                } else {
                    None
                }
            })
    }

    pub(super) fn clear(&self, mode: AsrMode) {
        let mut last = self.last.lock();
        if last
            .as_ref()
            .is_some_and(|(diagnostic_mode, _, _)| *diagnostic_mode == mode)
        {
            *last = None;
        }
    }

    pub(super) fn remember_error(&self, mode: AsrMode, error: AsrError) {
        let mut last = self.last.lock();
        if let Some((diagnostic_mode, diagnostics, _)) = last.as_mut() {
            if *diagnostic_mode == mode {
                diagnostics.last_error = Some(error);
                return;
            }
        }
        *last = Some((
            mode,
            LocalAsrRuntimeDiagnostics {
                input_sample_rate_hz: 16_000,
                queue_capacity: LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
                last_error: Some(error),
                ..LocalAsrRuntimeDiagnostics::default()
            },
            0,
        ));
    }

    pub(super) fn remember_snapshot(
        &self,
        mode: AsrMode,
        mut diagnostics: LocalAsrRuntimeDiagnostics,
        dropped_chunks: u64,
    ) {
        diagnostics.streaming = false;
        diagnostics.metrics_snapshot = true;
        diagnostics.queue_depth = 0;
        *self.last.lock() = Some((mode, diagnostics, dropped_chunks));
    }
}

pub(super) struct LocalAsrPreparation {
    pub(super) generation: u64,
    pub(super) asr_mode: AsrMode,
    pub(super) installer: Option<Arc<MoonshineModelInstaller>>,
    pub(super) session_id: String,
    pub(super) capture: Arc<SyncMutex<AudioCapture>>,
    pub(super) playback: Arc<AudioPlayback>,
    pub(super) state_callback: StateCallback,
    pub(super) lifecycle_callback: LifecycleCallback,
    pub(super) provider_error_callback: ProviderErrorCallback,
}

impl ConversationManager {
    pub(crate) fn last_local_asr_diagnostics(
        &self,
        mode: AsrMode,
    ) -> Option<(LocalAsrRuntimeDiagnostics, u64)> {
        self.local_asr_diagnostics.get(mode)
    }

    pub(super) async fn stop_local_asr_for_shutdown(
        &self,
        active_asr_mode: Option<AsrMode>,
        capture: &Arc<SyncMutex<AudioCapture>>,
    ) {
        if let Some(mode) = active_asr_mode.filter(|mode| is_local_mode(*mode)) {
            if let Some(diagnostics) = self.local_asr.diagnostics().await {
                self.local_asr_diagnostics.remember_snapshot(
                    mode,
                    diagnostics,
                    capture.lock().dropped_chunks(),
                );
            }
        }

        if let Err(error) = self.local_asr.stop_and_clear().await {
            if let Some(mode) = active_asr_mode.filter(|mode| is_local_mode(*mode)) {
                self.local_asr_diagnostics
                    .remember_error(mode, error.clone());
            }
            warn!(
                kind = ?error.kind,
                "Local ASR resource teardown reported an error"
            );
        }
    }

    pub(super) async fn stop_provisional_local_asr(pipeline: &mut Option<LocalAsrPipeline>) {
        let Some(mut pipeline) = pipeline.take() else {
            return;
        };
        if let Err(error) = pipeline.stop_and_join().await {
            warn!(
                kind = ?error.kind,
                "Failed to stop provisional local ASR pipeline"
            );
        }
    }

    pub(super) async fn prepare_local_asr(
        &self,
        preparation: LocalAsrPreparation,
    ) -> Result<Option<LocalAsrPipeline>, String> {
        let LocalAsrPreparation {
            generation,
            asr_mode,
            installer,
            session_id,
            capture,
            playback,
            state_callback,
            lifecycle_callback,
            provider_error_callback,
        } = preparation;

        if !is_local_mode(asr_mode) {
            return Ok(None);
        }

        self.local_asr_diagnostics.clear(asr_mode);
        let installer = match installer {
            Some(installer) => installer,
            None => {
                let message = concat!(
                    "Local Moonshine ASR is selected, but the model installer is unavailable. ",
                    "No microphone audio was sent."
                )
                .to_string();
                self.local_asr_diagnostics.remember_error(
                    asr_mode,
                    AsrError {
                        kind: AsrErrorKind::RuntimeUnavailable,
                        message: message.clone(),
                        retryable: false,
                    },
                );
                Self::set_lifecycle(
                    &self.lifecycle,
                    ConversationLifecycle::Failed,
                    Some(&lifecycle_callback),
                );
                state_callback(CharacterState::Error);
                return Err(message);
            }
        };

        let manager_for_asr = self.clone();
        let session_id_for_asr = session_id.clone();
        let capture_for_asr = capture.clone();
        let playback_for_asr = playback.clone();
        let state_for_asr = state_callback.clone();
        let provider_error_for_asr = provider_error_callback.clone();
        let event_callback: LocalAsrPipelineEventCallback = Arc::new(move |event| {
            let manager = manager_for_asr.clone();
            let session_id = session_id_for_asr.clone();
            let capture = capture_for_asr.clone();
            let playback = playback_for_asr.clone();
            let state_callback = state_for_asr.clone();
            let provider_error_callback = provider_error_for_asr.clone();
            tauri::async_runtime::spawn(async move {
                match event {
                    AsrEvent::Error { error } => {
                        let cleaned = manager
                            .shutdown_if_generation_current(
                                generation,
                                capture,
                                playback,
                                ConversationLifecycle::Failed,
                            )
                            .await;
                        if cleaned {
                            warn!(kind = ?error.kind, "Local ASR inference terminated");
                            state_callback(CharacterState::Error);
                        }
                    }
                    event => {
                        if let Err(error) = manager
                            .handle_local_asr_event(generation, &session_id, event)
                            .await
                        {
                            let cleaned = manager
                                .shutdown_if_generation_current(
                                    generation,
                                    capture,
                                    playback,
                                    ConversationLifecycle::Failed,
                                )
                                .await;
                            if cleaned {
                                provider_error_callback(error);
                                state_callback(CharacterState::Error);
                            }
                        }
                    }
                }
            });
        });

        let pipeline_result = match asr_mode {
            AsrMode::MoonshineTinyStreaming => {
                LocalAsrPipeline::start_tiny(installer, event_callback).await
            }
            AsrMode::MoonshineSmallStreaming => {
                LocalAsrPipeline::start_small(installer, event_callback).await
            }
            AsrMode::GeminiLiveAudio => unreachable!("cloud mode does not create local ASR"),
        };

        match pipeline_result {
            Ok(pipeline) => Ok(Some(pipeline)),
            Err(error) => {
                self.local_asr_diagnostics
                    .remember_error(asr_mode, error.clone());
                Self::set_lifecycle(
                    &self.lifecycle,
                    ConversationLifecycle::Failed,
                    Some(&lifecycle_callback),
                );
                state_callback(CharacterState::Error);
                Err(error.message)
            }
        }
    }
}

fn is_local_mode(mode: AsrMode) -> bool {
    matches!(
        mode,
        AsrMode::MoonshineTinyStreaming | AsrMode::MoonshineSmallStreaming
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_store_is_scoped_by_mode_and_preserves_errors() {
        let store = LocalAsrDiagnosticsStore::default();
        store.remember_snapshot(
            AsrMode::MoonshineTinyStreaming,
            LocalAsrRuntimeDiagnostics {
                input_sample_rate_hz: 16_000,
                streaming: true,
                first_partial_latency_ms: Some(42),
                ..LocalAsrRuntimeDiagnostics::default()
            },
            3,
        );

        let (snapshot, dropped_chunks) = store
            .get(AsrMode::MoonshineTinyStreaming)
            .expect("Tiny snapshot should be retained");
        assert!(!snapshot.streaming);
        assert!(snapshot.metrics_snapshot);
        assert_eq!(snapshot.first_partial_latency_ms, Some(42));
        assert_eq!(dropped_chunks, 3);
        assert!(store.get(AsrMode::MoonshineSmallStreaming).is_none());

        store.remember_error(
            AsrMode::MoonshineTinyStreaming,
            AsrError {
                kind: AsrErrorKind::Inference,
                message: "boom".to_string(),
                retryable: true,
            },
        );
        assert_eq!(
            store
                .get(AsrMode::MoonshineTinyStreaming)
                .unwrap()
                .0
                .last_error
                .unwrap()
                .kind,
            AsrErrorKind::Inference
        );
    }
}
