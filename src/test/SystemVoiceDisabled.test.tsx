import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { useMooseStore } from "../stores/mooseStore";

it("keeps unfinished system voice synthesis disabled", () => {
  useMooseStore.setState({ isSettingsOpen: true, settings: frontendDefaultSettings() });
  render(<SettingsModal />);
  fireEvent.click(screen.getByText("Voice & Audio"));
  expect(screen.getByRole("radio", { name: /System Voices — unavailable/i })).toBeDisabled();
});
