import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AsrSettingsPanel } from "../components/Settings/AsrSettingsPanel";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";
import type { AsrModelDescriptor, AsrModelProgressEvent } from "../types/moose";

const whisper: AsrModelDescriptor = {
  id: "whisper-small-ggml",
  display_name: "Whisper Small",
  mode: "whisper_small",
  install_state: "installed",
  revision: "60c0be6ac8fa71b1a2ae2dd938a31a34a508e774",
  runtime_release: "60c0be6ac8fa71b1a2ae2dd938a31a34a508e774",
  installed_bytes: 10,
  expected_bytes: 100,
  active: false,
  error_message: null,
};

const models = [whisper];

describe("AsrSettingsPanel accessibility", () => {
  let progressListener: ((event: AsrModelProgressEvent) => void) | null = null;

  beforeEach(() => {
    useMooseStore.setState({ settings: frontendDefaultSettings() });
    vi.spyOn(tauriBridge, "getAsrModels").mockResolvedValue(models);
    vi.spyOn(tauriBridge, "getAsrDiagnostics").mockResolvedValue({
      selected_mode: "whisper_small",
      engine_name: "Whisper Small",
      model_id: whisper.id,
      model_revision: whisper.revision,
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

  it("offers only Whisper local ASR and explicitly selected Gemini Live", async () => {
    render(<AsrSettingsPanel />);

    expect(
      await screen.findByRole("radio", { name: /Whisper Small/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("radio", { name: /Gemini Live Cloud Audio/i }),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Moonshine/i)).not.toBeInTheDocument();
    expect(screen.getAllByRole("radio")).toHaveLength(2);
  });

  it("shows progress and refresh controls", async () => {
    render(<AsrSettingsPanel />);

    expect(
      await screen.findByRole("button", {
        name: "Re-check local ASR model integrity",
      }),
    ).toBeInTheDocument();
    expect(
      await screen.findByRole("button", {
        name: "Refresh ASR diagnostics",
      }),
    ).toBeInTheDocument();

    await act(async () => {
      progressListener?.({
        mode: "whisper_small",
        install_state: "downloading",
        downloaded_bytes: 25,
        total_bytes: 100,
        current_file: "model.bin",
      });
    });

    const progress = screen.getByRole("progressbar", {
      name: "Whisper Small download progress",
    });
    expect(progress).toHaveAttribute("aria-valuenow", "25");
    expect(progress).toHaveAttribute("aria-valuetext", "25% downloaded");
  });

  it("uses truthful Whisper wording", async () => {
    render(<AsrSettingsPanel />);

    expect(await screen.findByText(/local endpointing/i)).toBeInTheDocument();
    expect(screen.getByText(/utterance ends/i)).toBeInTheDocument();

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
    expect(screen.queryByText(/CRC32C/)).not.toBeInTheDocument();
  });

  it("lets the user cancel an active Whisper model download", async () => {
    vi.spyOn(tauriBridge, "getAsrModels").mockResolvedValue([
      { ...whisper, install_state: "not_installed", installed_bytes: null },
    ]);
    let rejectInstall: ((reason?: unknown) => void) | undefined;
    vi.spyOn(tauriBridge, "installAsrModel").mockImplementation(
      () =>
        new Promise((_resolve, reject) => {
          rejectInstall = reject;
        }),
    );
    const cancelInstall = vi
      .spyOn(tauriBridge, "cancelWhisperAsrModelInstall")
      .mockResolvedValue(true);
    render(<AsrSettingsPanel />);

    const downloadButtons = await screen.findAllByRole("button", {
      name: "Download",
    });
    await act(async () => {
      fireEvent.click(downloadButtons[0]);
    });
    await screen.findByRole("button", { name: "Cancel download" });

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Cancel download" }));
      await Promise.resolve();
    });
    expect(cancelInstall).toHaveBeenCalledOnce();

    await act(async () => {
      rejectInstall?.(new Error("The Whisper model download was cancelled."));
    });
    expect(
      await screen.findByText("Whisper model download canceled."),
    ).toBeInTheDocument();
  });
});
