import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LocalLlmSettingsPanel } from "../components/Settings/LocalLlmSettingsPanel";
import { tauriBridge } from "../lib/tauriBridge";
import type { LocalLlmDiagnostics, LocalModelDescriptor } from "../types/moose";

const MODEL_ID = "smollm2-360m-instruct-q4-k-m";

const model: LocalModelDescriptor = {
  id: MODEL_ID,
  display_name: "SmolLM2 360M",
  family: "SmolLM2",
  parameter_scale: "360M",
  quantization: "Q4_K_M",
  revision: "0123456789012345678901234567890123456789",
  expected_bytes: 100 * 1024 * 1024,
  installed_bytes: 100 * 1024 * 1024,
  license: "Apache-2.0",
  context_limit: 8192,
  recommended_max_output: 192,
  install_state: "installed",
  active: true,
  error: null,
};

const diagnostics: LocalLlmDiagnostics = {
  installer: {
    model_root_ready: true,
    installs_in_progress: 0,
    last_error: null,
  },
  selected_install_state: "installed",
  runtime: {
    selected_model_id: MODEL_ID,
    loaded_model_id: MODEL_ID,
    loaded_revision: "revision",
    loaded_quantization: "Q4_K_M",
    loaded: true,
    phase: "ready",
    thread_count: 2,
    context_size: 4096,
    generation_in_progress: true,
    last_error_category: "decode",
    last_generation_duration_ms: 25,
    last_prompt_tokens: 7,
    last_output_tokens: 3,
    last_tokens_per_second: 120,
  },
};

describe("Local LLM diagnostics settings surface", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders live runtime identity, state, safe errors, and performance metrics", async () => {
    vi.spyOn(tauriBridge, "getLocalLlmModels").mockResolvedValue([model]);
    vi.spyOn(tauriBridge, "getLocalLlmDiagnostics").mockResolvedValue(
      diagnostics,
    );
    vi.spyOn(tauriBridge, "onLocalLlmModelProgress").mockResolvedValue(
      () => undefined,
    );

    render(
      <LocalLlmSettingsPanel
        selectedModelId={MODEL_ID}
        onSelectModel={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    expect(
      await screen.findByLabelText("Local LLM diagnostics"),
    ).toBeInTheDocument();
    expect(screen.getByText("Installer phase: Installed")).toBeInTheDocument();
    expect(screen.getByText("Runtime state: Generating")).toBeInTheDocument();
    expect(screen.getByText(`Selected model: ${MODEL_ID}`)).toBeInTheDocument();
    expect(
      screen.getByText(`Loaded model: ${MODEL_ID} @ revision (Q4_K_M)`),
    ).toBeInTheDocument();
    expect(screen.getByText("Threads: 2")).toBeInTheDocument();
    expect(screen.getByText("Context: 4096 tokens")).toBeInTheDocument();
    expect(screen.getByText("Generation: In progress")).toBeInTheDocument();
    expect(screen.getByText("Installer error: None")).toBeInTheDocument();
    expect(screen.getByText("Runtime error: decode")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Last generation: 25 ms • 7 prompt • 3 output • 120.0 tok/s",
      ),
    ).toBeInTheDocument();
  });
});
