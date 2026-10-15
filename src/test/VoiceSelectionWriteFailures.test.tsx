import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { VoiceTab } from "../components/Settings/VoiceTab";
import {
  frontendDefaultSettings,
  frontendTtsCatalog,
} from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";

describe("voice selection persistence failures", () => {
  beforeEach(() => {
    useMooseStore.setState({
      settings: { ...frontendDefaultSettings(), tts_provider: "google" },
      hasApiKey: true,
      isConversationActive: true,
      inputDevices: [],
      outputDevices: [],
    });
    vi.spyOn(tauriBridge, "getTtsCatalog").mockResolvedValue(
      frontendTtsCatalog(),
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("sanitizes failed standalone provider switching", async () => {
    const patch = vi.fn().mockRejectedValue(new Error("private-device-path"));
    useMooseStore.setState({ updateSettingsPatch: patch });
    render(<VoiceTab isAuditioning={false} setIsAuditioning={vi.fn()} />);
    fireEvent.click(
      await screen.findByRole("radio", { name: /Local KittenTTS/i }),
    );
    await waitFor(() =>
      expect(patch).toHaveBeenCalledWith({ tts_provider: "local" }),
    );
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(
      "Could not switch standalone speech provider",
    );
    expect(alert).not.toHaveTextContent("private-device-path");
  });

  it("hides backend details when a standalone audition fails", async () => {
    vi.spyOn(tauriBridge, "auditionTtsVoice").mockRejectedValue(
      new Error("private-transcript-in-audio-error"),
    );
    render(<VoiceTab isAuditioning={false} setIsAuditioning={vi.fn()} />);
    fireEvent.click(
      await screen.findByRole("button", { name: /Audition Google Gemini/i }),
    );
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Google TTS audition failed");
    expect(alert).not.toHaveTextContent("private-transcript-in-audio-error");
  });

  it("shows a sanitized error for failed standalone voice save", async () => {
    const patch = vi.fn().mockRejectedValue(new Error("private-key"));
    useMooseStore.setState({ updateSettingsPatch: patch });
    render(<VoiceTab isAuditioning={false} setIsAuditioning={vi.fn()} />);
    const selector = await screen.findByLabelText("Google Standalone Voice");
    const voice = frontendTtsCatalog().providers[0].voices[1].id;
    fireEvent.change(selector, { target: { value: voice } });
    await waitFor(() =>
      expect(patch).toHaveBeenCalledWith({ google_tts_voice: voice }),
    );
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(
      "Could not save the standalone voice selection",
    );
    expect(alert).not.toHaveTextContent("private-key");
  });

  it("shows a sanitized error for failed Live voice save", async () => {
    const patch = vi.fn().mockRejectedValue(new Error("private-transcript"));
    useMooseStore.setState({ updateSettingsPatch: patch });
    render(<VoiceTab isAuditioning={false} setIsAuditioning={vi.fn()} />);
    const selector = await screen.findByLabelText("Gemini Live Voice");
    await waitFor(() => expect(selector).not.toBeDisabled());
    const voice = frontendTtsCatalog().gemini_live.voices[1].id;
    fireEvent.change(selector, { target: { value: voice } });
    await waitFor(() =>
      expect(patch).toHaveBeenCalledWith({ live_voice: voice }),
    );
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(
      "Could not save the Gemini Live voice selection",
    );
    expect(alert).not.toHaveTextContent("private-transcript");
  });
});
