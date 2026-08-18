import {
  AppSettings,
  CharacterState,
  ConnectionTestResult,
  MemoryRecord,
  TranscriptRecord,
  AudioDeviceInfo,
} from "../types/moose";

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
    if (!isTauri()) {
      return {
        launch_at_login: false,
        show_in_menu_bar: true,
        always_on_top: false,
        restore_position: true,
        unsolicited_comments: true,
        talkativeness: 0.5,
        quiet_hours_enabled: true,
        quiet_hours_start: 22,
        quiet_hours_end: 8,
        max_comments_per_hour: 4,
        hide_delay_seconds: 6,
        input_device: null,
        output_device: null,
        volume: 1.0,
        tts_voice: "Fenrir",
        speaking_rate: 0.95,
        pitch: -1.5,
        provider: "google",
        live_model: "gemini-2.5-flash-native-audio-latest",
        text_model: "gemini-2.5-flash",
        tts_model: "en-US-Standard-B",
        microphone_permission_granted: true,
        active_app_observation: true,
        window_title_observation: false,
        memory_enabled: true,
        save_transcripts: true,
        dry: 0.85,
        sarcastic: 0.7,
        friendly: 0.55,
        absurd: 0.65,
        helpful: 0.35,
        verbosity: 0.3,
      };
    }
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<AppSettings>("get_settings");
  },

  async updateSettings(settings: AppSettings): Promise<void> {
    if (!isTauri()) return;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("update_settings", { newSettings: settings });
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
    if (!isTauri()) {
      const inputs: AudioDeviceInfo[] = [
        {
          id: "mock_mic",
          name: "Default Internal Microphone",
          is_default: true,
        },
      ];
      const outputs: AudioDeviceInfo[] = [
        {
          id: "mock_speakers",
          name: "Default Internal Speakers",
          is_default: true,
        },
      ];
      return [inputs, outputs];
    }
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<[AudioDeviceInfo[], AudioDeviceInfo[]]>("list_audio_devices");
  },

  async getCharacterState(): Promise<CharacterState> {
    if (!isTauri()) return "idle";
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<CharacterState>("get_character_state");
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

  async triggerAmbientRemark(eventSummary: string): Promise<string | null> {
    if (!isTauri()) return "Did you really just switch applications 8 times?";
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string | null>("trigger_ambient_remark", { eventSummary });
  },

  async auditionVoice(voiceName: string): Promise<string> {
    if (!isTauri()) return `Auditioning ${voiceName}`;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("audition_voice", { voiceName });
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
