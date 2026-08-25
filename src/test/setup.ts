import "@testing-library/jest-dom";
import { vi } from "vitest";
import {
  frontendDefaultSettings,
  frontendGoogleModels,
} from "../lib/backendContract";

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
        return frontendDefaultSettings();
      }
      if (cmd === "get_microphone_permission") return "granted";
      if (cmd === "request_microphone_access") return "granted";
      if (cmd === "get_audio_diagnostics") return mockAudioDiagnostics;
      if (cmd === "test_microphone") {
        return { peak_level: 0.42, diagnostics: mockAudioDiagnostics };
      }
      if (cmd === "test_audio_output") return mockAudioDiagnostics;
      if (cmd === "get_character_state") return "idle";
      if (cmd === "get_conversation_lifecycle") return "idle";
      if (cmd === "is_muted") return false;
      if (cmd === "has_google_api_key") return true;
      if (cmd === "list_audio_devices") return [[], []];
      if (cmd === "get_memories") return [];
      if (cmd === "get_google_models") return frontendGoogleModels();
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
