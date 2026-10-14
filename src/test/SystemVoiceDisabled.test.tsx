import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { useMooseStore } from "../stores/mooseStore";

it("keeps unfinished system voice synthesis disabled", () => {
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

it("preserves a previously selected System voice visibly without pretending it works", () => {
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
  const systemRadio = screen.getByRole("radio", {
    name: /System Voices — unavailable/i,
  });
  expect(systemRadio).toBeChecked();
  expect(systemRadio).toBeDisabled();
  expect(
    screen.getByText(/System voice synthesis is not yet available in this build/i),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: /Audition System Voices/i }),
  ).toBeDisabled();
  expect(useMooseStore.getState().settings?.tts_provider).toBe("system");
  expect(useMooseStore.getState().settings?.system_tts_voice).toBe(
    "com.apple.voice.unavailable",
  );
});
