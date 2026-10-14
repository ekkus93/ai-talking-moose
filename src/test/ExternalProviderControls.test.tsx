import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AiTab } from "../components/Settings/AiTab";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "../stores/mooseStore";

vi.mock("../components/Settings/LocalLlmSettingsPanel", () => ({
  LocalLlmSettingsPanel: () => React.createElement("div", null, "Local model"),
}));

const props = {
  googleModels: [],
  googleModelsStatus: "ready" as const,
  apiKeyInput: "",
  setApiKeyInput: vi.fn(),
  openaiKeyInput: "",
  setOpenaiKeyInput: vi.fn(),
  hasOpenaiKey: false,
  setHasOpenaiKey: vi.fn(),
  openaiKeyError: null,
  setOpenaiKeyError: vi.fn(),
  serverStatus: null,
  setServerStatus: vi.fn(),
  testResult: null,
  setTestResult: vi.fn(),
  isTesting: false,
  setIsTesting: vi.fn(),
};

describe("External text provider controls", () => {
  beforeEach(() => {
    resetSettingsPersistenceForTests();
    useMooseStore.setState({
      settings: {
        ...frontendDefaultSettings(),
        asr_mode: "whisper_small_batch",
        text_provider: "local",
      },
      hasApiKey: false,
    });
    vi.spyOn(tauriBridge, "updateSettings").mockResolvedValue(undefined);
  });

  afterEach(() => {
    resetSettingsPersistenceForTests();
    vi.restoreAllMocks();
  });

  it("shows five explicit text providers and persists the selected one", async () => {
    render(<AiTab {...props} />);
    expect(screen.getAllByRole("radio", { name: /cloud|loopback|local/i }).length)
      .toBeGreaterThanOrEqual(4);
    fireEvent.click(screen.getByRole("radio", { name: /Ollama — loopback/i }));
    await waitFor(() =>
      expect(useMooseStore.getState().settings?.text_provider).toBe("ollama"),
    );
    expect(tauriBridge.updateSettings).toHaveBeenCalled();
  });

  it("checks the selected loopback server without changing the provider", async () => {
    useMooseStore.setState((state) => ({
      settings: state.settings
        ? { ...state.settings, text_provider: "ollama" }
        : null,
    }));
    const check = vi
      .spyOn(tauriBridge, "testLocalTextServer")
      .mockResolvedValue("model_ready");
    const setStatus = vi.fn();
    render(<AiTab {...props} setServerStatus={setStatus} />);
    fireEvent.click(screen.getByRole("button", { name: "Check Server and Model" }));
    await waitFor(() => expect(check).toHaveBeenCalledWith("ollama"));
    expect(setStatus).toHaveBeenCalledWith(
      "Connected: selected model is available.",
    );
    expect(useMooseStore.getState().settings?.text_provider).toBe("ollama");
  });

  it("saves the independent OpenAI key without including it in settings", async () => {
    useMooseStore.setState((state) => ({
      settings: state.settings
        ? { ...state.settings, text_provider: "openai" }
        : null,
    }));
    const save = vi
      .spyOn(tauriBridge, "setOpenaiApiKey")
      .mockResolvedValue(undefined);
    const setPresence = vi.fn();
    render(
      <AiTab
        {...props}
        openaiKeyInput="test-credential-not-a-real-key"
        setHasOpenaiKey={setPresence}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Save Key" }));
    await waitFor(() =>
      expect(save).toHaveBeenCalledWith("test-credential-not-a-real-key"),
    );
    expect(setPresence).toHaveBeenCalledWith(true);
    expect(JSON.stringify(useMooseStore.getState().settings)).not.toContain(
      "test-credential-not-a-real-key",
    );
  });
});
