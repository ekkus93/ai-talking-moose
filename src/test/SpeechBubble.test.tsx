import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach } from "vitest";
import { SpeechBubble } from "../components/SpeechBubble/SpeechBubble";
import { useMooseStore } from "../stores/mooseStore";

describe("SpeechBubble Component", () => {
  beforeEach(() => {
    useMooseStore.setState({
      speechBubbleText: null,
      speechBubbleVisible: false,
    });
  });

  it("does not render when speechBubbleVisible is false", () => {
    render(<SpeechBubble />);
    expect(screen.queryByTestId("speech-bubble")).not.toBeInTheDocument();
  });

  it("renders text when visible and dismisses on click", () => {
    useMooseStore.getState().showSpeechBubble("Hey! Back to work, pal.");
    render(<SpeechBubble />);

    const bubble = screen.getByTestId("speech-bubble");
    expect(bubble).toBeInTheDocument();
    expect(bubble).toHaveTextContent("Hey! Back to work, pal.");

    // Dismiss by clicking
    fireEvent.click(bubble);
    expect(useMooseStore.getState().speechBubbleVisible).toBe(false);
  });
});
