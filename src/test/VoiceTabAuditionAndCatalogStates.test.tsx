import { useState } from "react";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { VoiceTab } from "../components/Settings/VoiceTab";
import {
  frontendDefaultSettings,
  frontendTtsCatalog,
} from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";
import type { TtsCatalog } from "../types/moose";

// Mirrors SettingsModalBase, which owns the audition flag across tab switches.
const Harness = () => {
  const [isAuditioning, setIsAuditioning] = useState(false);
  return (
    <VoiceTab
      isAuditioning={isAuditioning}
      setIsAuditioning={setIsAuditioning}
    />
  );
};

describe("Voice tab catalog states and audition", () => {
  beforeEach(() => {
    useMooseStore.setState({
      settings: { ...frontendDefaultSettings(), tts_provider: "google" },
      hasApiKey: true,
      isConversationActive: false,
      inputDevices: [],
      outputDevices: [],
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("shows a loading status, with no voice select, until the catalog arrives", async () => {
    let resolveCatalog: (catalog: TtsCatalog) => void = () => {};
    vi.spyOn(tauriBridge, "getTtsCatalog").mockReturnValue(
      new Promise<TtsCatalog>((resolve) => {
        resolveCatalog = resolve;
      }),
    );
    render(<Harness />);
    expect(screen.getByText("Loading speech voices…")).toHaveAttribute(
      "role",
      "status",
    );
    expect(
      screen.queryByLabelText("Google Standalone Voice"),
    ).not.toBeInTheDocument();

    await act(async () => resolveCatalog(frontendTtsCatalog()));
    expect(
      await screen.findByLabelText("Google Standalone Voice"),
    ).toBeInTheDocument();
    expect(screen.queryByText("Loading speech voices…")).toBeNull();
  });

  it("shows an alert, not the loading status, when the catalog fails", async () => {
    vi.spyOn(tauriBridge, "getTtsCatalog").mockRejectedValue(
      new Error("private-path"),
    );
    render(<Harness />);
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Speech provider catalog is unavailable.");
    expect(alert).not.toHaveTextContent("private-path");
    expect(screen.queryByText("Loading speech voices…")).toBeNull();
  });

  it("reports an empty voice list for the selected provider", async () => {
    const catalog = frontendTtsCatalog();
    vi.spyOn(tauriBridge, "getTtsCatalog").mockResolvedValue({
      ...catalog,
      providers: catalog.providers.map((provider) =>
        provider.id === "google" ? { ...provider, voices: [] } : provider,
      ),
    });
    render(<Harness />);
    expect(
      await screen.findByText(
        "No voices are available for this speech provider.",
      ),
    ).toHaveAttribute("role", "status");
  });

  it("auditions the selected voice once and ignores clicks while busy", async () => {
    vi.spyOn(tauriBridge, "getTtsCatalog").mockResolvedValue(
      frontendTtsCatalog(),
    );
    let finish: () => void = () => {};
    const audition = vi.spyOn(tauriBridge, "auditionTtsVoice").mockReturnValue(
      new Promise<number>((resolve) => {
        finish = () => resolve(5000);
      }),
    );
    render(<Harness />);
    const voice = useMooseStore.getState().settings!.google_tts_voice;
    const button = await screen.findByRole("button", {
      name: /Audition Google Gemini/i,
    });
    await waitFor(() => expect(button).not.toBeDisabled());

    fireEvent.click(button);
    expect(audition).toHaveBeenCalledTimes(1);
    expect(audition).toHaveBeenCalledWith("google", voice);
    expect(button).toBeDisabled();

    fireEvent.click(button);
    expect(audition).toHaveBeenCalledTimes(1);

    vi.useFakeTimers();
    await act(async () => finish());
    // Still locked for the reported PCM duration and a short output-device margin.
    expect(button).toBeDisabled();
    await act(async () => {
      vi.advanceTimersByTime(5249);
    });
    expect(button).toBeDisabled();
    await act(async () => {
      vi.advanceTimersByTime(1);
    });
    expect(button).not.toBeDisabled();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("blocks Google audition without a saved key and explains why", async () => {
    useMooseStore.setState({ hasApiKey: false });
    vi.spyOn(tauriBridge, "getTtsCatalog").mockResolvedValue(
      frontendTtsCatalog(),
    );
    const audition = vi.spyOn(tauriBridge, "auditionTtsVoice");
    render(<Harness />);
    const button = await screen.findByRole("button", {
      name: /Audition Google Gemini/i,
    });
    expect(button).toBeDisabled();
    expect(
      screen.getByText(/Google TTS requires a saved Google API key/i),
    ).toBeInTheDocument();
    fireEvent.click(button);
    expect(audition).not.toHaveBeenCalled();
  });
});
