import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { VoiceTab } from "../components/Settings/VoiceTab";
import { frontendDefaultSettings, frontendTtsCatalog } from "../lib/backendContract";
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
    vi.spyOn(tauriBridge, "getTtsCatalog").mockResolvedValue(frontendTtsCatalog());
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("reports a rejected standalone voice patch without exposing raw errors", async () => {
    const patch = vi.fn().mockRejectedValue(new Error("private-key-in-native-error"));
    useMooseStore.setState({ updateSettingsPatch: patch });
    render(<VoiceTab isAuditioning={false} setIsAuditioning={vi.fn()} />);
    const selector = await screen.findByLabelText("Google Standalone Voice");
    const voice = frontendTtsCatalog().providers[0].voices[1].id;
    fireEvent.change(selector, { target: { value: voice } });
    await waitFor(() =>
      expect(patch).toHaveBeenCalledWith({ google_tts_voice: voice }),
    );
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Could not save the standalone voice selection");
    expect(alert).not.toHaveTextContent("private-key-in-native-error");
  });

  it("reports a rejected Live voice patch without leaking provider details", async () => {
    const patch = vi.fn().mockRejectedValue(new Error("private-transcript-in-native-error"));
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
    expect(alert).toHaveTextContent("Could not save the Gemini Live voice selection");
    expect(alert).not.toHaveTextContent("private-transcript-in-native-error");
  });
});
