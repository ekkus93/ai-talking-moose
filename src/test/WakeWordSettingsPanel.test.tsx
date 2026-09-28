import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WakeWordSettingsPanel } from "../components/Settings/WakeWordSettingsPanel";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";
import type { WakeWordDiagnostics } from "../types/moose";

const renderPanel = () => render(<WakeWordSettingsPanel />);

const wakeDiagnostics = (
  patch: Partial<WakeWordDiagnostics> = {},
): WakeWordDiagnostics => ({
  enabled: false,
  runtime_phase: "disabled",
  listener_status: "stopped",
  listener_active: false,
  listening: false,
  engine_id: "sherpa-onnx-kws",
  model_id: "sherpa-onnx-kws-gigaspeech-v1",
  model_archive_sha256: "test-model-sha256",
  model_license: "Apache-2.0",
  keyword_sha256: "test-keyword-sha256",
  runtime_id: "sherpa-onnx-v1.13.8",
  runtime_license: "Apache-2.0",
  runtime_c_api_sha256: null,
  platform: "test",
  architecture: "test",
  canonical_sample_rate_hz: 16_000,
  canonical_channels: 1,
  inference_threads: 1,
  threshold: 0.25,
  score: 1.0,
  ring_buffer_capacity_samples: 32_000,
  ring_buffer_capacity_ms: 2_000,
  ring_buffer_samples: 0,
  handoff_pre_roll_samples: 0,
  handoff_pre_roll_duration_ms: 0,
  trigger_count: 0,
  last_trigger_age_ms: null,
  runtime_initialization_ms: null,
  measured_idle_cpu_percent: null,
  measured_memory_rss_bytes: null,
  last_inference_duration_ms: null,
  last_handoff_duration_ms: null,
  talking_suspended: false,
  last_error: null,
  ...patch,
});

describe("WakeWordSettingsPanel", () => {
  beforeEach(() => {
    useMooseStore.setState({ settings: frontendDefaultSettings() });
    vi.spyOn(tauriBridge, "updateSettings").mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders disabled-by-default wake word controls and local microphone disclosures", () => {
    renderPanel();

    expect(screen.getByText("Wake Word")).toBeInTheDocument();
    expect(
      screen.getByRole("checkbox", { name: /Enable wake word/i }),
    ).not.toBeChecked();
    expect(screen.getByLabelText("Wake word phrase")).toHaveTextContent(
      "Hey, Moose",
    );
    expect(
      screen.queryByRole("textbox", { name: /wake word phrase/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText(/local\/offline keyword spotting/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/microphone remains locally active/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/not full-time cloud transcription/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/no barge-in support/i)).toBeInTheDocument();
  });

  it("persists the enable toggle with the fixed canonical phrase", async () => {
    const updateSpy = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockResolvedValue(undefined);
    renderPanel();

    fireEvent.click(
      screen.getByRole("checkbox", { name: /Enable wake word/i }),
    );

    await waitFor(() =>
      expect(useMooseStore.getState().settings?.wake_word_enabled).toBe(true),
    );
    expect(useMooseStore.getState().settings?.wake_word_phrase).toBe(
      "Hey, Moose",
    );
    await waitFor(() => expect(updateSpy).toHaveBeenCalled());
    expect(updateSpy.mock.calls.at(-1)?.[0]).toMatchObject({
      wake_word_enabled: true,
      wake_word_phrase: "Hey, Moose",
    });
  });

  it("refreshes diagnostics after the enable toggle completes", async () => {
    const diagnosticsSpy = vi
      .spyOn(tauriBridge, "getWakeWordDiagnostics")
      .mockResolvedValueOnce(wakeDiagnostics())
      .mockResolvedValueOnce(
        wakeDiagnostics({
          enabled: true,
          runtime_phase: "listening",
          listener_status: "active",
          listener_active: true,
          listening: true,
        }),
      );
    const updateSpy = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockResolvedValue(undefined);
    renderPanel();

    expect(await screen.findByText(/listener: stopped/i)).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("checkbox", { name: /Enable wake word/i }),
    );

    await waitFor(() => expect(updateSpy).toHaveBeenCalled());
    await waitFor(() => expect(diagnosticsSpy).toHaveBeenCalledTimes(2));
    expect(
      screen.getByText(/runtime: listening locally/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/listener: active locally/i)).toBeInTheDocument();
    expect(
      screen.getByText(
        /preference is enabled and listener ownership is active/i,
      ),
    ).toBeInTheDocument();
  });

  it("shows an enabled runtime status without implying cloud transcription", () => {
    useMooseStore.setState({
      settings: {
        ...frontendDefaultSettings(),
        wake_word_enabled: true,
        wake_word_phrase: "Hey, Moose",
      },
    });

    renderPanel();

    expect(
      screen.getByRole("checkbox", { name: /Enable wake word/i }),
    ).toBeChecked();
    expect(screen.getByText(/Enabled — runtime starts/i)).toBeInTheDocument();
    expect(
      screen.getByText(/not full-time cloud transcription/i),
    ).toBeInTheDocument();
  });
});
