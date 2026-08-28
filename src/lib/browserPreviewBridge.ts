import type {
  AppSettings,
  AsrDiagnostics,
  AsrMode,
  AsrModelDescriptor,
  AsrModelProgressEvent,
  AudioDeviceInfo,
  AudioDiagnostics,
  CharacterState,
  ConnectionTestResult,
  ConversationLifecycle,
  GoogleModelDescriptor,
  GoogleTtsVoiceDescriptor,
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
import type { TauriBridge } from "./tauriBridge";

const previewAudioDiagnostics = (): AudioDiagnostics => ({
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

const previewAsrDiagnostics = (): AsrDiagnostics => ({
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

const previewAsrModels = (): AsrModelDescriptor[] => [
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

const previewModel = (
  mode: Exclude<AsrMode, "gemini_live_audio">,
): AsrModelDescriptor => {
  const model = previewAsrModels().find((candidate) => candidate.mode === mode);
  if (!model) throw new Error("Unknown local ASR model");
  return model;
};

/**
 * Development-only frontend preview adapter.
 *
 * These values intentionally simulate backend effects so `npm run dev` can
 * render and exercise the UI without Tauri. Production code may select this
 * adapter only through the compile-time Vite development gate in tauriBridge.
 */
export const browserPreviewBridge = {
  async resizeWindow(_width: number, _height: number): Promise<void> {},

  async getSettings(): Promise<AppSettings> {
    return frontendDefaultSettings();
  },

  async getOnboardingStatus(): Promise<OnboardingStatus> {
    return {
      current_version: 1,
      acknowledged_version: null,
      needs_acknowledgement: true,
    };
  },

  async acknowledgeOnboarding(): Promise<OnboardingStatus> {
    return {
      current_version: 1,
      acknowledged_version: 1,
      needs_acknowledgement: false,
    };
  },

  async updateSettings(_settings: AppSettings): Promise<void> {},

  async getGoogleModels(): Promise<GoogleModelDescriptor[]> {
    return frontendGoogleModels();
  },

  async getGoogleTtsVoices(): Promise<GoogleTtsVoiceDescriptor[]> {
    return frontendGoogleTtsVoices();
  },

  async getAsrModels(): Promise<AsrModelDescriptor[]> {
    return previewAsrModels();
  },

  async getAsrDiagnostics(): Promise<AsrDiagnostics> {
    return previewAsrDiagnostics();
  },

  async installAsrModel(
    mode: Exclude<AsrMode, "gemini_live_audio">,
  ): Promise<AsrModelDescriptor> {
    const model = previewModel(mode);
    return {
      ...model,
      install_state: "installed",
      installed_bytes: model.expected_bytes,
    };
  },

  async deleteAsrModel(
    mode: Exclude<AsrMode, "gemini_live_audio">,
  ): Promise<AsrModelDescriptor> {
    return previewModel(mode);
  },

  async onAsrModelProgress(
    _callback: (progress: AsrModelProgressEvent) => void,
  ): Promise<() => void> {
    return () => undefined;
  },

  async setGoogleApiKey(_apiKey: string): Promise<void> {},

  async clearGoogleApiKey(): Promise<void> {},

  async hasGoogleApiKey(): Promise<boolean> {
    return true;
  },

  async testAiConnection(): Promise<ConnectionTestResult> {
    return { success: true, message: "Mock API connection successful!" };
  },

  async listAudioDevices(): Promise<[AudioDeviceInfo[], AudioDeviceInfo[]]> {
    return [[], []];
  },

  async getMicrophonePermission(): Promise<MicrophonePermissionState> {
    return "unavailable";
  },

  async requestMicrophoneAccess(): Promise<MicrophonePermissionState> {
    return "unavailable";
  },

  async getToolAudit(): Promise<ToolAuditRecord[]> {
    return [];
  },

  async getAudioDiagnostics(): Promise<AudioDiagnostics> {
    return previewAudioDiagnostics();
  },

  async testMicrophone(): Promise<MicrophoneTestResult> {
    return { peak_level: 0, diagnostics: previewAudioDiagnostics() };
  },

  async testAudioOutput(): Promise<AudioDiagnostics> {
    return previewAudioDiagnostics();
  },

  async getCharacterState(): Promise<CharacterState> {
    return "idle";
  },

  async getConversationLifecycle(): Promise<ConversationLifecycle> {
    return "idle";
  },

  async setCharacterState(_newState: CharacterState): Promise<void> {},

  async triggerCannedReaction(_reactionType: string): Promise<string> {
    return "Hey! Nice clicking.";
  },

  async auditionVoice(voiceName: string): Promise<string> {
    return `Auditioning ${voiceName}`;
  },

  async cancelStandaloneSpeech(): Promise<void> {},

  async startConversation(): Promise<string> {
    return "mock-sess-123";
  },

  async stopConversation(): Promise<void> {},

  async bargeIn(): Promise<void> {},

  async setMute(_muted: boolean): Promise<void> {},

  async isMuted(): Promise<boolean> {
    return false;
  },

  async getMemories(): Promise<MemoryRecord[]> {
    return [];
  },

  async deleteMemory(_id: number): Promise<boolean> {
    return true;
  },

  async forgetEverything(): Promise<void> {},

  async getTranscripts(_limit = 50): Promise<TranscriptRecord[]> {
    return [];
  },

  async sendTextMessage(message: string): Promise<string> {
    return `Mock reply to: "${message}"`;
  },

  async listenEvent<T>(
    _eventName: string,
    _handler: (payload: T) => void,
  ): Promise<() => void> {
    return () => {};
  },
} satisfies TauriBridge;
