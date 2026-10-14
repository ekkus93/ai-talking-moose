import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { useMooseStore } from "../stores/mooseStore";

it.each(["openai", "ollama", "llama_server"] as const)(
  "%s text selection leaves standalone speech on Local",
  (provider) => {
    useMooseStore.setState({
      isSettingsOpen: true,
      settings: {
        ...frontendDefaultSettings(),
        text_provider: provider,
        tts_provider: "local",
        asr_mode: "whisper_small",
      },
    });
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("Voice & Audio"));
    expect(
      screen.getByRole("radio", { name: /Local KittenTTS/i }),
    ).toBeChecked();
    expect(
      screen.getByRole("radio", { name: /Google Gemini TTS/i }),
    ).not.toBeChecked();
    expect(useMooseStore.getState().settings?.text_provider).toBe(provider);
  },
);
