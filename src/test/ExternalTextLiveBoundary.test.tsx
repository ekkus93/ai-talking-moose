import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { useMooseStore } from "../stores/mooseStore";

const renderWithAsrMode = (asrMode: "gemini_live_audio" | "whisper_small") => {
  useMooseStore.setState({
    isSettingsOpen: true,
    settings: {
      ...frontendDefaultSettings(),
      text_provider: "google",
      asr_mode: asrMode,
    },
    settingsPersistenceError: null,
    hasApiKey: false,
  });
  render(<SettingsModal />);
  fireEvent.click(screen.getByText("AI & Models"));
};

describe("Google text and Gemini Live settings boundary", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("does not expose a Live voice model for local ASR with Google text", () => {
    renderWithAsrMode("whisper_small");

    expect(
      screen.getByLabelText("Gemini Text & Ambient Remark Model"),
    ).toBeInTheDocument();
    expect(
      screen.queryByLabelText("Realtime Live Voice Model"),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText(/Non-Live Talk uses the selected text provider/i),
    ).toBeInTheDocument();
  });

  it("exposes a Live model only for explicitly selected Gemini Live ASR", () => {
    renderWithAsrMode("gemini_live_audio");

    expect(
      screen.getByLabelText("Gemini Text & Ambient Remark Model"),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText("Realtime Live Voice Model"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /Gemini Live is a separate Google cloud-audio mode and may send microphone audio to Google/i,
      ),
    ).toBeInTheDocument();
  });
});
