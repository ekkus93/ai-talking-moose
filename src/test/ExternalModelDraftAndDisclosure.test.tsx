import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";
import type { TextProvider } from "../types/moose";

function renderProvider(provider: TextProvider) {
  const patch = vi.fn().mockResolvedValue(undefined);
  useMooseStore.setState({
    isSettingsOpen: true,
    settings: {
      ...frontendDefaultSettings(),
      text_provider: provider,
      asr_mode: "whisper_small",
    },
    settingsPersistenceError: null,
    hasApiKey: false,
    updateSettingsPatch: patch,
  });
  vi.spyOn(tauriBridge, "hasOpenaiApiKey").mockResolvedValue(false);
  render(<SettingsModal />);
  fireEvent.click(screen.getByText("AI & Models"));
  return patch;
}

describe("external model ID draft handling", () => {
  afterEach(() => vi.restoreAllMocks());

  it("commits a model ID typed without blur when the tab is switched", () => {
    const patch = renderProvider("ollama");
    fireEvent.change(screen.getByLabelText("Ollama model ID"), {
      target: { value: "  qwen3.5:9b  " },
    });
    expect(patch).not.toHaveBeenCalled();
    fireEvent.click(screen.getByText("Privacy"));
    expect(patch).toHaveBeenCalledTimes(1);
    expect(patch).toHaveBeenCalledWith({ ollama_text_model: "qwen3.5:9b" });
  });

  it("commits once on blur and not again when the tab unmounts", () => {
    const patch = renderProvider("llama_server");
    const input = screen.getByLabelText("llama-server model ID");
    fireEvent.change(input, { target: { value: "my-model" } });
    fireEvent.blur(input);
    expect(patch).toHaveBeenCalledWith({ llama_server_text_model: "my-model" });
    fireEvent.click(screen.getByText("Privacy"));
    // The store mock does not update settings, so the unmount commit is the
    // only extra opportunity; the draft still differs from the stored value.
    expect(
      patch.mock.calls.every(([arg]) => "llama_server_text_model" in arg),
    ).toBe(true);
  });

  it("reverts an emptied model ID and never saves a blank value", () => {
    const patch = renderProvider("openai");
    const original = (
      screen.getByLabelText("OpenAI model ID") as HTMLInputElement
    ).value;
    const input = screen.getByLabelText("OpenAI model ID");
    fireEvent.change(input, { target: { value: "   " } });
    fireEvent.blur(input);
    expect((input as HTMLInputElement).value).toBe(original);
    fireEvent.click(screen.getByText("Privacy"));
    expect(patch).not.toHaveBeenCalled();
  });
});

describe("text provider data-destination disclosure", () => {
  afterEach(() => vi.restoreAllMocks());

  it.each([
    ["openai", /OpenAI — cloud/, /Sends prompts to OpenAI with your own key/],
    ["ollama", /Ollama — loopback/, /independently managed loopback Ollama/],
    [
      "llama_server",
      /llama-server — loopback/,
      /independently managed loopback llama\.cpp server/,
    ],
  ] as const)("labels %s with its destination", (provider, label, detail) => {
    renderProvider(provider);
    expect(screen.getByText(label)).toBeInTheDocument();
    expect(screen.getByText(detail)).toBeInTheDocument();
    expect(screen.getByText(/Local \/ cloud privacy boundary/)).toBeVisible();
    expect(
      screen.getByText(
        /OpenAI and Google text or TTS send content to their respective cloud services/,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/use only your configured loopback server/),
    ).toBeInTheDocument();
  });

  it("shows OpenAI key state: not configured, then saved with unverified model", async () => {
    renderProvider("openai");
    expect(
      await screen.findByText("No OpenAI key is configured."),
    ).toBeInTheDocument();
    expect(screen.queryByText(/OpenAI key saved/)).toBeNull();
  });

  it("states that a saved OpenAI key does not verify the model", async () => {
    const patch = renderProvider("openai");
    expect(patch).not.toHaveBeenCalled();
    vi.spyOn(tauriBridge, "setOpenaiApiKey").mockResolvedValue(undefined);
    fireEvent.change(screen.getByLabelText("OpenAI API key (BYOK)"), {
      target: { value: "k" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save OpenAI Key" }));
    const status = await screen.findByText(/OpenAI key saved/);
    expect(status).toHaveAttribute("role", "status");
    expect(status).toHaveTextContent("not verified until a request is made");
    await waitFor(() =>
      expect(screen.queryByText("No OpenAI key is configured.")).toBeNull(),
    );
  });
});
