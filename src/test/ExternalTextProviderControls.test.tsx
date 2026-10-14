import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { useMooseStore } from "../stores/mooseStore";

describe("external provider control panel", () => {
  it("shows provider-specific server and model fields", () => {
    useMooseStore.setState({
      isSettingsOpen: true,
      settings: { ...frontendDefaultSettings(), text_provider: "ollama" },
      settingsPersistenceError: null,
      hasApiKey: false,
    });
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("AI & Models"));
    expect(screen.getByLabelText("Ollama model ID")).toBeInTheDocument();
    expect(screen.getByLabelText(/HTTP loopback server/i)).toBeInTheDocument();
    expect(
      screen.queryByLabelText("Realtime Live Voice Model"),
    ).not.toBeInTheDocument();
  });
});
