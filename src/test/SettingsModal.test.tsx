import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, beforeEach } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { useMooseStore } from "../stores/mooseStore";

describe("SettingsModal Component", () => {
  beforeEach(() => {
    useMooseStore.setState({
      isSettingsOpen: true,
      settings: {
        settings_version: 2,
        asr_mode: "moonshine_tiny_streaming",
        launch_at_login: false,
        show_in_menu_bar: true,
        always_on_top: false,
        restore_position: true,
        unsolicited_comments: true,
        talkativeness: 0.5,
        quiet_hours_enabled: true,
        quiet_hours_start: 22,
        quiet_hours_end: 8,
        max_comments_per_hour: 4,
        hide_delay_seconds: 6,
        input_device: null,
        output_device: null,
        volume: 1.0,
        tts_voice: "Fenrir",
        speaking_rate: 0.95,
        pitch: -1.5,
        provider: "google",
        live_model: "gemini-3.1-flash-live-preview",
        text_model: "gemini-3.7-flash",
        tts_model: "en-US-Standard-B",
        microphone_permission_granted: false,
        active_app_observation: false,
        window_title_observation: false,
        memory_enabled: false,
        save_transcripts: false,
        dry: 0.85,
        sarcastic: 0.7,
        friendly: 0.55,
        absurd: 0.65,
        helpful: 0.35,
        verbosity: 0.3,
      },
    });
  });

  it("renders tabs and navigates to personality", () => {
    render(<SettingsModal />);
    expect(screen.getByTestId("settings-modal")).toBeInTheDocument();

    const personalityTab = screen.getByText("Personality");
    fireEvent.click(personalityTab);

    expect(screen.getByText("Moose Character Persona")).toBeInTheDocument();
    expect(screen.getByText("Dry Wit & Deadpan")).toBeInTheDocument();
  });

  it("offers explicit local and cloud ASR modes with privacy copy", async () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("Speech Recognition"));

    const tiny = await screen.findByRole("radio", {
      name: /Moonshine Tiny Streaming/,
    });
    expect(tiny).toBeChecked();
    expect(
      screen.getByText(/Only finalized transcript text is sent to Gemini/i),
    ).toBeInTheDocument();
    expect(screen.getAllByText("Not installed")).toHaveLength(2);

    fireEvent.click(
      screen.getByRole("radio", { name: /Gemini Live Cloud Audio/ }),
    );
    await waitFor(() =>
      expect(useMooseStore.getState().settings?.asr_mode).toBe(
        "gemini_live_audio",
      ),
    );
    expect(screen.getByText(/Cloud mode selected:/i)).toBeInTheDocument();
    expect(
      screen.getByText(
        /never switch to cloud microphone upload automatically/i,
      ),
    ).toBeInTheDocument();
  });

  it("exposes audio diagnostics and local microphone test controls", async () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("Diagnostics"));

    expect(screen.getByText("Audio Diagnostics")).toBeInTheDocument();
    expect(screen.getByText("Test Microphone")).toBeInTheDocument();
    expect(screen.getByText("Test Output")).toBeInTheDocument();
    expect(await screen.findByText("Unavailable")).toBeInTheDocument();
  });
});
