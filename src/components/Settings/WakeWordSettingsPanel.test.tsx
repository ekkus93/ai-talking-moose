import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { frontendDefaultSettings } from "../../lib/backendContract";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "../../stores/mooseStore";
import type { WakeWordDiagnostics } from "../../types/moose";
import { WakeWordSettingsPanel } from "./WakeWordSettingsPanel";

const wakeToggle = () => {
  return screen.getByRole("checkbox", { name: /enable wake word/i });
};

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

const mockWakeDiagnosticsResponse = (diagnostics: WakeWordDiagnostics) => {
  const defaultInvoke = vi.mocked(invoke).getMockImplementation();
  if (!defaultInvoke) {
    throw new Error("Tauri invoke test fixture is missing its default implementation");
  }
  vi.mocked(invoke).mockImplementation(
    async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "get_wake_word_diagnostics") return diagnostics;
      if (cmd === "update_settings") return undefined;
      return defaultInvoke(cmd, args);
    },
  );
};

const renderPanel = (wakeWordEnabled = false) => {
  resetSettingsPersistenceForTests();
  useMooseStore.setState({
    settings: {
      ...frontendDefaultSettings(),
      wake_word_enabled: wakeWordEnabled,
      wake_word_phrase: "Hey, Moose",
    },
  });
  render(<WakeWordSettingsPanel />);
};

const expectPersistedWakeSetting = async (enabled: boolean) => {
  await waitFor(() => {
    expect(invoke).toHaveBeenCalledWith(
      "update_settings",
      expect.objectContaining({
        newSettings: expect.objectContaining({
          wake_word_enabled: enabled,
          wake_word_phrase: "Hey, Moose",
        }),
      }),
    );
  });
};

const waitForRuntimeDiagnostics = async () => {
  expect(await screen.findByText(/runtime: disabled/i)).toBeInTheDocument();
};

describe("WakeWordSettingsPanel", () => {
  beforeEach(() => {
    resetSettingsPersistenceForTests();
    vi.clearAllMocks();
  });

  it("shows disabled-by-default controls and privacy disclosures", async () => {
    renderPanel(false);

    expect(wakeToggle()).not.toBeChecked();
    expect(screen.getByLabelText("Wake word phrase")).toHaveTextContent(
      "Hey, Moose",
    );
    expect(screen.queryByDisplayValue("Hey, Moose")).not.toBeInTheDocument();
    expect(screen.getByText(/local\/offline keyword/i)).toBeInTheDocument();
    expect(screen.getByText(/locally active/i)).toBeInTheDocument();
    expect(screen.getByText(/not full-time cloud/i)).toBeInTheDocument();
    expect(
      screen.getByText(/local Moonshine command ASR/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/developer-prepared/i)).toBeInTheDocument();
    expect(screen.getByText(/clean install fails closed/i)).toBeInTheDocument();
    expect(screen.getByText(/no barge-in support/i)).toBeInTheDocument();
    expect(
      screen.getByText("Disabled — manual start remains available."),
    ).toBeInTheDocument();
    await waitForRuntimeDiagnostics();
    expect(screen.getByText(/listener: stopped/i)).toBeInTheDocument();
    expect(screen.getByText(/listener ownership is stopped/i)).toBeInTheDocument();
    expect(screen.getAllByText("No").length).toBeGreaterThanOrEqual(1);
  });

  it("distinguishes active listener ownership from runtime phase", async () => {
    mockWakeDiagnosticsResponse(
      wakeDiagnostics({
        enabled: true,
        runtime_phase: "listening",
        listener_status: "active",
        listener_active: true,
        listening: true,
      }),
    );

    renderPanel(true);

    expect(
      await screen.findByText(/runtime: listening locally/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/listener: active locally/i)).toBeInTheDocument();
    expect(
      screen.getByText(/microphone is active locally/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /preference is enabled and listener ownership is active/i,
      ),
    ).toBeInTheDocument();
  });

  it("shows pending listener ownership without claiming active listening", async () => {
    mockWakeDiagnosticsResponse(
      wakeDiagnostics({
        enabled: true,
        runtime_phase: "loading",
        listener_status: "pending_until_idle",
        listener_active: false,
        listening: false,
      }),
    );

    renderPanel(true);

    expect(
      await screen.findByText(/listener: pending until idle/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/pending until the current conversation/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /preference is enabled, but listener ownership is not active yet/i,
      ),
    ).toBeInTheDocument();
  });

  it("shows failed-closed listener ownership without leaking artifact paths", async () => {
    mockWakeDiagnosticsResponse(
      wakeDiagnostics({
        enabled: true,
        runtime_phase: "error",
        listener_status: "failed_closed",
        listener_active: false,
        listening: false,
        last_error: "The Wake Word runtime encountered an internal error.",
      }),
    );

    renderPanel(true);

    expect(
      await screen.findByText(/listener: failed closed/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/check local Moonshine ASR mode/i),
    ).toBeInTheDocument();
    expect(screen.queryByText(/\/tmp|users\/|appdata/i)).not.toBeInTheDocument();
  });

  it("persists the fixed phrase when enabling Wake Word", async () => {
    renderPanel(false);
    await waitForRuntimeDiagnostics();

    fireEvent.click(wakeToggle());

    await expectPersistedWakeSetting(true);
    expect(wakeToggle()).toBeChecked();
    expect(
      screen.getByText(
        "Enabled — runtime starts only after developer-prepared local KWS artifacts, local Moonshine ASR policy, and lifecycle state permit.",
      ),
    ).toBeInTheDocument();
  });

  it("persists the fixed phrase when disabling Wake Word", async () => {
    renderPanel(true);
    await waitForRuntimeDiagnostics();

    fireEvent.click(wakeToggle());

    await expectPersistedWakeSetting(false);
    expect(wakeToggle()).not.toBeChecked();
    expect(
      screen.getByText("Disabled — manual start remains available."),
    ).toBeInTheDocument();
  });
});
