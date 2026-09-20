import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { frontendDefaultSettings } from "../../lib/backendContract";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "../../stores/mooseStore";
import { WakeWordSettingsPanel } from "./WakeWordSettingsPanel";

const wakeToggle = () => {
  return screen.getByRole("checkbox", { name: /enable wake word/i });
};

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

const expectPersistedWakeSetting = async (enabled: boolean) => {
  await waitFor(() => {
    expect(invoke).toHaveBeenCalledWith(
      "update_settings",
      expect.objectContaining({
        newSettings: expect.objectContaining({
          wake_word_enabled: enabled,
          wake_word_phrase: "Hey, Moose",
        }),
      }),
    );
  });
};

const waitForRuntimeDiagnostics = async () => {
  expect(await screen.findByText(/runtime: disabled/i)).toBeInTheDocument();
};

describe("WakeWordSettingsPanel", () => {
  beforeEach(() => {
    resetSettingsPersistenceForTests();
    vi.clearAllMocks();
  });

  it("shows disabled-by-default controls and privacy disclosures", async () => {
    renderPanel(false);

    expect(wakeToggle()).not.toBeChecked();
    expect(screen.getByLabelText("Wake word phrase")).toHaveTextContent(
      "Hey, Moose",
    );
    expect(screen.queryByDisplayValue("Hey, Moose")).not.toBeInTheDocument();
    expect(screen.getByText(/local\/offline keyword/i)).toBeInTheDocument();
    expect(screen.getByText(/locally active/i)).toBeInTheDocument();
    expect(screen.getByText(/not full-time cloud/i)).toBeInTheDocument();
    expect(screen.getByText(/no barge-in support/i)).toBeInTheDocument();
    expect(
      screen.getByText("Disabled — manual start remains available."),
    ).toBeInTheDocument();
    await waitForRuntimeDiagnostics();
  });

  it("persists the fixed phrase when enabling Wake Word", async () => {
    renderPanel(false);
    await waitForRuntimeDiagnostics();

    fireEvent.click(wakeToggle());

    await expectPersistedWakeSetting(true);
    expect(wakeToggle()).toBeChecked();
    expect(
      screen.getByText(
        "Enabled — runtime starts when local KWS artifacts and lifecycle state permit.",
      ),
    ).toBeInTheDocument();
  });

  it("persists the fixed phrase when disabling Wake Word", async () => {
    renderPanel(true);
    await waitForRuntimeDiagnostics();

    fireEvent.click(wakeToggle());

    await expectPersistedWakeSetting(false);
    expect(wakeToggle()).not.toBeChecked();
    expect(
      screen.getByText("Disabled — manual start remains available."),
    ).toBeInTheDocument();
  });
});
