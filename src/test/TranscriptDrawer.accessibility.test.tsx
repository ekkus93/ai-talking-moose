import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { TranscriptDrawer } from "../components/Transcript/TranscriptDrawer";
import { useMooseStore } from "../stores/mooseStore";

describe("TranscriptDrawer accessibility", () => {
  beforeEach(() => {
    useMooseStore.setState({
      isTranscriptOpen: true,
      transcripts: [
        {
          id: 1,
          session_id: "test",
          role: "user",
          text: "Hello Moose",
          created_at: "12:00",
        },
      ],
      partialUserTranscript: "still speaking",
      partialMooseTranscript: null,
    });
  });

  it("uses audited readable colors for empty guidance and the prompt marker", () => {
    useMooseStore.setState({ transcripts: [], partialUserTranscript: null });
    render(<TranscriptDrawer />);

    const emptyState = screen.getByText(
      "Interactive Debug Terminal",
    ).parentElement;
    expect(emptyState).toHaveClass("text-gray-600");
    expect(screen.getByText(">", { selector: "span" })).toHaveClass(
      "text-green-800",
    );
  });

  it("announces finalized and streaming transcript updates as a polite log", () => {
    render(<TranscriptDrawer />);

    const log = screen.getByRole("log", { name: "Conversation transcript" });
    expect(log).toHaveAttribute("aria-live", "polite");
    expect(log).toHaveAttribute("aria-relevant", "additions text");
    expect(log).toHaveTextContent("Hello Moose");
    expect(log).toHaveTextContent("still speaking");
  });
});
