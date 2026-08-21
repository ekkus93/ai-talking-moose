import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AsrDiagnosticsPanel } from "../components/Settings/AsrDiagnosticsPanel";
import { tauriBridge } from "../lib/tauriBridge";

const diagnostics = {
  selected_mode: "moonshine_tiny_streaming" as const,
  engine_name: "Moonshine Tiny Streaming (English)",
  model_id: "moonshine-tiny-streaming-en",
  model_revision: "quantized_26_07_30",
  install_state: "installed" as const,
  input_sample_rate_hz: 16_000,
  streaming: false,
  metrics_snapshot: true,
  cpu_threads: 8,
  queue_depth: 0,
  queue_capacity: 8,
  dropped_chunks: 0,
  last_error: {
    kind: "inference" as const,
    message: "previous session stopped during inference",
    retryable: true,
  },
  first_partial_latency_ms: 42,
  first_final_latency_ms: 96,
  last_transcription_latency_ms: 12,
  processed_audio_ms: 2_000,
  inference_wall_time_ms: 500,
  real_time_factor: 0.25,
  process_cpu_time_ms: 800,
  average_cpu_utilization_percent: 40,
  baseline_resident_memory_bytes: 100 * 1024 * 1024,
  resident_memory_bytes: 140 * 1024 * 1024,
  peak_resident_memory_bytes: 160 * 1024 * 1024,
};

describe("AsrDiagnosticsPanel", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("shows retained local ASR performance and typed-error diagnostics", async () => {
    vi.spyOn(tauriBridge, "getAsrDiagnostics").mockResolvedValue(diagnostics);

    render(<AsrDiagnosticsPanel />);

    expect(
      await screen.findByText("Moonshine Tiny Streaming (English)"),
    ).toBeInTheDocument();
    expect(screen.getByText("0.250×")).toBeInTheDocument();
    expect(screen.getByText("40.0%")).toBeInTheDocument();
    expect(screen.getByText("60.0 MiB")).toBeInTheDocument();
    expect(
      screen.getByText(/inference: previous session stopped during inference/i),
    ).toBeInTheDocument();
  });
});
