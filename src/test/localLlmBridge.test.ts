import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { tauriBridge } from "../lib/tauriBridge";

beforeEach(() => {
  vi.mocked(invoke).mockClear();
});

describe("local LLM lifecycle bridge", () => {
  it("uses registered native commands for catalog and diagnostics", async () => {
    const models = await tauriBridge.getLocalLlmModels();
    const diagnostics = await tauriBridge.getLocalLlmDiagnostics();

    expect(models.map((model) => model.id)).toEqual([
      "smollm2-360m-instruct-q4-k-m",
      "qwen3-0-6b-instruct-q4-k-m",
    ]);
    expect(diagnostics.model_root_ready).toBe(true);
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("get_local_llm_models");
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("get_local_llm_diagnostics");
  });

  it("passes stable app model IDs to install cancel and delete commands", async () => {
    const modelId = "smollm2-360m-instruct-q4-k-m";

    const installed = await tauriBridge.installLocalLlmModel(modelId);
    const cancelled = await tauriBridge.cancelLocalLlmInstall(modelId);
    const deleted = await tauriBridge.deleteLocalLlmModel(modelId);

    expect(installed.install_state).toBe("installed");
    expect(cancelled).toBe(true);
    expect(deleted.id).toBe(modelId);
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("install_local_llm_model", {
      modelId,
    });
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("cancel_local_llm_install", {
      modelId,
    });
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("delete_local_llm_model", {
      modelId,
    });
  });

  it("uses the dedicated bounded local model self-test command", async () => {
    const result = await tauriBridge.testLocalLlmModel();

    expect(result).toEqual({
      success: true,
      message: "Local model test succeeded",
    });
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("test_local_llm_model");
  });
});
