import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { useMooseStore } from "../stores/mooseStore";

type ExternalProvider = "openai" | "ollama" | "llama_server";

function renderProvider(provider: ExternalProvider, live = false) {
  useMooseStore.setState({
    isSettingsOpen: true,
    settings: {
      ...frontendDefaultSettings(),
      text_provider: provider,
      asr_mode: live ? "gemini_live_audio" : "whisper_small",
    },
    settingsPersistenceError: null,
    hasApiKey: false,
  });
  render(<SettingsModal />);
  fireEvent.click(screen.getByText("AI & Models"));
}

describe("external text provider disclosure and Live isolation", () => {
  afterEach(() => vi.restoreAllMocks());

  it.each([
    ["openai", "OpenAI model ID", "OpenAI API key (BYOK)"],
    ["ollama", "Ollama model ID", "HTTP loopback server (explicit port required)"],
    ["llama_server", "llama-server model ID", "HTTP loopback server (explicit port required)"],
  ] as const)(
    "%s with local ASR exposes only its own provider controls",
    (provider, modelLabel, configLabel) => {
      renderProvider(provider);
      const displayName = provider === "llama_server" ? "llama-server" : provider;
      const providerRadio = screen.getByRole("radio", {
        name: new RegExp(displayName, "i"),
      });
      expect(providerRadio).toBeChecked();
      expect(screen.getByLabelText(modelLabel)).toBeInTheDocument();
      expect(screen.getByLabelText(configLabel)).toBeInTheDocument();
      expect(
        screen.queryByLabelText("Realtime Live Voice Model"),
      ).not.toBeInTheDocument();
      expect(
        screen.getByText(/Non-Live Talk uses the selected text provider/i),
      ).toBeInTheDocument();
      if (provider !== "openai") {
        expect(
          screen.queryByLabelText("OpenAI API key (BYOK)"),
        ).not.toBeInTheDocument();
      } else {
        expect(
          screen.queryByRole("button", { name: "Check Server and Model" }),
        ).not.toBeInTheDocument();
      }
    },
  );

  it("explicit Gemini Live ASR overrides standalone text controls only for Talk", () => {
    renderProvider("ollama", true);
    expect(screen.getByLabelText("Ollama model ID")).toBeInTheDocument();
    expect(screen.getByLabelText("Realtime Live Voice Model")).toBeInTheDocument();
    expect(
      screen.getByText(/Gemini Live is a separate Google cloud-audio mode/i),
    ).toBeInTheDocument();
  });
});
