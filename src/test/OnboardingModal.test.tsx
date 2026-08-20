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
    fireEvent.click(screen.getByText("Next: Voice"));

    expect(screen.getByText(/Moonshine Tiny Streaming/)).toBeInTheDocument();
    expect(
      screen.getByText(/processes microphone audio locally/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /never automatically switch to cloud microphone upload/i,
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

    expect(screen.getByText(/Gemini Live Cloud Audio/)).toBeInTheDocument();
    expect(
      screen.getByText(
        /sends microphone audio to Google during the active conversation/i,
      ),
    ).toBeInTheDocument();
  });
});
