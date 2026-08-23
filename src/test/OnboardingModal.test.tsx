import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { OnboardingModal } from "../components/Onboarding/OnboardingModal";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";

describe("OnboardingModal ASR privacy", () => {
  beforeEach(async () => {
    useMooseStore.setState({
      isOnboardingOpen: true,
      settings: await tauriBridge.getSettings(),
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
});
