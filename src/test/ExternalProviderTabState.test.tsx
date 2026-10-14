import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { useMooseStore } from "../stores/mooseStore";

it("retains the external provider across settings tab switches", () => {
  useMooseStore.setState({
    isSettingsOpen: true,
    settings: { ...frontendDefaultSettings(), text_provider: "ollama" },
  });
  render(<SettingsModal />);
  fireEvent.click(screen.getByText("AI & Models"));
  fireEvent.click(screen.getByText("Privacy"));
  fireEvent.click(screen.getByText("AI & Models"));
  expect(screen.getByLabelText("Ollama model ID")).toBeInTheDocument();
});
