import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";

describe("mooseStore State Management", () => {
  beforeEach(() => {
    useMooseStore.setState({
      characterState: "idle",
      mouthShape: "closed",
      isMuted: false,
      inputLevel: 0,
      outputLevel: 0,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    useMooseStore.getState().hideSpeechBubble();
  });

  it("updates character state correctly", () => {
    const store = useMooseStore.getState();
    store.setCharacterState("listening");
    expect(useMooseStore.getState().characterState).toBe("listening");
  });

  it("updates mouth shapes and levels", () => {
    const store = useMooseStore.getState();
    store.setMouthShape("wide");
    store.setInputLevel(0.75);
    store.setOutputLevel(0.9);

    expect(useMooseStore.getState().mouthShape).toBe("wide");
    expect(useMooseStore.getState().inputLevel).toBe(0.75);
    expect(useMooseStore.getState().outputLevel).toBe(0.9);
  });

  it("shows sanitized provider errors from backend events", async () => {
    vi.spyOn(tauriBridge, "listenEvent").mockImplementation(
      async (eventName: string, handler: (payload: unknown) => void) => {
        if (eventName === "moose://conversation/error") {
          handler({
            kind: "quota",
            message:
              "Provider quota or rate limit was reached. Try again later or review your quota.",
            retryable: true,
          });
        }
        return () => {};
      },
    );

    const cleanup = await useMooseStore.getState().initEventListeners();

    expect(useMooseStore.getState().speechBubbleVisible).toBe(true);
    expect(useMooseStore.getState().speechBubbleText).toContain("quota");
    cleanup();
  });
});
