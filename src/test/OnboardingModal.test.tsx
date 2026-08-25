import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { OnboardingModal } from "../components/Onboarding/OnboardingModal";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";

describe("OnboardingModal ASR privacy", () => {
  beforeEach(async () => {
    useMooseStore.setState({
      isOnboardingOpen: true,
      settings: await tauriBridge.getSettings(),
      hasApiKey: false,
    });
  });

  it("describes the selected local ASR privacy boundary", () => {
    render(<OnboardingModal />);
    expect(
      screen.getByText(
        /active-app observation, cross-conversation memory, and transcript retention start Off/i,
      ),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByText("Next: Voice"));

    expect(screen.getByText(/Current mode:/)).toHaveTextContent(
      "Moonshine Tiny Streaming",
    );
    expect(
      screen.getByText(/processes microphone audio locally/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /never automatically switch to cloud microphone upload/i,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /active conversation, or for an explicit microphone test/i,
      ),
    ).toBeInTheDocument();
  });

  it("describes cloud microphone upload when Gemini Live is selected", () => {
    const current = useMooseStore.getState().settings!;
    useMooseStore.setState({
      settings: { ...current, asr_mode: "gemini_live_audio" },
    });

    render(<OnboardingModal />);
    fireEvent.click(screen.getByText("Next: Voice"));

    expect(screen.getByText("Gemini Live Cloud Audio")).toBeInTheDocument();
    expect(
      screen.getByText(
        /sends microphone audio to Google during the active conversation/i,
      ),
    ).toBeInTheDocument();
  });
});

describe("OnboardingModal P11 onboarding controls", () => {
  beforeEach(async () => {
    useMooseStore.setState({
      isOnboardingOpen: true,
      settings: await tauriBridge.getSettings(),
      hasApiKey: false,
    });
  });

  it("offers the Tiny model download without blocking onboarding", async () => {
    render(<OnboardingModal />);
    fireEvent.click(screen.getByText("Next: Voice"));
    fireEvent.click(screen.getByText("Next: Local Model"));

    expect(
      await screen.findByRole("button", { name: /Download Tiny Model/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Continue to Gemini Key/i }),
    ).toBeInTheDocument();
  });

  it("explains secure key storage and conservative defaults", () => {
    render(<OnboardingModal />);
    expect(
      screen.getByText(
        /active-app observation.*memory.*transcript retention start Off/i,
      ),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByText("Next: Voice"));
    fireEvent.click(screen.getByText("Next: Local Model"));
    fireEvent.click(screen.getByText("Continue to Gemini Key"));

    expect(
      screen.getByText(/never stored in the app settings database/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/stored in Keychain/i)).toBeInTheDocument();
  });

  it("updates key-presence state immediately when onboarding saves a key", async () => {
    vi.spyOn(tauriBridge, "setGoogleApiKey").mockResolvedValue(undefined);

    render(<OnboardingModal />);
    fireEvent.click(screen.getByText("Next: Voice"));
    fireEvent.click(screen.getByText("Next: Local Model"));
    fireEvent.click(screen.getByText("Continue to Gemini Key"));
    fireEvent.change(screen.getByLabelText(/Google Gemini API key/i), {
      target: { value: "AIzaSyOnboardingKey" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Save Key Securely/i }));

    await waitFor(() => expect(useMooseStore.getState().hasApiKey).toBe(true));
  });

  it("acknowledges onboarding without changing privacy settings", async () => {
    const current = useMooseStore.getState().settings!;
    expect(current.memory_enabled).toBe(false);
    expect(current.save_transcripts).toBe(false);

    const updateSettings = vi.spyOn(tauriBridge, "updateSettings");
    const acknowledge = vi
      .spyOn(tauriBridge, "acknowledgeOnboarding")
      .mockResolvedValue({
        current_version: 1,
        acknowledged_version: 1,
        needs_acknowledgement: false,
      });

    render(<OnboardingModal />);
    fireEvent.click(screen.getByRole("button", { name: /Skip onboarding/i }));

    await waitFor(() => expect(acknowledge).toHaveBeenCalledTimes(1));
    expect(updateSettings).not.toHaveBeenCalled();
    expect(useMooseStore.getState().settings?.memory_enabled).toBe(false);
    expect(useMooseStore.getState().settings?.save_transcripts).toBe(false);
    expect(useMooseStore.getState().isOnboardingOpen).toBe(false);
  });
});
