use super::*;
use crate::asr::pipeline::LOCAL_ASR_QUEUE_CAPACITY_CHUNKS;
use crate::asr::types::LocalAsrRuntimeDiagnostics;
use async_trait::async_trait;

#[tokio::test]
async fn shutdown_preserves_last_local_asr_diagnostics_for_selected_mode() {
    struct DiagnosticLocalAsrResource {
        diagnostics: LocalAsrRuntimeDiagnostics,
    }

    #[async_trait]
    impl crate::asr::lifecycle::LocalAsrResource for DiagnosticLocalAsrResource {
        async fn stop(&mut self) -> Result<(), crate::asr::AsrError> {
            Ok(())
        }

        fn diagnostics(&self) -> Option<LocalAsrRuntimeDiagnostics> {
            Some(self.diagnostics.clone())
        }
    }

    let manager = ConversationManager::new();
    *manager.active_asr_mode.lock() = Some(AsrMode::MoonshineTinyStreaming);
    manager
        .local_asr_lifecycle()
        .attach(
            1,
            Box::new(DiagnosticLocalAsrResource {
                diagnostics: LocalAsrRuntimeDiagnostics {
                    input_sample_rate_hz: 16_000,
                    streaming: true,
                    queue_capacity: LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
                    first_partial_latency_ms: Some(42),
                    real_time_factor: Some(0.4),
                    ..LocalAsrRuntimeDiagnostics::default()
                },
            }),
        )
        .await
        .unwrap();

    manager
        .shutdown_locked(
            Arc::new(SyncMutex::new(AudioCapture::new_mock())),
            Arc::new(AudioPlayback::new()),
            ConversationLifecycle::Idle,
        )
        .await;

    let (snapshot, dropped_chunks) = manager
        .last_local_asr_diagnostics(AsrMode::MoonshineTinyStreaming)
        .expect("Tiny diagnostics should survive resource teardown");
    assert_eq!(snapshot.first_partial_latency_ms, Some(42));
    assert_eq!(snapshot.real_time_factor, Some(0.4));
    assert!(!snapshot.streaming);
    assert!(snapshot.metrics_snapshot);
    assert_eq!(dropped_chunks, 0);
    assert!(manager
        .last_local_asr_diagnostics(AsrMode::MoonshineSmallStreaming)
        .is_none());
}
