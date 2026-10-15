import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { VoiceTab } from "../components/Settings/VoiceTab";
import {
  frontendDefaultSettings,
  frontendTtsCatalog,
} from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";
import type { TtsCatalog } from "../types/moose";

// What the backend returns on macOS: the static providers plus a runtime System provider.
function macCatalog(): TtsCatalog {
  const base = frontendTtsCatalog();
  return {
    ...base,
    providers: [
      ...base.providers,
      {
        id: "system",
        display_name: "macOS System Voices",
        is_local: true,
        models: [],
        voices: [
          { id: "system_default", display_name: "System Default", style: null },
          {
            id: "com.apple.voice.compact.en-US.Samantha",
            display_name: "Samantha",
            style: "en-US · Standard",
          },
        ],
        sample_rate_hz: 22050,
        supports_speaking_rate: true,
        supports_pitch: false,
        install_required: false,
        license_summary: "Uses installed macOS voices.",
      },
    ],
  };
}

describe("System Voices on macOS", () => {
  const patch = vi.fn().mockResolvedValue(undefined);

  beforeEach(() => {
    patch.mockClear();
    useMooseStore.setState({
      settings: { ...frontendDefaultSettings(), tts_provider: "local" },
      hasApiKey: true,
      isConversationActive: false,
      inputDevices: [],
      outputDevices: [],
      updateSettingsPatch: patch,
    });
    vi.spyOn(tauriBridge, "cancelStandaloneSpeech").mockResolvedValue(
      undefined as never,
    );
  });

  afterEach(() => vi.restoreAllMocks());

  it("enables the System radio only when the catalog lists the provider", async () => {
    vi.spyOn(tauriBridge, "getTtsCatalog").mockResolvedValue(macCatalog());
    render(<VoiceTab isAuditioning={false} setIsAuditioning={vi.fn()} />);
    const radio = await screen.findByRole("radio", {
      name: /System Voices — on this computer/i,
    });
    await waitFor(() => expect(radio).toBeEnabled());
    fireEvent.click(radio);
    await waitFor(() =>
      expect(patch).toHaveBeenCalledWith({ tts_provider: "system" }),
    );
  });

  it("saves a chosen system voice and auditions it through the system provider", async () => {
    useMooseStore.setState({
      settings: { ...frontendDefaultSettings(), tts_provider: "system" },
    });
    vi.spyOn(tauriBridge, "getTtsCatalog").mockResolvedValue(macCatalog());
    const audition = vi
      .spyOn(tauriBridge, "auditionTtsVoice")
      .mockResolvedValue("ok");
    render(<VoiceTab isAuditioning={false} setIsAuditioning={vi.fn()} />);
    const select = await screen.findByLabelText("macOS System Voice");
    fireEvent.change(select, {
      target: { value: "com.apple.voice.compact.en-US.Samantha" },
    });
    await waitFor(() =>
      expect(patch).toHaveBeenCalledWith({
        system_tts_voice: "com.apple.voice.compact.en-US.Samantha",
      }),
    );
    const button = await screen.findByRole("button", { name: /Audition/i });
    await waitFor(() => expect(button).toBeEnabled());
    fireEvent.click(button);
    expect(audition).toHaveBeenCalledWith("system", "system_default");
  });

  it("refreshes the installed voice list on demand", async () => {
    const catalogs = vi
      .spyOn(tauriBridge, "getTtsCatalog")
      .mockResolvedValue(macCatalog());
    render(<VoiceTab isAuditioning={false} setIsAuditioning={vi.fn()} />);
    fireEvent.click(
      await screen.findByRole("button", { name: "Refresh System Voices" }),
    );
    await waitFor(() => expect(catalogs).toHaveBeenCalledTimes(2));
  });

  it("blocks audition and says why when the saved system voice is no longer installed", async () => {
    useMooseStore.setState({
      settings: {
        ...frontendDefaultSettings(),
        tts_provider: "system",
        system_tts_voice: "com.apple.voice.removed",
      },
    });
    vi.spyOn(tauriBridge, "getTtsCatalog").mockResolvedValue(macCatalog());
    const audition = vi.spyOn(tauriBridge, "auditionTtsVoice");
    render(<VoiceTab isAuditioning={false} setIsAuditioning={vi.fn()} />);
    const button = await screen.findByRole("button", { name: /Audition/i });
    expect(button).toBeDisabled();
    expect(
      screen.getByText(/saved system voice is not installed/i),
    ).toBeInTheDocument();
    fireEvent.click(button);
    expect(audition).not.toHaveBeenCalled();
  });
});
