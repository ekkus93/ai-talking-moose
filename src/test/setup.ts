import "@testing-library/jest-dom";
import { vi } from "vitest";

// Mock Tauri internals in test environment
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === "get_settings") {
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
    if (cmd === "get_character_state") return "idle";
    if (cmd === "is_muted") return false;
    if (cmd === "has_google_api_key") return true;
    if (cmd === "list_audio_devices") return [[], []];
    if (cmd === "get_memories") return [];
    if (cmd === "get_transcripts") return [];
    return null;
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => {
    return () => {};
  }),
}));

// Mock SpeechSynthesis
if (typeof window !== "undefined") {
  window.speechSynthesis = {
    speak: vi.fn(),
    cancel: vi.fn(),
    pause: vi.fn(),
    resume: vi.fn(),
    getVoices: vi.fn(() => []),
    pending: false,
    speaking: false,
    paused: false,
    onvoiceschanged: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  } as unknown as SpeechSynthesis;

  window.SpeechSynthesisUtterance = vi.fn().mockImplementation((text) => ({
    text,
    pitch: 1,
    rate: 1,
    volume: 1,
    voice: null,
    lang: "en-US",
    onstart: null,
    onend: null,
    onerror: null,
    onpause: null,
    onresume: null,
    onmark: null,
    onboundary: null,
  })) as unknown as typeof SpeechSynthesisUtterance;
}
