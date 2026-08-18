import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { useMooseStore } from "../stores/mooseStore";

describe("SettingsModal Component", () => {
  beforeEach(() => {
    useMooseStore.setState({
      isSettingsOpen: true,
      settings: {
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
        live_model: "gemini-2.5-flash-native-audio-latest",
        text_model: "gemini-2.5-flash",
        tts_model: "en-US-Standard-B",
        microphone_permission_granted: true,
        active_app_observation: true,
        window_title_observation: false,
        memory_enabled: true,
        save_transcripts: true,
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
});
