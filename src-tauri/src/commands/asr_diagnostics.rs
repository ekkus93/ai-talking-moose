use super::asr_models::{architecture_for_mode, load_descriptor, model_in_use};
use crate::app::state::AppState;
use crate::asr::pipeline::LOCAL_ASR_QUEUE_CAPACITY_CHUNKS;
use crate::asr::types::LocalAsrRuntimeDiagnostics;
use crate::asr::{AsrDiagnostics, AsrMode, AsrModelDescriptor};
use tauri::State;

fn empty_local_runtime() -> LocalAsrRuntimeDiagnostics {
    LocalAsrRuntimeDiagnostics {
        input_sample_rate_hz: 16_000,
        streaming: false,
        queue_depth: 0,
        queue_capacity: LOCAL_ASR_QUEUE_CAPACITY_CHUNKS,
        ..LocalAsrRuntimeDiagnostics::default()
    }
}

fn compose_asr_diagnostics(
    selected_mode: AsrMode,
    descriptor: Option<&AsrModelDescriptor>,
    runtime: Option<LocalAsrRuntimeDiagnostics>,
    dropped_chunks: u64,
    capture_sample_rate_hz: Option<u32>,
) -> AsrDiagnostics {
    match selected_mode {
        AsrMode::GeminiLiveAudio => AsrDiagnostics {
            selected_mode,
            engine_name: "Gemini Live Cloud Audio".to_string(),
            model_id: None,
            model_revision: None,
            install_state: None,
            input_sample_rate_hz: capture_sample_rate_hz.unwrap_or(0),
            streaming: false,
            metrics_snapshot: false,
            cpu_threads: None,
            queue_depth: 0,
            queue_capacity: 0,
            dropped_chunks,
            last_error: None,
            first_partial_latency_ms: None,
            first_final_latency_ms: None,
            last_transcription_latency_ms: None,
            processed_audio_ms: 0,
            inference_wall_time_ms: 0,
            real_time_factor: None,
            process_cpu_time_ms: None,
            average_cpu_utilization_percent: None,
            baseline_resident_memory_bytes: None,
            resident_memory_bytes: None,
            peak_resident_memory_bytes: None,
        },
        AsrMode::MoonshineTinyStreaming | AsrMode::MoonshineSmallStreaming => {
            let runtime = runtime.unwrap_or_else(empty_local_runtime);
            AsrDiagnostics {
                selected_mode,
                engine_name: descriptor.map_or_else(
                    || "Moonshine local ASR".to_string(),
                    |model| model.display_name.clone(),
                ),
                model_id: descriptor.map(|model| model.id.clone()),
                model_revision: descriptor.map(|model| model.revision.clone()),
                install_state: descriptor.map(|model| model.install_state),
                input_sample_rate_hz: runtime.input_sample_rate_hz,
                streaming: runtime.streaming,
                metrics_snapshot: runtime.metrics_snapshot,
                cpu_threads: std::thread::available_parallelism()
                    .ok()
                    .map(std::num::NonZero::get),
                queue_depth: runtime.queue_depth,
                queue_capacity: runtime.queue_capacity,
                dropped_chunks,
                last_error: runtime.last_error,
                first_partial_latency_ms: runtime.first_partial_latency_ms,
                first_final_latency_ms: runtime.first_final_latency_ms,
                last_transcription_latency_ms: runtime.last_transcription_latency_ms,
                processed_audio_ms: runtime.processed_audio_ms,
                inference_wall_time_ms: runtime.inference_wall_time_ms,
                real_time_factor: runtime.real_time_factor,
                process_cpu_time_ms: runtime.process_cpu_time_ms,
                average_cpu_utilization_percent: runtime.average_cpu_utilization_percent,
                baseline_resident_memory_bytes: runtime.baseline_resident_memory_bytes,
                resident_memory_bytes: runtime.resident_memory_bytes,
                peak_resident_memory_bytes: runtime.peak_resident_memory_bytes,
            }
        }
    }
}

#[tauri::command]
pub async fn get_asr_diagnostics(state: State<'_, AppState>) -> Result<AsrDiagnostics, String> {
    let selected_mode = state.settings.read().asr_mode;
    let capture_diagnostics = state.audio_capture.lock().diagnostics();
    let dropped_chunks = capture_diagnostics.dropped_chunks;

    if selected_mode == AsrMode::GeminiLiveAudio {
        return Ok(compose_asr_diagnostics(
            selected_mode,
            None,
            None,
            dropped_chunks,
            capture_diagnostics.sample_rate_hz,
        ));
    }

    let architecture = architecture_for_mode(selected_mode)?;
    let active = model_in_use(state.inner(), selected_mode);
    let descriptor =
        load_descriptor(state.moonshine_installer.clone(), architecture, active).await?;
    let (runtime, local_dropped_chunks) =
        if state.conversation_mgr.active_asr_mode() == Some(selected_mode) {
            (
                state
                    .conversation_mgr
                    .local_asr_lifecycle()
                    .diagnostics()
                    .await,
                dropped_chunks,
            )
        } else if let Some((runtime, snapshot_dropped_chunks)) = state
            .conversation_mgr
            .last_local_asr_diagnostics(selected_mode)
        {
            (Some(runtime), snapshot_dropped_chunks)
        } else {
            (None, 0)
        };

    Ok(compose_asr_diagnostics(
        selected_mode,
        Some(&descriptor),
        runtime,
        local_dropped_chunks,
        capture_diagnostics.sample_rate_hz,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asr::AsrModelInstallState;

    fn descriptor(mode: AsrMode) -> AsrModelDescriptor {
        AsrModelDescriptor {
            id: match mode {
                AsrMode::MoonshineTinyStreaming => "moonshine-tiny-streaming-en",
                AsrMode::MoonshineSmallStreaming => "moonshine-small-streaming-en",
                AsrMode::GeminiLiveAudio => unreachable!("cloud mode has no local descriptor"),
            }
            .to_string(),
            display_name: "Moonshine local ASR".to_string(),
            mode,
            install_state: AsrModelInstallState::Installed,
            revision: "test-revision".to_string(),
            runtime_release: "test-runtime".to_string(),
            installed_bytes: Some(1),
            expected_bytes: 1,
            active: false,
            error_message: None,
        }
    }

    #[test]
    fn inactive_local_diagnostics_report_contract_without_fabricated_measurements() {
        let descriptor = descriptor(AsrMode::MoonshineTinyStreaming);
        let diagnostics = compose_asr_diagnostics(
            AsrMode::MoonshineTinyStreaming,
            Some(&descriptor),
            None,
            3,
            None,
        );
        assert_eq!(
            diagnostics.model_id.as_deref(),
            Some("moonshine-tiny-streaming-en")
        );
        assert_eq!(diagnostics.input_sample_rate_hz, 16_000);
        assert_eq!(diagnostics.queue_capacity, LOCAL_ASR_QUEUE_CAPACITY_CHUNKS);
        assert_eq!(diagnostics.dropped_chunks, 3);
        assert!(!diagnostics.streaming);
        assert!(!diagnostics.metrics_snapshot);
        assert_eq!(diagnostics.first_partial_latency_ms, None);
        assert_eq!(diagnostics.real_time_factor, None);
        assert_eq!(diagnostics.resident_memory_bytes, None);
    }

    #[test]
    fn active_local_diagnostics_preserve_runtime_metrics() {
        let descriptor = descriptor(AsrMode::MoonshineSmallStreaming);
        let runtime = LocalAsrRuntimeDiagnostics {
            input_sample_rate_hz: 16_000,
            streaming: true,
            metrics_snapshot: false,
            queue_depth: 2,
            queue_capacity: 8,
            last_error: None,
            first_partial_latency_ms: Some(41),
            first_final_latency_ms: Some(88),
            last_transcription_latency_ms: Some(12),
            processed_audio_ms: 1_000,
            inference_wall_time_ms: 250,
            real_time_factor: Some(0.25),
            process_cpu_time_ms: Some(325),
            average_cpu_utilization_percent: Some(32.5),
            baseline_resident_memory_bytes: Some(100),
            resident_memory_bytes: Some(200),
            peak_resident_memory_bytes: Some(250),
        };
        let diagnostics = compose_asr_diagnostics(
            AsrMode::MoonshineSmallStreaming,
            Some(&descriptor),
            Some(runtime),
            1,
            Some(16_000),
        );
        assert!(diagnostics.streaming);
        assert!(!diagnostics.metrics_snapshot);
        assert_eq!(diagnostics.queue_depth, 2);
        assert_eq!(diagnostics.first_partial_latency_ms, Some(41));
        assert_eq!(diagnostics.first_final_latency_ms, Some(88));
        assert_eq!(diagnostics.last_transcription_latency_ms, Some(12));
        assert_eq!(diagnostics.processed_audio_ms, 1_000);
        assert_eq!(diagnostics.inference_wall_time_ms, 250);
        assert_eq!(diagnostics.real_time_factor, Some(0.25));
        assert_eq!(diagnostics.process_cpu_time_ms, Some(325));
        assert_eq!(diagnostics.average_cpu_utilization_percent, Some(32.5));
        assert_eq!(diagnostics.baseline_resident_memory_bytes, Some(100));
        assert_eq!(diagnostics.resident_memory_bytes, Some(200));
        assert_eq!(diagnostics.peak_resident_memory_bytes, Some(250));
    }

    #[test]
    fn cloud_diagnostics_do_not_fabricate_local_runtime_metrics() {
        let diagnostics =
            compose_asr_diagnostics(AsrMode::GeminiLiveAudio, None, None, 4, Some(16_000));
        assert_eq!(diagnostics.engine_name, "Gemini Live Cloud Audio");
        assert_eq!(diagnostics.model_id, None);
        assert_eq!(diagnostics.install_state, None);
        assert_eq!(diagnostics.cpu_threads, None);
        assert_eq!(diagnostics.queue_capacity, 0);
        assert!(!diagnostics.metrics_snapshot);
        assert_eq!(diagnostics.real_time_factor, None);
        assert_eq!(diagnostics.resident_memory_bytes, None);
        assert_eq!(diagnostics.dropped_chunks, 4);
    }
}
