import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { VoiceTab } from "../components/Settings/VoiceTab";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";

describe("standalone voice provider switching and Live isolation", () => {
  beforeEach(() => {
    useMooseStore.setState({
      settings: { ...frontendDefaultSettings(), tts_provider: "google" },
      isConversationActive: true,
      hasApiKey: true,
      inputDevices: [],
      outputDevices: [],
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("keeps Live playback running during TTS provider changes", async () => {
    const cancel = vi
      .spyOn(tauriBridge, "cancelStandaloneSpeech")
      .mockResolvedValue(undefined);
    const patch = vi.fn().mockResolvedValue(undefined);
    useMooseStore.setState({ updateSettingsPatch: patch });
    render(<VoiceTab isAuditioning={false} setIsAuditioning={vi.fn()} />);
    fireEvent.click(screen.getByRole("radio", { name: /Local KittenTTS/i }));
    await waitFor(() =>
      expect(patch).toHaveBeenCalledWith({ tts_provider: "local" }),
    );
    expect(cancel).not.toHaveBeenCalled();
  });

  it("preempts idle speech before switching provider", async () => {
    const cancel = vi
      .spyOn(tauriBridge, "cancelStandaloneSpeech")
      .mockResolvedValue(undefined);
    const patch = vi.fn().mockResolvedValue(undefined);
    useMooseStore.setState({
      isConversationActive: false,
      updateSettingsPatch: patch,
    });
    render(<VoiceTab isAuditioning={false} setIsAuditioning={vi.fn()} />);
    fireEvent.click(screen.getByRole("radio", { name: /Local KittenTTS/i }));
    await waitFor(() =>
      expect(patch).toHaveBeenCalledWith({ tts_provider: "local" }),
    );
    expect(cancel).toHaveBeenCalledTimes(1);
    expect(cancel.mock.invocationCallOrder[0]).toBeLessThan(
      patch.mock.invocationCallOrder[0],
    );
  });
});
