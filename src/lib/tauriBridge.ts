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
  LocalModelDescriptor,
  LocalModelDiagnostics,
  LocalModelInstallProgress,
  MemoryRecord,
  MicrophonePermissionState,
  MicrophoneTestResult,
  OnboardingStatus,
  TranscriptRecord,
  ToolAuditRecord,
} from "../types/moose";
import { browserPreviewBridge } from "./browserPreviewBridge";

// Tauri 2 exposes its low-level IPC function through this internal object.
// Checking the function rather than mere object presence makes a malformed or
// partially injected browser global fail closed instead of looking native.
export const isTauri = () => {
  if (typeof window === "undefined") return false;
  const internals = (
    window as unknown as {
      __TAURI_INTERNALS__?: { invoke?: unknown };
    }
  ).__TAURI_INTERNALS__;
  return typeof internals?.invoke === "function";
};

export const nativeTauriBridge = {
  async resizeWindow(width: number, height: number): Promise<void> {
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
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AppSettings>("get_settings");
  },

  async getOnboardingStatus(): Promise<OnboardingStatus> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<OnboardingStatus>("get_onboarding_status");
  },

  async acknowledgeOnboarding(): Promise<OnboardingStatus> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<OnboardingStatus>("acknowledge_onboarding");
  },

  async updateSettings(settings: AppSettings): Promise<void> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("update_settings", { newSettings: settings });
  },

  async getGoogleModels(): Promise<GoogleModelDescriptor[]> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<GoogleModelDescriptor[]>("get_google_models");
  },

  async getGoogleTtsVoices(): Promise<GoogleTtsVoiceDescriptor[]> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<GoogleTtsVoiceDescriptor[]>("get_google_tts_voices");
  },

  async getAsrModels(): Promise<AsrModelDescriptor[]> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AsrModelDescriptor[]>("get_asr_models");
  },

  async getAsrDiagnostics(): Promise<AsrDiagnostics> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AsrDiagnostics>("get_asr_diagnostics");
  },

  async installAsrModel(
    mode: Exclude<AsrMode, "gemini_live_audio">,
  ): Promise<AsrModelDescriptor> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AsrModelDescriptor>("install_asr_model", { mode });
  },

  async deleteAsrModel(
    mode: Exclude<AsrMode, "gemini_live_audio">,
  ): Promise<AsrModelDescriptor> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AsrModelDescriptor>("delete_asr_model", { mode });
  },

  async onAsrModelProgress(
    callback: (progress: AsrModelProgressEvent) => void,
  ): Promise<() => void> {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<AsrModelProgressEvent>(
      "moose://asr/model-progress",
      (event) => callback(event.payload),
    );
  },

  async getLocalLlmModels(): Promise<LocalModelDescriptor[]> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<LocalModelDescriptor[]>("get_local_llm_models");
  },

  async getLocalLlmDiagnostics(): Promise<LocalModelDiagnostics> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<LocalModelDiagnostics>("get_local_llm_diagnostics");
  },

  async installLocalLlmModel(modelId: string): Promise<LocalModelDescriptor> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<LocalModelDescriptor>("install_local_llm_model", {
      modelId,
    });
  },

  async cancelLocalLlmInstall(modelId: string): Promise<boolean> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<boolean>("cancel_local_llm_install", { modelId });
  },

  async deleteLocalLlmModel(modelId: string): Promise<LocalModelDescriptor> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<LocalModelDescriptor>("delete_local_llm_model", { modelId });
  },

  async testLocalLlmModel(): Promise<ConnectionTestResult> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<ConnectionTestResult>("test_local_llm_model");
  },

  async onLocalLlmModelProgress(
    callback: (progress: LocalModelInstallProgress) => void,
  ): Promise<() => void> {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<LocalModelInstallProgress>(
      "moose://local-llm/model-progress",
      (event) => callback(event.payload),
    );
  },

  async setGoogleApiKey(apiKey: string): Promise<void> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("set_google_api_key", { apiKey });
  },

  async clearGoogleApiKey(): Promise<void> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("clear_google_api_key");
  },

  async hasGoogleApiKey(): Promise<boolean> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<boolean>("has_google_api_key");
  },

  async testAiConnection(): Promise<ConnectionTestResult> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<ConnectionTestResult>("test_ai_connection");
  },

  async listAudioDevices(): Promise<[AudioDeviceInfo[], AudioDeviceInfo[]]> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<[AudioDeviceInfo[], AudioDeviceInfo[]]>("list_audio_devices");
  },

  async getMicrophonePermission(): Promise<MicrophonePermissionState> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<MicrophonePermissionState>("get_microphone_permission");
  },

  async requestMicrophoneAccess(): Promise<MicrophonePermissionState> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<MicrophonePermissionState>("request_microphone_access");
  },

  async getToolAudit(): Promise<ToolAuditRecord[]> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<ToolAuditRecord[]>("get_tool_audit");
  },

  async getAudioDiagnostics(): Promise<AudioDiagnostics> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AudioDiagnostics>("get_audio_diagnostics");
  },

  async testMicrophone(): Promise<MicrophoneTestResult> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<MicrophoneTestResult>("test_microphone");
  },

  async testAudioOutput(): Promise<AudioDiagnostics> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AudioDiagnostics>("test_audio_output");
  },

  async getCharacterState(): Promise<CharacterState> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<CharacterState>("get_character_state");
  },

  async getConversationLifecycle(): Promise<ConversationLifecycle> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<ConversationLifecycle>("get_conversation_lifecycle");
  },

  async setCharacterState(newState: CharacterState): Promise<void> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("set_character_state", { newState });
  },

  async triggerCannedReaction(reactionType: string): Promise<string> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("trigger_canned_reaction", { reactionType });
  },

  async auditionVoice(voiceName: string): Promise<string> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("audition_voice", { voiceName });
  },

  async cancelStandaloneSpeech(): Promise<void> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("cancel_standalone_speech");
  },

  async startConversation(): Promise<string> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("start_conversation");
  },

  async stopConversation(): Promise<void> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("stop_conversation");
  },

  async bargeIn(): Promise<void> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("barge_in");
  },

  async setMute(muted: boolean): Promise<void> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("set_mute", { muted });
  },

  async isMuted(): Promise<boolean> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<boolean>("is_muted");
  },

  async getMemories(): Promise<MemoryRecord[]> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<MemoryRecord[]>("get_memories");
  },

  async deleteMemory(id: number): Promise<boolean> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<boolean>("delete_memory", { id });
  },

  async forgetEverything(): Promise<void> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("forget_everything");
  },

  async getTranscripts(limit = 50): Promise<TranscriptRecord[]> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<TranscriptRecord[]>("get_transcripts", { limit });
  },

  async sendTextMessage(message: string): Promise<string> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("send_text_message", { message });
  },

  async listenEvent<T>(
    eventName: string,
    handler: (payload: T) => void,
  ): Promise<() => void> {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<T>(eventName, (event) =>
      handler(event.payload),
    );
    return unlisten;
  },
};

export type TauriBridge = typeof nativeTauriBridge;

export const selectTauriBridge = (
  tauriAvailable: boolean,
  developmentPreviewAllowed: boolean,
): TauriBridge => {
  if (tauriAvailable) return nativeTauriBridge;
  if (developmentPreviewAllowed) return browserPreviewBridge;
  throw new Error(
    "Tauri IPC is unavailable and the development-only browser preview adapter is disabled.",
  );
};

// `import.meta.env.DEV` is replaced by Vite at build time. It is not a query
// parameter, localStorage value, browser global, or user-configurable runtime
// switch. A production bundle therefore cannot opt into fabricated preview
// behavior when Tauri IPC is absent or malformed.
const developmentPreviewAllowed = import.meta.env.DEV;

export const tauriBridge = selectTauriBridge(
  isTauri(),
  developmentPreviewAllowed,
);
