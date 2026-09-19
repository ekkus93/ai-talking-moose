import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { WakeWordSettingsPanel } from "./WakeWordSettingsPanel";

const { mockUpdateSettingsPatch, storeState } = vi.hoisted(() => ({
  mockUpdateSettingsPatch: vi.fn(),
  storeState: {
    settings: {
      wake_word_enabled: false,
      wake_word_phrase: "Hey, Moose",
    },
  },
}));

vi.mock("../../stores/mooseStore", () => ({
  useMooseStore: () => ({
    settings: storeState.settings,
    updateSettingsPatch: mockUpdateSettingsPatch,
  }),
}));

describe("WakeWordSettingsPanel", () => {
  beforeEach(() => {
    mockUpdateSettingsPatch.mockReset();
    storeState.settings.wake_word_enabled = false;
    storeState.settings.wake_word_phrase = "Hey, Moose";
  });

  it("renders disabled by default with fixed phrase and privacy disclosures", () => {
    render(<WakeWordSettingsPanel />);

    const toggle = screen.getByRole("checkbox", { name: "Enable wake word" });
    expect(toggle).not.toBeChecked();
    expect(screen.getByLabelText("Wake word phrase")).toHaveTextContent("Hey, Moose");
    expect(
      screen.getByText("Wake Word V1 uses local/offline keyword spotting."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("The microphone remains locally active while listening."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Wake detection is not full-time cloud transcription."),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Wake Word V1 has no barge-in support while Moose talks.",
      ),
    ).toBeInTheDocument();
  });

  it("enables through the normal settings patch with the immutable V1 phrase", () => {
    render(<WakeWordSettingsPanel />);

    fireEvent.click(screen.getByRole("checkbox", { name: "Enable wake word" }));

    expect(mockUpdateSettingsPatch).toHaveBeenCalledTimes(1);
    expect(mockUpdateSettingsPatch).toHaveBeenCalledWith({
      wake_word_enabled: true,
      wake_word_phrase: "Hey, Moose",
    });
  });

  it("disables immediately through the same settings path", () => {
    storeState.settings.wake_word_enabled = true;
    render(<WakeWordSettingsPanel />);

    const toggle = screen.getByRole("checkbox", { name: "Enable wake word" });
    expect(toggle).toBeChecked();
    fireEvent.click(toggle);

    expect(mockUpdateSettingsPatch).toHaveBeenCalledTimes(1);
    expect(mockUpdateSettingsPatch).toHaveBeenCalledWith({
      wake_word_enabled: false,
      wake_word_phrase: "Hey, Moose",
    });
  });

  it("displays the phrase as text rather than an editable control", () => {
    render(<WakeWordSettingsPanel />);

    expect(screen.getByLabelText("Wake word phrase")).toHaveTextContent("Hey, Moose");
    expect(screen.queryByDisplayValue("Hey, Moose")).not.toBeInTheDocument();
    expect(screen.queryByRole("slider")).not.toBeInTheDocument();
  });
});
