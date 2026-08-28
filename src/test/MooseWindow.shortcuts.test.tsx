import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MooseWindow } from "../windows/MooseWindow";
import { useMooseStore } from "../stores/mooseStore";
import { tauriBridge } from "../lib/tauriBridge";

vi.mock("../components/Moose/MooseController", () => ({
  MooseController: () => <div data-testid="moose-controller" />,
}));
vi.mock("../components/SpeechBubble/SpeechBubble", () => ({
  SpeechBubble: () => null,
}));
vi.mock("../components/Transcript/TranscriptDrawer", () => ({
  TranscriptDrawer: () => null,
}));
vi.mock("../components/Settings/SettingsModal", () => ({
  SettingsModal: () => null,
}));
vi.mock("../components/Onboarding/OnboardingModal", () => ({
  OnboardingModal: () => null,
}));

describe("MooseWindow focused keyboard shortcuts", () => {
  const startConversation = vi.fn(async () => undefined);
  const stopConversation = vi.fn(async () => undefined);
  const toggleMute = vi.fn(async () => undefined);
  const toggleSettings = vi.fn();
  const toggleOnboarding = vi.fn();
  const toggleTranscript = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    useMooseStore.setState({
      characterState: "idle",
      inputLevel: 0,
      outputLevel: 0,
      isMuted: false,
      isConversationActive: false,
      hasApiKey: true,
      isSettingsOpen: false,
      isOnboardingOpen: false,
      isTranscriptOpen: false,
      startConversation,
      stopConversation,
      toggleMute,
      toggleSettings,
      toggleOnboarding,
      toggleTranscript,
      loadSettings: vi.fn(async () => undefined),
      initEventListeners: vi.fn(async () => () => undefined),
    });
  });

  it("starts, mutes, and opens settings with focused-window shortcuts", () => {
    render(<MooseWindow />);

    fireEvent.keyDown(window, { key: "Enter", ctrlKey: true });
    expect(startConversation).toHaveBeenCalledTimes(1);

    fireEvent.keyDown(window, { key: "m", ctrlKey: true, shiftKey: true });
    expect(toggleMute).toHaveBeenCalledTimes(1);

    fireEvent.keyDown(window, { key: ",", metaKey: true });
    expect(toggleSettings).toHaveBeenCalledWith(true);
  });

  it("updates the title-bar AI badge immediately after saving a key", async () => {
    useMooseStore.setState({ hasApiKey: false });
    vi.spyOn(tauriBridge, "setGoogleApiKey").mockResolvedValue(undefined);
    render(<MooseWindow />);

    expect(screen.getByText("AI: NO KEY")).toBeInTheDocument();
    await useMooseStore.getState().saveGoogleApiKey("AIzaSyBadgeKey");

    await waitFor(() =>
      expect(screen.getByText("AI: READY")).toBeInTheDocument(),
    );
  });

  it("uses AA contrast classes for state badges and the active Stop control", async () => {
    render(<MooseWindow />);

    const expectedBadgeClasses = [
      ["listening", "LISTENING...", "bg-green-700"],
      ["thinking", "THINKING...", "bg-amber-700"],
      ["error", "ERROR", "bg-red-600"],
    ] as const;

    for (const [characterState, label, expectedClass] of expectedBadgeClasses) {
      act(() => useMooseStore.setState({ characterState }));
      const badge = await screen.findByRole("status");
      expect(badge).toHaveTextContent(label);
      expect(badge).toHaveClass(expectedClass, "text-white");
    }

    act(() => useMooseStore.setState({ isConversationActive: true }));
    const stop = await screen.findByRole("button", {
      name: "Stop conversation",
    });
    expect(stop).toHaveClass("bg-red-600", "text-white", "hover:bg-red-700");
  });

  it("derives the MUTED badge from the single mute authority", async () => {
    useMooseStore.setState({ characterState: "talking", isMuted: true });
    render(<MooseWindow />);

    const mutedBadge = await screen.findByRole("status");
    expect(mutedBadge).toHaveTextContent("MUTED");
    expect(mutedBadge).toHaveClass("bg-gray-600", "text-white");

    act(() => useMooseStore.setState({ isMuted: false }));
    expect(await screen.findByRole("status")).toHaveTextContent("TALKING");
  });

  it("stops an active conversation with the focused-window shortcut", () => {
    useMooseStore.setState({
      isConversationActive: true,
      isTranscriptOpen: false,
    });
    render(<MooseWindow />);

    fireEvent.keyDown(window, { key: "Enter", ctrlKey: true });
    expect(stopConversation).toHaveBeenCalledTimes(1);
  });

  it("closes the active panel with Escape and suppresses start/stop behind it", () => {
    useMooseStore.setState({
      isConversationActive: true,
      isTranscriptOpen: true,
    });
    render(<MooseWindow />);

    fireEvent.keyDown(window, { key: "Enter", ctrlKey: true });
    expect(stopConversation).not.toHaveBeenCalled();

    fireEvent.keyDown(window, { key: "Escape" });
    expect(toggleTranscript).toHaveBeenCalledWith(false);
  });
});
