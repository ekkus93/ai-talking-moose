import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WakeWordSettingsPanel } from "../components/Settings/WakeWordSettingsPanel";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";

const renderPanel = () => render(<WakeWordSettingsPanel />);

describe("WakeWordSettingsPanel", () => {
  beforeEach(() => {
    useMooseStore.setState({ settings: frontendDefaultSettings() });
    vi.spyOn(tauriBridge, "updateSettings").mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders disabled-by-default wake word controls and local microphone disclosures", () => {
    renderPanel();

    expect(screen.getByText("Wake Word")).toBeInTheDocument();
    expect(
      screen.getByRole("checkbox", { name: /Enable wake word/i }),
    ).not.toBeChecked();
    expect(screen.getByLabelText("Wake word phrase")).toHaveTextContent(
      "Hey, Moose",
    );
    expect(
      screen.queryByRole("textbox", { name: /wake word phrase/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText(/local\/offline keyword spotting/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/microphone remains locally active/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/not full-time cloud transcription/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/no barge-in support/i)).toBeInTheDocument();
  });

  it("persists the enable toggle with the fixed canonical phrase", async () => {
    const updateSpy = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockResolvedValue(undefined);
    renderPanel();

    fireEvent.click(
      screen.getByRole("checkbox", { name: /Enable wake word/i }),
    );

    await waitFor(() =>
      expect(useMooseStore.getState().settings?.wake_word_enabled).toBe(true),
    );
    expect(useMooseStore.getState().settings?.wake_word_phrase).toBe(
      "Hey, Moose",
    );
    await waitFor(() => expect(updateSpy).toHaveBeenCalled());
    expect(updateSpy.mock.calls.at(-1)?.[0]).toMatchObject({
      wake_word_enabled: true,
      wake_word_phrase: "Hey, Moose",
    });
  });

  it("shows an enabled runtime status without implying cloud transcription", () => {
    useMooseStore.setState({
      settings: {
        ...frontendDefaultSettings(),
        wake_word_enabled: true,
        wake_word_phrase: "Hey, Moose",
      },
    });

    renderPanel();

    expect(
      screen.getByRole("checkbox", { name: /Enable wake word/i }),
    ).toBeChecked();
    expect(screen.getByText(/Enabled — runtime starts/i)).toBeInTheDocument();
    expect(
      screen.getByText(/not full-time cloud transcription/i),
    ).toBeInTheDocument();
  });
});
