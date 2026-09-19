import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { frontendDefaultSettings } from "../../lib/backendContract";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "../../stores/mooseStore";
import { WakeWordSettingsPanel } from "./WakeWordSettingsPanel";

const renderPanel = (wakeWordEnabled = false) => {
  resetSettingsPersistenceForTests();
  useMooseStore.setState({
    settings: {
      ...frontendDefaultSettings(),
      wake_word_enabled: wakeWordEnabled,
      wake_word_phrase: "Hey, Moose",
    },
  });
  render(<WakeWordSettingsPanel />);
};

describe("WakeWordSettingsPanel", () => {
  beforeEach(() => {
    resetSettingsPersistenceForTests();
    vi.clearAllMocks();
  });

  it(
    "shows disabled-by-default Wake Word controls and required privacy disclosures",
    () => {
      renderPanel(false);

      const toggle = screen.getByRole("checkbox", {
        name: /enable wake word/i,
      });
      expect(toggle).not.toBeChecked();
      expect(screen.getByLabelText("Wake word phrase")).toHaveTextContent(
        "Hey, Moose",
      );
      expect(screen.queryByDisplayValue("Hey, Moose")).not.toBeInTheDocument();
      expect(
        screen.getByText(/local\/offline keyword spotting/i),
      ).toBeInTheDocument();
      expect(
        screen.getByText(/microphone remains locally active while listening/i),
      ).toBeInTheDocument();
      expect(
        screen.getByText(/not full-time cloud transcription/i),
      ).toBeInTheDocument();
      expect(screen.getByText(/no barge-in support/i)).toBeInTheDocument();
      expect(
        screen.getByText(/manual start remains available/i),
      ).toBeInTheDocument();
    },
  );

  it("persists the fixed phrase when the user enables Wake Word", async () => {
    renderPanel(false);

    fireEvent.click(screen.getByRole("checkbox", { name: /enable wake word/i }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "update_settings",
        expect.objectContaining({
          newSettings: expect.objectContaining({
            wake_word_enabled: true,
            wake_word_phrase: "Hey, Moose",
          }),
        }),
      ),
    );
    expect(
      screen.getByRole("checkbox", { name: /enable wake word/i }),
    ).toBeChecked();
    expect(screen.getByText(/enabled — runtime starts/i)).toBeInTheDocument();
  });

  it("persists the fixed phrase when the user disables Wake Word", async () => {
    renderPanel(true);

    fireEvent.click(screen.getByRole("checkbox", { name: /enable wake word/i }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "update_settings",
        expect.objectContaining({
          newSettings: expect.objectContaining({
            wake_word_enabled: false,
            wake_word_phrase: "Hey, Moose",
          }),
        }),
      ),
    );
    expect(
      screen.getByRole("checkbox", { name: /enable wake word/i }),
    ).not.toBeChecked();
    expect(
      screen.getByText(/manual start remains available/i),
    ).toBeInTheDocument();
  });
});
