import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { P6VoiceAcceptancePanel } from "../components/Settings/P6VoiceAcceptancePanel";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";
import {
  frontendDefaultSettings,
  frontendGoogleTtsVoices,
} from "../lib/backendContract";

describe("P6VoiceAcceptancePanel", () => {
  beforeEach(() => {
    useMooseStore.setState({
      isSettingsOpen: true,
      settings: frontendDefaultSettings(),
      googleTtsVoices: frontendGoogleTtsVoices(),
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
