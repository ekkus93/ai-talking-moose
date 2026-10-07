import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AsrSettingsPanel } from "../components/Settings/AsrSettingsPanel";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";
import type { AsrModelDescriptor, AsrModelProgressEvent } from "../types/moose";

const tiny: AsrModelDescriptor = {
  id: "moonshine-tiny-streaming-en",
  display_name: "Moonshine Tiny Streaming",
  mode: "moonshine_tiny_streaming",
  install_state: "installed",
  revision: "test-revision",
  runtime_release: "test-runtime",
  installed_bytes: 10,
  expected_bytes: 100,
  active: false,
  error_message: null,
};

const small: AsrModelDescriptor = {
  ...tiny,
  id: "moonshine-small-streaming-en",
  display_name: "Moonshine Small Streaming",
  mode: "moonshine_small_streaming",
};

const whisper: AsrModelDescriptor = {
  ...tiny,
  id: "whisper-small-ggml",
  display_name: "Whisper Small",
  mode: "whisper_small",
  revision: "60c0be6ac8fa71b1a2ae2dd938a31a34a508e774",
  runtime_release: "60c0be6ac8fa71b1a2ae2dd938a31a34a508e774",
};

describe("AsrSettingsPanel accessibility", () => {
  let progressListener: ((event: AsrModelProgressEvent) => void) | null = null;

  beforeEach(() => {
    useMooseStore.setState({ settings: frontendDefaultSettings() });
    vi.spyOn(tauriBridge, "getAsrModels").mockResolvedValue([tiny, small, whisper]);
    vi.spyOn(tauriBridge, "getAsrDiagnostics").mockResolvedValue({
      selected_mode: "moonshine_tiny_streaming",
      engine_name: "Moonshine Tiny Streaming",
      model_id: tiny.id,
      model_revision: tiny.revision,
      install_state: "installed",
      input_sample_rate_hz: 16_000,
      streaming: false,
      metrics_snapshot: false,
      cpu_threads: null,
      queue_depth: 0,
      queue_capacity: 0,
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
    vi.spyOn(tauriBridge, "onAsrModelProgress").mockImplementation(
      async (listener) => {
        progressListener = listener;
        return () => undefined;
      },
    );
  });

  afterEach(() => {
    progressListener = null;
    vi.restoreAllMocks();
  });

  it("exposes numeric download progress and consistently labeled refresh controls", async () => {
    render(<AsrSettingsPanel />);
    expect(
      await screen.findByRole("button", {
        name: "Re-check local ASR model integrity",
      }),
    ).toBeInTheDocument();
    expect(
      await screen.findByRole("button", { name: "Refresh ASR diagnostics" }),
    ).toBeInTheDocument();

    await act(async () => {
      progressListener?.({
        mode: "moonshine_tiny_streaming",
        install_state: "downloading",
        downloaded_bytes: 25,
        total_bytes: 100,
        current_file: "model.bin",
      });
    });

    const progress = screen.getByRole("progressbar", {
      name: "Moonshine Tiny Streaming download progress",
    });
    expect(progress).toHaveAttribute("aria-valuemin", "0");
    expect(progress).toHaveAttribute("aria-valuemax", "100");
    expect(progress).toHaveAttribute("aria-valuenow", "25");
    expect(progress).toHaveAttribute("aria-valuetext", "25% downloaded");
  });
  it("describes Whisper finality truthfully and uses SHA-256-only verification wording", async () => {
    render(<AsrSettingsPanel />);

    expect(
      await screen.findByText(/in-utterance partial updates and local endpointing/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/final user turn is emitted only when the utterance ends/i),
    ).toBeInTheDocument();

    await act(async () => {
      progressListener?.({
        mode: "whisper_small",
        install_state: "verifying",
        downloaded_bytes: whisper.expected_bytes,
        total_bytes: whisper.expected_bytes,
        current_file: "ggml-small.bin",
      });
    });

    expect(
      screen.getByText("Verifying SHA-256 and install metadata…"),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("Verifying SHA-256/CRC32C and install metadata…"),
    ).not.toBeInTheDocument();
  });
});
