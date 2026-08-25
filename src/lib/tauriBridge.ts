import {
  AppSettings,
  AsrDiagnostics,
  AsrMode,
  AsrModelDescriptor,
  AsrModelProgressEvent,
  AudioDeviceInfo,
  AudioDiagnostics,
  CharacterState,
  ConnectionTestResult,
  GoogleModelDescriptor,
  GoogleTtsVoiceDescriptor,
  ConversationLifecycle,
  MemoryRecord,
  MicrophonePermissionState,
  MicrophoneTestResult,
  OnboardingStatus,
  TranscriptRecord,
  ToolAuditRecord,
} from "../types/moose";
import {
  frontendDefaultSettings,
  frontendGoogleModels,
  frontendGoogleTtsVoices,
} from "./backendContract";

// Check if running inside native Tauri runtime
export const isTauri = () => {
  return (
    typeof window !== "undefined" &&
    Boolean(
      (window as unknown as { __TAURI_INTERNALS__?: unknown })
        .__TAURI_INTERNALS__,
    )
  );
};

const mockAudioDiagnostics = (): AudioDiagnostics => ({
  configured_input_device: null,
  configured_output_device: null,
  microphone_permission: "unavailable",
  capture: {
    selected_device: null,
    sample_rate_hz: null,
    sample_format: null,
    channels: null,
    active: false,
    input_level: 0,
    dropped_chunks: 0,
    last_error: null,
  },
  playback: {
    selected_device: null,
    sample_rate_hz: null,
    sample_format: null,
    channels: null,
    playing: false,
    output_level: 0,
    queue_depth_samples: 0,
    queue_limit_samples: 0,
    dropped_samples: 0,
    last_error: null,
  },
});

const mockAsrDiagnostics = (): AsrDiagnostics => ({
  selected_mode: "moonshine_tiny_streaming",
  engine_name: "Moonshine Tiny Streaming (English)",
  model_id: "moonshine-tiny-streaming-en",
  model_revision: "quantized_26_07_30",
  install_state: "not_installed",
  input_sample_rate_hz: 16_000,
  streaming: false,
  metrics_snapshot: false,
  cpu_threads: null,
  queue_depth: 0,
  queue_capacity: 8,
  dropped_chunks: 0,
  last_error: null,
  first_partial_latency_ms: null,
  first_final_latency_ms: null,
  last_transcription_latency_ms: null,
  processed_audio_ms: 0,
  inference_wall_time_ms: 0,
  real_time_factor: null,
  process_cpu_time_ms: null,
  average_cpu_utilization_percent: null,
  baseline_resident_memory_bytes: null,
  resident_memory_bytes: null,
  peak_resident_memory_bytes: null,
});

const mockAsrModels = (): AsrModelDescriptor[] => [
  {
    id: "moonshine-tiny-streaming-en",
    display_name: "Moonshine Tiny Streaming (English)",
    mode: "moonshine_tiny_streaming",
    install_state: "not_installed",
    revision: "quantized_26_07_30",
    runtime_release: "v0.1.3",
    installed_bytes: null,
    expected_bytes: 51_441_771,
    active: false,
    error_message: null,
  },
  {
    id: "moonshine-small-streaming-en",
    display_name: "Moonshine Small Streaming (English)",
    mode: "moonshine_small_streaming",
    install_state: "not_installed",
    revision: "quantized_26_07_30",
    runtime_release: "v0.1.3",
    installed_bytes: null,
    expected_bytes: 165_489_086,
    active: false,
    error_message: null,
  },
];

export const tauriBridge = {
  async resizeWindow(width: number, height: number): Promise<void> {
    if (!isTauri()) return;
    try {
      const { getCurrentWindow, LogicalSize } =
        await import("@tauri-apps/api/window");
      const appWindow = getCurrentWindow();
      await appWindow.setSize(new LogicalSize(width, height));
    } catch (err) {
      console.error("Resize window error:", err);
    }
  },

  async getSettings(): Promise<AppSettings> {
    if (!isTauri()) return frontendDefaultSettings();
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AppSettings>("get_settings");
  },

  async getOnboardingStatus(): Promise<OnboardingStatus> {
    if (!isTauri()) {
      return {
        current_version: 1,
        acknowledged_version: null,
        needs_acknowledgement: true,
      };
    }
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<OnboardingStatus>("get_onboarding_status");
  },

  async acknowledgeOnboarding(): Promise<OnboardingStatus> {
    if (!isTauri()) {
      return {
        current_version: 1,
        acknowledged_version: 1,
        needs_acknowledgement: false,
      };
    }
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<OnboardingStatus>("acknowledge_onboarding");
  },

  async updateSettings(settings: AppSettings): Promise<void> {
    if (!isTauri()) return;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("update_settings", { newSettings: settings });
  },

  async getGoogleModels(): Promise<GoogleModelDescriptor[]> {
    if (!isTauri()) return frontendGoogleModels();
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<GoogleModelDescriptor[]>("get_google_models");
  },

  async getGoogleTtsVoices(): Promise<GoogleTtsVoiceDescriptor[]> {
    if (!isTauri()) return frontendGoogleTtsVoices();
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<GoogleTtsVoiceDescriptor[]>("get_google_tts_voices");
  },

  async getAsrModels(): Promise<AsrModelDescriptor[]> {
    if (!isTauri()) return mockAsrModels();
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AsrModelDescriptor[]>("get_asr_models");
  },

  async getAsrDiagnostics(): Promise<AsrDiagnostics> {
    if (!isTauri()) return mockAsrDiagnostics();
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AsrDiagnostics>("get_asr_diagnostics");
  },

  async installAsrModel(
    mode: Exclude<AsrMode, "gemini_live_audio">,
  ): Promise<AsrModelDescriptor> {
    if (!isTauri()) {
      const model = mockAsrModels().find(
        (candidate) => candidate.mode === mode,
      );
      if (!model) throw new Error("Unknown local ASR model");
      return {
        ...model,
        install_state: "installed",
        installed_bytes: model.expected_bytes,
      };
    }
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AsrModelDescriptor>("install_asr_model", { mode });
  },

  async deleteAsrModel(
    mode: Exclude<AsrMode, "gemini_live_audio">,
  ): Promise<AsrModelDescriptor> {
    if (!isTauri()) {
      const model = mockAsrModels().find(
        (candidate) => candidate.mode === mode,
      );
      if (!model) throw new Error("Unknown local ASR model");
      return model;
    }
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AsrModelDescriptor>("delete_asr_model", { mode });
  },

  async onAsrModelProgress(
    callback: (progress: AsrModelProgressEvent) => void,
  ): Promise<() => void> {
    if (!isTauri()) return () => undefined;
    const { listen } = await import("@tauri-apps/api/event");
    return listen<AsrModelProgressEvent>(
      "moose://asr/model-progress",
      (event) => callback(event.payload),
    );
  },

  async setGoogleApiKey(apiKey: string): Promise<void> {
    if (!isTauri()) return;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("set_google_api_key", { apiKey });
  },

  async clearGoogleApiKey(): Promise<void> {
    if (!isTauri()) return;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("clear_google_api_key");
  },

  async hasGoogleApiKey(): Promise<boolean> {
    if (!isTauri()) return true;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<boolean>("has_google_api_key");
  },

  async testAiConnection(): Promise<ConnectionTestResult> {
    if (!isTauri()) {
      return { success: true, message: "Mock API connection successful!" };
    }
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<ConnectionTestResult>("test_ai_connection");
  },

  async listAudioDevices(): Promise<[AudioDeviceInfo[], AudioDeviceInfo[]]> {
    if (!isTauri()) return [[], []];
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<[AudioDeviceInfo[], AudioDeviceInfo[]]>("list_audio_devices");
  },

  async getMicrophonePermission(): Promise<MicrophonePermissionState> {
    if (!isTauri()) return "unavailable";
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<MicrophonePermissionState>("get_microphone_permission");
  },

  async requestMicrophoneAccess(): Promise<MicrophonePermissionState> {
    if (!isTauri()) return "unavailable";
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<MicrophonePermissionState>("request_microphone_access");
  },

  async getToolAudit(): Promise<ToolAuditRecord[]> {
    if (!isTauri()) return [];
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<ToolAuditRecord[]>("get_tool_audit");
  },

  async getAudioDiagnostics(): Promise<AudioDiagnostics> {
    if (!isTauri()) return mockAudioDiagnostics();
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AudioDiagnostics>("get_audio_diagnostics");
  },

  async testMicrophone(): Promise<MicrophoneTestResult> {
    if (!isTauri()) {
      return { peak_level: 0, diagnostics: mockAudioDiagnostics() };
    }
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<MicrophoneTestResult>("test_microphone");
  },

  async testAudioOutput(): Promise<AudioDiagnostics> {
    if (!isTauri()) return mockAudioDiagnostics();
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AudioDiagnostics>("test_audio_output");
  },

  async getCharacterState(): Promise<CharacterState> {
    if (!isTauri()) return "idle";
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<CharacterState>("get_character_state");
  },

  async getConversationLifecycle(): Promise<ConversationLifecycle> {
    if (!isTauri()) return "idle";
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<ConversationLifecycle>("get_conversation_lifecycle");
  },

  async setCharacterState(newState: CharacterState): Promise<void> {
    if (!isTauri()) return;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("set_character_state", { newState });
  },

  async triggerCannedReaction(reactionType: string): Promise<string> {
    if (!isTauri()) return "Hey! Nice clicking.";
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("trigger_canned_reaction", { reactionType });
  },

  async auditionVoice(voiceName: string): Promise<string> {
    if (!isTauri()) return `Auditioning ${voiceName}`;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("audition_voice", { voiceName });
  },

  async cancelStandaloneSpeech(): Promise<void> {
    if (!isTauri()) return;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("cancel_standalone_speech");
  },

  async startConversation(): Promise<string> {
    if (!isTauri()) return "mock-sess-123";
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("start_conversation");
  },

  async stopConversation(): Promise<void> {
    if (!isTauri()) return;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("stop_conversation");
  },

  async bargeIn(): Promise<void> {
    if (!isTauri()) return;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("barge_in");
  },

  async setMute(muted: boolean): Promise<void> {
    if (!isTauri()) return;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("set_mute", { muted });
  },

  async isMuted(): Promise<boolean> {
    if (!isTauri()) return false;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<boolean>("is_muted");
  },

  async getMemories(): Promise<MemoryRecord[]> {
    if (!isTauri()) return [];
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<MemoryRecord[]>("get_memories");
  },

  async deleteMemory(id: number): Promise<boolean> {
    if (!isTauri()) return true;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<boolean>("delete_memory", { id });
  },

  async forgetEverything(): Promise<void> {
    if (!isTauri()) return;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("forget_everything");
  },

  async getTranscripts(limit = 50): Promise<TranscriptRecord[]> {
    if (!isTauri()) return [];
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<TranscriptRecord[]>("get_transcripts", { limit });
  },

  async sendTextMessage(message: string): Promise<string> {
    if (!isTauri()) return `Mock reply to: "${message}"`;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("send_text_message", { message });
  },

  async listenEvent<T>(
    eventName: string,
    handler: (payload: T) => void,
  ): Promise<() => void> {
    if (!isTauri()) return () => {};
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<T>(eventName, (event) =>
      handler(event.payload),
    );
    return unlisten;
  },
};
