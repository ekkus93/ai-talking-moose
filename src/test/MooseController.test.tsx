import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MooseController } from "../components/Moose/MooseController";
import { useMooseStore } from "../stores/mooseStore";
import { getMooseSprite } from "../lib/sprites";

describe("MooseController dismissal visibility", () => {
  const originalMatchMedia = window.matchMedia;

  beforeEach(() => {
    useMooseStore.setState({
      characterState: "idle",
      mouthShape: "closed",
      isBlinking: false,
      isConversationActive: false,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      value: originalMatchMedia,
    });
  });

  it.each(["hidden", "dismissed"] as const)(
    "does not render the Moose while %s",
    (characterState) => {
      useMooseStore.setState({ characterState });

      render(<MooseController />);

      expect(screen.queryByTestId("moose-sprite")).not.toBeInTheDocument();
    },
  );

  it("renders again after the authoritative state returns to idle", () => {
    useMooseStore.setState({ characterState: "idle" });

    render(<MooseController />);

    expect(screen.getByTestId("moose-sprite")).toBeInTheDocument();
  });

  it("stills blink and mouth frames when reduced motion is requested", () => {
    vi.useFakeTimers();
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      value: vi.fn().mockReturnValue({
        matches: true,
        media: "(prefers-reduced-motion: reduce)",
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }),
    });
    useMooseStore.setState({
      characterState: "talking",
      mouthShape: "wide",
      isBlinking: true,
    });

    render(<MooseController />);

    const expectedFrame = document.createElement("div");
    expectedFrame.innerHTML = getMooseSprite("talking", "closed", false);
    expect(screen.getByTestId("moose-sprite").innerHTML).toBe(
      expectedFrame.innerHTML,
    );
    expect(useMooseStore.getState().isBlinking).toBe(false);
    expect(vi.getTimerCount()).toBe(0);

    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(useMooseStore.getState().isBlinking).toBe(false);
  });
});
