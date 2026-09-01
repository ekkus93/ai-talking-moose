import "@testing-library/jest-dom";
import { beforeEach, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  frontendDefaultSettings,
  frontendGoogleModels,
  frontendGoogleTtsVoices,
} from "../lib/backendContract";
import type {
  AsrModelDescriptor,
  AudioDiagnostics,
  LocalModelDescriptor,
} from "../types/moose";

// Production-like frontend tests must select the same Tauri/IPC branch as the
// packaged application. The API modules themselves remain mocked, so tests stay
// offline and hardware-free while exercising the real bridge command strings.
const mockAudioDiagnostics: AudioDiagnostics = {
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

const mockAsrModels: AsrModelDescriptor[] = [
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

const mockLocalModels: LocalModelDescriptor[] = [
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
    active: true,
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
    active: false,
    error: null,
  },
];

const modelForMode = (mode: unknown): AsrModelDescriptor => {
  const model = mockAsrModels.find((candidate) => candidate.mode === mode);
  if (!model) {
    throw new Error(`Unknown ASR mode in test fixture: ${String(mode)}`);
  }
  return { ...model };
};

const localModelForId = (modelId: unknown): LocalModelDescriptor => {
  const model = mockLocalModels.find((candidate) => candidate.id === modelId);
  if (!model) {
    throw new Error(
      `Unknown local LLM model in test fixture: ${String(modelId)}`,
    );
  }
  return { ...model };
};

const dispatchTauriCommand = async (
  cmd: string,
  args?: Record<string, unknown>,
): Promise<unknown> => {
  // Settings/onboarding: src-tauri/src/commands/settings.rs.
  if (cmd === "get_settings") return frontendDefaultSettings();
  if (cmd === "get_onboarding_status") {
    return {
      current_version: 1,
      acknowledged_version: null,
      needs_acknowledgement: true,
    };
  }
  if (cmd === "acknowledge_onboarding") {
    return {
      current_version: 1,
      acknowledged_version: 1,
      needs_acknowledgement: false,
    };
  }
  if (cmd === "update_settings") return undefined;
  if (cmd === "get_google_models") return frontendGoogleModels();
  if (cmd === "get_google_tts_voices") return frontendGoogleTtsVoices();
  if (cmd === "set_google_api_key") return undefined;
  if (cmd === "clear_google_api_key") return undefined;
  if (cmd === "has_google_api_key") return true;
  if (cmd === "test_ai_connection") {
    return { success: true, message: "Test connection succeeded" };
  }
  if (cmd === "list_audio_devices") return [[], []];
  if (cmd === "get_microphone_permission") return "granted";
  if (cmd === "request_microphone_access") return "granted";
  if (cmd === "get_audio_diagnostics") return mockAudioDiagnostics;
  if (cmd === "test_microphone") {
    return { peak_level: 0.42, diagnostics: mockAudioDiagnostics };
  }
  if (cmd === "test_audio_output") return mockAudioDiagnostics;

  // Local ASR: src-tauri/src/commands/asr_models.rs and asr_diagnostics.rs.
  if (cmd === "get_asr_models") {
    return mockAsrModels.map((model) => ({ ...model }));
  }
  if (cmd === "install_asr_model") {
    const model = modelForMode(args?.mode);
    return {
      ...model,
      install_state: "installed",
      installed_bytes: model.expected_bytes,
    };
  }
  if (cmd === "delete_asr_model") return modelForMode(args?.mode);
  if (cmd === "get_asr_diagnostics") {
    return {
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
    };
  }

  // Local LLM lifecycle: src-tauri/src/commands/local_llm_models.rs.
  if (cmd === "get_local_llm_models") {
    return mockLocalModels.map((model) => ({ ...model }));
  }
  if (cmd === "get_local_llm_diagnostics") {
    return {
      model_root_ready: true,
      installs_in_progress: 0,
      last_error: null,
    };
  }
  if (cmd === "install_local_llm_model") {
    const model = localModelForId(args?.modelId);
    return {
      ...model,
      install_state: "installed",
      installed_bytes: model.expected_bytes,
    };
  }
  if (cmd === "cancel_local_llm_install") return true;
  if (cmd === "delete_local_llm_model") return localModelForId(args?.modelId);
  if (cmd === "test_local_llm_model") {
    return { success: true, message: "Local model test succeeded" };
  }

  // Character/speech: src-tauri/src/commands/character.rs.
  if (cmd === "get_character_state") return "idle";
  if (cmd === "set_character_state") return undefined;
  if (cmd === "trigger_canned_reaction") return "Test canned reaction";
  if (cmd === "audition_voice") return "Test voice audition";
  if (cmd === "cancel_standalone_speech") return undefined;
  if (cmd === "set_mute") return undefined;
  if (cmd === "is_muted") return false;

  // Conversation/memory: src-tauri/src/commands/conversation/core.rs.
  if (cmd === "start_conversation") return "test-session";
  if (cmd === "stop_conversation") return undefined;
  if (cmd === "barge_in") return undefined;
  if (cmd === "get_conversation_lifecycle") return "idle";
  if (cmd === "get_memories") return [];
  if (cmd === "delete_memory") return true;
  if (cmd === "forget_everything") return undefined;
  if (cmd === "get_transcripts") return [];
  if (cmd === "send_text_message") return "Test text response";

  // Tool diagnostics: src-tauri/src/commands/tool_diagnostics.rs.
  if (cmd === "get_tool_audit") return [];

  throw new Error(`Unhandled mocked Tauri command: ${cmd}`);
};

Object.defineProperty(window, "__TAURI_INTERNALS__", {
  configurable: true,
  value: { invoke: dispatchTauriCommand },
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string, args?: Record<string, unknown>) =>
    (
      window as unknown as {
        __TAURI_INTERNALS__: {
          invoke: (
            command: string,
            commandArgs?: Record<string, unknown>,
          ) => Promise<unknown>;
        };
      }
    ).__TAURI_INTERNALS__.invoke(cmd, args),
  ),
}));

const defaultInvokeImplementation = vi.mocked(invoke).getMockImplementation();

beforeEach(() => {
  if (!defaultInvokeImplementation) {
    throw new Error(
      "Tauri invoke test fixture is missing its default implementation",
    );
  }
  vi.mocked(invoke).mockImplementation(defaultInvokeImplementation);
});

vi.mock("@tauri-apps/api/window", () => {
  class LogicalSize {
    constructor(
      public readonly width: number,
      public readonly height: number,
    ) {}
  }

  return {
    LogicalSize,
    getCurrentWindow: () => ({
      setSize: vi.fn(async () => undefined),
      startDragging: vi.fn(async () => undefined),
    }),
  };
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => {
    return () => {};
  }),
}));
