import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { useMooseStore } from "../stores/mooseStore";

it("shows System voice as unavailable", () => {
  useMooseStore.setState({
    isSettingsOpen: true,
    settings: frontendDefaultSettings(),
  });
  render(<SettingsModal />);
  fireEvent.click(screen.getByText("Voice & Audio"));
  expect(
    screen.getByRole("radio", { name: /System Voices — unavailable/i }),
  ).toBeDisabled();
});

it("preserves a saved system provider", () => {
  useMooseStore.setState({
    isSettingsOpen: true,
    settings: {
      ...frontendDefaultSettings(),
      tts_provider: "system",
      system_tts_voice: "com.apple.voice.unavailable",
    },
  });
  render(<SettingsModal />);
  fireEvent.click(screen.getByText("Voice & Audio"));
  expect(
    screen.getByRole("radio", { name: /System Voices — unavailable/i }),
  ).toBeChecked();
  expect(
    screen.getByRole("radio", { name: /System Voices — unavailable/i }),
  ).toBeDisabled();
  expect(
    screen.getByText(/Saved macOS system voice: com.apple.voice.unavailable/),
  ).toBeInTheDocument();
});
