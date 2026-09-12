import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LocalTtsDiagnosticsPanel } from "../components/Settings/LocalTtsDiagnosticsPanel";
import { tauriBridge } from "../lib/tauriBridge";
import type { LocalTtsDiagnostics } from "../types/localTts";

const MODEL_ID = "KittenML/kitten-tts-mini-0.8";
const VOICE_ID = "Jasper";
const SENTINEL = "KCR-130-UTTERANCE-SENTINEL-DO-NOT-EXPOSE";

const diagnostics: LocalTtsDiagnostics = {
  provider: "local",
  selected_model_id: MODEL_ID,
  selected_voice_id: VOICE_ID,
  install_state: "installed",
  expected_bytes: 93_604_191,
  installed_bytes: 93_604_191,
  installer_error_category: null,
  installer_error_retryable: null,
  runtime: {
    selected_model_id: MODEL_ID,
    loaded_model_id: MODEL_ID,
    loaded_revision: "revision",
    runtime_compatibility_version: 1,
    phase: "ready",
    sample_rate_hz: 24_000,
    inference_thread_count: 2,
    last_model_load_duration_ms: 125,
    last_synthesis_duration_ms: 50,
    last_generated_audio_duration_ms: 500,
    last_real_time_factor: 0.1,
    last_error_category: null,
  },
};

describe("Local TTS diagnostics surface", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders provider/model/voice/runtime positive controls without sensitive payload fields", async () => {
    vi.spyOn(tauriBridge, "getLocalTtsDiagnostics").mockResolvedValue(
      diagnostics,
    );

    render(<LocalTtsDiagnosticsPanel />);

    expect(await screen.findByText(MODEL_ID)).toBeInTheDocument();
    expect(screen.getByText(VOICE_ID)).toBeInTheDocument();
    expect(screen.getByText("ready")).toBeInTheDocument();
    expect(screen.getByText("24000 Hz")).toBeInTheDocument();
    expect(screen.getByText("0.100")).toBeInTheDocument();
    expect(screen.queryByText(SENTINEL)).not.toBeInTheDocument();
  });

  it("does not render raw IPC failure text", async () => {
    vi.spyOn(tauriBridge, "getLocalTtsDiagnostics").mockRejectedValue(
      new Error(SENTINEL),
    );

    render(<LocalTtsDiagnosticsPanel />);

    expect(
      await screen.findByText("Local TTS diagnostics unavailable."),
    ).toBeInTheDocument();
    expect(screen.queryByText(SENTINEL)).not.toBeInTheDocument();
  });
});
