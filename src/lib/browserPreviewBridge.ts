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
  LocalLlmDiagnostics,
  LocalModelDescriptor,
  LocalModelInstallProgress,
  MemoryRecord,
  MicrophonePermissionState,
  MicrophoneTestResult,
  OnboardingStatus,
  TranscriptRecord,
  ToolAuditRecord,
  TtsCatalog,
  TtsProvider,
} from "../types/moose";
import type {
  LocalTtsDiagnostics,
  LocalTtsInstallProgress,
  LocalTtsModelDescriptor,
} from "../types/localTts";
import {
  frontendDefaultSettings,
  frontendGoogleModels,
  frontendGoogleTtsVoices,
  frontendTtsCatalog,
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

const previewLocalLlmModels = (): LocalModelDescriptor[] => {
  const selected = frontendDefaultSettings().local_text_model;
  return [
    {
      id: "smollm2-360m-instruct-q4-k-m",
      display_name: "SmolLM2 360M Instruct (Q4_K_M)",
      family: "SmolLM2",
      parameter_scale: "360M",
      quantization: "Q4_K_M",
      revision: "ab928a97ee49f3a015f35194879f68211291d6ca",
      expected_bytes: 270_590_880,
      installed_bytes: null,
      license: "Apache-2.0",
      context_limit: 8_192,
      recommended_max_output: 192,
      install_state: "not_installed",
      active: selected === "smollm2-360m-instruct-q4-k-m",
      error: null,
    },
    {
      id: "qwen3-0-6b-instruct-q4-k-m",
      display_name: "Qwen3 0.6B (Q4_K_M, non-thinking)",
      family: "Qwen3",
      parameter_scale: "0.6B",
      quantization: "Q4_K_M",
      revision: "7bcae0bc7b0606f1e948f8cdb31b98a2c10635db",
      expected_bytes: 484_220_320,
      installed_bytes: null,
      license: "Apache-2.0",
      context_limit: 32_768,
      recommended_max_output: 192,
      install_state: "not_installed",
      active: selected === "qwen3-0-6b-instruct-q4-k-m",
      error: null,
    },
  ];
};

const previewLocalLlmModel = (modelId: string): LocalModelDescriptor => {
  const model = previewLocalLlmModels().find(
    (candidate) => candidate.id === modelId,
  );
  if (!model) throw new Error("Unknown local LLM model");
  return model;
};

const previewLocalTtsModels = (): LocalTtsModelDescriptor[] => {
  const settings = frontendDefaultSettings();
  const localProvider = frontendTtsCatalog().providers.find(
    (provider) => provider.id === "local",
  );
  const model = localProvider?.models[0];
  if (!model) return [];
  return [
    {
      id: model.id,
      display_name: model.display_name,
      version: "0.8",
      expected_bytes: 93_604_191,
      installed_bytes: null,
      license: "Apache-2.0",
      install_state: "not_installed",
      active: settings.local_tts_model === model.id,
      error: null,
    },
  ];
};

const previewLocalTtsModel = (modelId: string): LocalTtsModelDescriptor => {
  const model = previewLocalTtsModels().find(
    (candidate) => candidate.id === modelId,
  );
  if (!model) throw new Error("Unknown Local TTS model");
  return model;
};

const previewLocalTtsDiagnostics = (): LocalTtsDiagnostics => {
  const settings = frontendDefaultSettings();
  const model = previewLocalTtsModel(settings.local_tts_model);
  return {
    provider: settings.tts_provider,
    selected_model_id: settings.local_tts_model,
    selected_voice_id: settings.local_tts_voice,
    install_state: model.install_state,
    expected_bytes: model.expected_bytes,
    installed_bytes: model.installed_bytes,
    installer_error_category: null,
    installer_error_retryable: null,
    runtime: {
      selected_model_id: settings.local_tts_model,
      loaded_model_id: null,
      loaded_revision: null,
      runtime_compatibility_version: null,
      phase: "unloaded",
      sample_rate_hz: 24_000,
      inference_thread_count: 2,
      last_model_load_duration_ms: null,
      last_synthesis_duration_ms: null,
      last_generated_audio_duration_ms: null,
      last_real_time_factor: null,
      last_error_category: null,
    },
  };
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

  async getTtsCatalog(): Promise<TtsCatalog> {
    return frontendTtsCatalog();
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

  async getLocalLlmModels(): Promise<LocalModelDescriptor[]> {
    return previewLocalLlmModels();
  },

  async getLocalLlmDiagnostics(): Promise<LocalLlmDiagnostics> {
    const selectedModelId = frontendDefaultSettings().local_text_model;
    return {
      installer: {
        model_root_ready: true,
        installs_in_progress: 0,
        last_error: null,
      },
      selected_install_state: "not_installed",
      runtime: {
        selected_model_id: selectedModelId,
        loaded_model_id: null,
        loaded_revision: null,
        loaded_quantization: null,
        loaded: false,
        phase: "ready",
        thread_count: 2,
        context_size: 4_096,
        generation_in_progress: false,
        last_error_category: null,
        last_generation_duration_ms: null,
        last_prompt_tokens: null,
        last_output_tokens: null,
        last_tokens_per_second: null,
      },
    };
  },

  async installLocalLlmModel(modelId: string): Promise<LocalModelDescriptor> {
    const model = previewLocalLlmModel(modelId);
    return {
      ...model,
      install_state: "installed",
      installed_bytes: model.expected_bytes,
    };
  },

  async cancelLocalLlmInstall(_modelId: string): Promise<boolean> {
    return false;
  },

  async deleteLocalLlmModel(modelId: string): Promise<LocalModelDescriptor> {
    return previewLocalLlmModel(modelId);
  },

  async testLocalLlmModel(): Promise<ConnectionTestResult> {
    return {
      success: false,
      message:
        "Install the selected local model before testing it in browser preview.",
    };
  },

  async onLocalLlmModelProgress(
    _callback: (progress: LocalModelInstallProgress) => void,
  ): Promise<() => void> {
    return () => undefined;
  },

  async getLocalTtsModels(): Promise<LocalTtsModelDescriptor[]> {
    return previewLocalTtsModels();
  },

  async getLocalTtsDiagnostics(): Promise<LocalTtsDiagnostics> {
    return previewLocalTtsDiagnostics();
  },

  async installLocalTtsModel(
    modelId: string,
  ): Promise<LocalTtsModelDescriptor> {
    const model = previewLocalTtsModel(modelId);
    return {
      ...model,
      install_state: "installed",
      installed_bytes: model.expected_bytes,
    };
  },

  async cancelLocalTtsInstall(_modelId: string): Promise<boolean> {
    return false;
  },

  async deleteLocalTtsModel(modelId: string): Promise<LocalTtsModelDescriptor> {
    return previewLocalTtsModel(modelId);
  },

  async onLocalTtsModelProgress(
    _callback: (progress: LocalTtsInstallProgress) => void,
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

  async auditionTtsVoice(
    provider: TtsProvider,
    voiceName: string,
  ): Promise<string> {
    return `Auditioning ${provider} voice ${voiceName}`;
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
