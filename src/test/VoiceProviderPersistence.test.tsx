import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { useMooseStore } from "../stores/mooseStore";

it("shows persisted unavailable speech choice", () => {
  useMooseStore.setState({
    isSettingsOpen: true,
    settings: { ...frontendDefaultSettings(), tts_provider: "system" },
  });
  render(<SettingsModal />);
  fireEvent.click(screen.getByText("Voice & Audio"));
  expect(
    screen.getByRole("radio", { name: /System Voices — unavailable/i }),
  ).toBeChecked();
  expect(useMooseStore.getState().settings?.tts_provider).toBe("system");
});
