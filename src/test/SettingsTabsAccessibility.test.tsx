import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import {
  frontendDefaultSettings,
  frontendTtsCatalog,
} from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";
import type { TextProvider } from "../types/moose";

function open(tab: string, provider: TextProvider) {
  vi.spyOn(tauriBridge, "hasOpenaiApiKey").mockResolvedValue(false);
  vi.spyOn(tauriBridge, "getTtsCatalog").mockResolvedValue(
    frontendTtsCatalog(),
  );
  useMooseStore.setState({
    isSettingsOpen: true,
    settings: {
      ...frontendDefaultSettings(),
      text_provider: provider,
      asr_mode: "whisper_small",
    },
    settingsPersistenceError: null,
    hasApiKey: false,
  });
  render(<SettingsModal />);
  fireEvent.click(screen.getByText(tab));
}

// Every interactive control must expose a non-empty accessible name.
function expectNamedControls(root: HTMLElement) {
  const controls = Array.from(
    root.querySelectorAll<HTMLElement>("button, input, select, textarea"),
  );
  expect(controls.length).toBeGreaterThan(0);
  for (const control of controls) {
    const name =
      control.getAttribute("aria-label") ??
      control.getAttribute("aria-labelledby") ??
      (control as HTMLInputElement).labels?.[0]?.textContent ??
      control.textContent;
    expect(name?.trim(), control.outerHTML).toBeTruthy();
  }
}

describe("Settings tab accessibility", () => {
  afterEach(() => vi.restoreAllMocks());

  it.each(["local", "google", "openai", "ollama", "llama_server"] as const)(
    "AI & Models (%s) exposes named controls and a labelled provider group",
    (provider) => {
      open("AI & Models", provider);
      const group = screen.getByRole("group", {
        name: "Text Generation Provider",
      });
      expect(within(group).getAllByRole("radio")).toHaveLength(5);
      expectNamedControls(screen.getByRole("dialog"));
    },
  );

  it("Voice & Audio exposes named controls and a labelled provider group", async () => {
    open("Voice & Audio", "local");
    const group = await screen.findByRole("group", {
      name: "Standalone Speech Provider",
    });
    const radios = within(group).getAllByRole("radio");
    expect(radios).toHaveLength(3);
    const system = within(group).getByRole("radio", { name: /System Voices/ });
    expect(system).toBeDisabled();
    expectNamedControls(screen.getByRole("dialog"));
  });
});
