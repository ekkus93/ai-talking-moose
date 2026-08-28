use serde::Serialize;
use talking_moose_lib::ai::google::{
    GoogleModelDescriptor, GoogleTtsVoiceDescriptor, GOOGLE_MODELS, GOOGLE_TTS_VOICES,
};
use talking_moose_lib::ai::types::{ProviderError, ProviderErrorKind};
use talking_moose_lib::app::state::{AppSettings, OnboardingStatus};
use talking_moose_lib::asr::{
    AsrDiagnostics, AsrError, AsrErrorKind, AsrMode, AsrModelDescriptor, AsrModelInstallState,
};
use talking_moose_lib::audio::capture::AudioCaptureDiagnostics;
use talking_moose_lib::audio::devices::AudioDeviceInfo;
use talking_moose_lib::audio::permissions::MicrophonePermissionState;
use talking_moose_lib::audio::playback::AudioPlaybackDiagnostics;
use talking_moose_lib::commands::{
    AsrModelProgressEvent, AudioDiagnostics, ConnectionTestResult, MicrophoneTestResult,
};
use talking_moose_lib::persistence::sqlite::{MemoryRecord, TranscriptRecord};
use talking_moose_lib::tools::policy::{
    ToolAuditRecord, ToolPermissionLevel, ToolPermissionOutcome, ToolResultCategory,
};

#[derive(Serialize)]
struct FrontendContract<'a> {
    settings: AppSettings,
    google_models: &'a [GoogleModelDescriptor],
    google_tts_voices: &'a [GoogleTtsVoiceDescriptor],
    ipc_shapes: FrontendIpcShapes,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct FrontendIpcShapes {
    provider_error: ProviderError,
    asr_error: AsrError,
    asr_model_descriptor: AsrModelDescriptor,
    asr_diagnostics: AsrDiagnostics,
    asr_model_progress_event: AsrModelProgressEvent,
    audio_device_info: AudioDeviceInfo,
    audio_capture_diagnostics: AudioCaptureDiagnostics,
    audio_playback_diagnostics: AudioPlaybackDiagnostics,
    tool_audit_record: ToolAuditRecord,
    audio_diagnostics: AudioDiagnostics,
    microphone_test_result: MicrophoneTestResult,
    onboarding_status: OnboardingStatus,
    app_settings: AppSettings,
    memory_record: MemoryRecord,
    transcript_record: TranscriptRecord,
    google_model_descriptor: GoogleModelDescriptor,
    google_tts_voice_descriptor: GoogleTtsVoiceDescriptor,
    connection_test_result: ConnectionTestResult,
}

fn representative_settings() -> AppSettings {
    AppSettings {
        input_device: Some("Contract input device".to_string()),
        output_device: Some("Contract output device".to_string()),
        ..Default::default()
    }
}

fn representative_capture_diagnostics() -> AudioCaptureDiagnostics {
    AudioCaptureDiagnostics {
        selected_device: Some("Contract microphone".to_string()),
        sample_rate_hz: Some(48_000),
        sample_format: Some("F32".to_string()),
        channels: Some(1),
        active: true,
        input_level: 0.25,
        dropped_chunks: 1,
        last_error: Some("contract capture error".to_string()),
    }
}

fn representative_playback_diagnostics() -> AudioPlaybackDiagnostics {
    AudioPlaybackDiagnostics {
        selected_device: Some("Contract speakers".to_string()),
        sample_rate_hz: Some(48_000),
        sample_format: Some("F32".to_string()),
        channels: Some(2),
        playing: true,
        output_level: 0.5,
        queue_depth_samples: 128,
        queue_limit_samples: 1_024,
        dropped_samples: 2,
        last_error: Some("contract playback error".to_string()),
    }
}

fn representative_audio_diagnostics() -> AudioDiagnostics {
    AudioDiagnostics {
        configured_input_device: Some("Contract input device".to_string()),
        configured_output_device: Some("Contract output device".to_string()),
        microphone_permission: MicrophonePermissionState::Granted,
        capture: representative_capture_diagnostics(),
        playback: representative_playback_diagnostics(),
    }
}

fn representative_ipc_shapes() -> FrontendIpcShapes {
    let asr_error = AsrError {
        kind: AsrErrorKind::Inference,
        message: "contract ASR error".to_string(),
        retryable: true,
    };
    let audio_diagnostics = representative_audio_diagnostics();

    FrontendIpcShapes {
        provider_error: ProviderError {
            kind: ProviderErrorKind::Network,
            message: "contract provider error".to_string(),
            retryable: true,
        },
        asr_error: asr_error.clone(),
        asr_model_descriptor: AsrModelDescriptor {
            id: "contract-asr-model".to_string(),
            display_name: "Contract ASR model".to_string(),
            mode: AsrMode::MoonshineTinyStreaming,
            install_state: AsrModelInstallState::Installed,
            revision: "contract-revision".to_string(),
            runtime_release: "contract-runtime".to_string(),
            installed_bytes: Some(128),
            expected_bytes: 256,
            active: true,
            error_message: Some("contract model error".to_string()),
        },
        asr_diagnostics: AsrDiagnostics {
            selected_mode: AsrMode::MoonshineTinyStreaming,
            engine_name: "Contract ASR engine".to_string(),
            model_id: Some("contract-asr-model".to_string()),
            model_revision: Some("contract-revision".to_string()),
            install_state: Some(AsrModelInstallState::Installed),
            input_sample_rate_hz: 16_000,
            streaming: true,
            metrics_snapshot: true,
            cpu_threads: Some(2),
            queue_depth: 1,
            queue_capacity: 8,
            dropped_chunks: 1,
            last_error: Some(asr_error),
            first_partial_latency_ms: Some(10),
            first_final_latency_ms: Some(20),
            last_transcription_latency_ms: Some(30),
            processed_audio_ms: 1_000,
            inference_wall_time_ms: 100,
            real_time_factor: Some(0.1),
            process_cpu_time_ms: Some(50),
            average_cpu_utilization_percent: Some(25.0),
            baseline_resident_memory_bytes: Some(1_000),
            resident_memory_bytes: Some(2_000),
            peak_resident_memory_bytes: Some(3_000),
        },
        asr_model_progress_event: AsrModelProgressEvent {
            mode: AsrMode::MoonshineTinyStreaming,
            install_state: AsrModelInstallState::Downloading,
            downloaded_bytes: 128,
            total_bytes: 256,
            current_file: Some("contract-model.bin".to_string()),
        },
        audio_device_info: AudioDeviceInfo {
            id: "contract-device".to_string(),
            name: "Contract device".to_string(),
            is_default: true,
        },
        audio_capture_diagnostics: representative_capture_diagnostics(),
        audio_playback_diagnostics: representative_playback_diagnostics(),
        tool_audit_record: ToolAuditRecord {
            tool_name: "contract_tool".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            duration_ms: 1,
            permission: ToolPermissionLevel::SafeReadOnly,
            permission_outcome: ToolPermissionOutcome::Allowed,
            result_category: ToolResultCategory::Success,
        },
        audio_diagnostics: audio_diagnostics.clone(),
        microphone_test_result: MicrophoneTestResult {
            peak_level: 0.5,
            diagnostics: audio_diagnostics,
        },
        onboarding_status: OnboardingStatus {
            current_version: 1,
            acknowledged_version: Some(1),
            needs_acknowledgement: false,
        },
        app_settings: representative_settings(),
        memory_record: MemoryRecord {
            id: 1,
            fact: "contract memory".to_string(),
            category: "contract".to_string(),
            source: "contract".to_string(),
            confidence: 1.0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        },
        transcript_record: TranscriptRecord {
            id: 1,
            session_id: "contract-session".to_string(),
            role: "user".to_string(),
            text: "contract transcript".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        google_model_descriptor: GOOGLE_MODELS[0],
        google_tts_voice_descriptor: GOOGLE_TTS_VOICES[0],
        connection_test_result: ConnectionTestResult {
            success: true,
            message: "contract connection result".to_string(),
        },
    }
}

fn main() {
    let contract = FrontendContract {
        settings: AppSettings::default(),
        google_models: GOOGLE_MODELS,
        google_tts_voices: GOOGLE_TTS_VOICES,
        ipc_shapes: representative_ipc_shapes(),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&contract).expect("frontend contract must serialize")
    );
}
