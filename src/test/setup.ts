import "@testing-library/jest-dom";
import { vi } from "vitest";

// Mock Tauri internals in test environment
vi.mock("@tauri-apps/api/core", () => {
  const mockAudioDiagnostics = {
    configured_input_device: null,
    configured_output_device: null,
    microphone_permission: "granted",
    capture: {
      selected_device: "Test Microphone",
      sample_rate_hz: 48_000,
      sample_format: "F32",
      channels: 1,
      active: false,
      input_level: 0,
      dropped_chunks: 0,
      last_error: null,
    },
    playback: {
      selected_device: "Test Speakers",
      sample_rate_hz: 48_000,
      sample_format: "F32",
      channels: 2,
      playing: false,
      output_level: 0,
      queue_depth_samples: 0,
      queue_limit_samples: 480_000,
      dropped_samples: 0,
      last_error: null,
    },
  };

  return {
    invoke: vi.fn(async (cmd: string) => {
      if (cmd === "get_settings") {
        return {
          settings_version: 1,
          asr_mode: "moonshine_tiny_streaming",
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
          active_app_observation: false,
          window_title_observation: false,
          memory_enabled: false,
          save_transcripts: false,
          dry: 0.85,
          sarcastic: 0.7,
          friendly: 0.55,
          absurd: 0.65,
          helpful: 0.35,
          verbosity: 0.3,
        };
      }
      if (cmd === "get_microphone_permission") return "granted";
      if (cmd === "request_microphone_access") return "granted";
      if (cmd === "get_audio_diagnostics") return mockAudioDiagnostics;
      if (cmd === "test_microphone") {
        return { peak_level: 0.42, diagnostics: mockAudioDiagnostics };
      }
      if (cmd === "test_audio_output") return mockAudioDiagnostics;
      if (cmd === "get_character_state") return "idle";
      if (cmd === "is_muted") return false;
      if (cmd === "has_google_api_key") return true;
      if (cmd === "list_audio_devices") return [[], []];
      if (cmd === "get_memories") return [];
      if (cmd === "get_transcripts") return [];
      return null;
    }),
  };
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => {
    return () => {};
  }),
}));
