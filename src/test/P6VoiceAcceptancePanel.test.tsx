import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { P6VoiceAcceptancePanel } from "../components/Settings/P6VoiceAcceptancePanel";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";

describe("P6VoiceAcceptancePanel", () => {
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
        volume: 1,
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

  it("exposes all 30 voices and explicit audition cancellation", async () => {
    const auditionSpy = vi
      .spyOn(tauriBridge, "auditionVoice")
      .mockImplementation(() => new Promise(() => undefined));
    const cancelSpy = vi
      .spyOn(tauriBridge, "cancelStandaloneSpeech")
      .mockResolvedValue();

    render(<P6VoiceAcceptancePanel />);
    fireEvent.click(
      screen.getByRole("button", { name: "P6 Voice Acceptance" }),
    );

    const select = await screen.findByLabelText("P6 voice under test");
    await waitFor(() =>
      expect(select.querySelectorAll("option")).toHaveLength(30),
    );
    expect(
      screen.getByRole("option", { name: /Fenrir.*Provisional default/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("option", { name: /Sulafat \(Warm\)/i }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Audition Fenrir/i }));
    const stop = await screen.findByRole("button", { name: /Stop Sample/i });
    fireEvent.click(stop);

    await waitFor(() => expect(cancelSpy).toHaveBeenCalledTimes(1));
    expect(auditionSpy).toHaveBeenCalledWith("Fenrir");
    auditionSpy.mockRestore();
    cancelSpy.mockRestore();
  });
  it("scopes Escape to the active voice panel and restores launcher focus", async () => {
    render(<P6VoiceAcceptancePanel />);
    const launcher = screen.getByRole("button", {
      name: "P6 Voice Acceptance",
    });
    launcher.focus();
    fireEvent.click(launcher);

    const panel = await screen.findByRole("dialog", {
      name: "P6 Voice Acceptance",
    });
    expect(panel).toBeInTheDocument();
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Close P6 Voice Acceptance" }),
      ).toHaveFocus(),
    );

    fireEvent.keyDown(window, { key: "Escape" });

    await waitFor(() => {
      expect(
        screen.queryByRole("dialog", { name: "P6 Voice Acceptance" }),
      ).not.toBeInTheDocument();
      expect(useMooseStore.getState().isSettingsOpen).toBe(true);
      expect(
        screen.getByRole("button", { name: "P6 Voice Acceptance" }),
      ).toHaveFocus();
    });
  });
});
