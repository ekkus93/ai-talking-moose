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

  it("does not re-show an ambient bubble after the backend lifecycle completes", async () => {
    vi.spyOn(tauriBridge, "triggerAmbientRemark").mockResolvedValue(
      "A bounded ambient remark.",
    );

    await useMooseStore.getState().triggerAmbient("safe event");

    expect(useMooseStore.getState().speechBubbleVisible).toBe(false);
    expect(useMooseStore.getState().speechBubbleText).toBeNull();
  });

  it("hides the bubble when the backend explicitly clears speech text", async () => {
    useMooseStore.getState().showSpeechBubble("stale response");
    expect(useMooseStore.getState().speechBubbleVisible).toBe(true);

    vi.spyOn(tauriBridge, "listenEvent").mockImplementation(
      async (eventName: string, handler: (payload: unknown) => void) => {
        if (eventName === "moose://speech-bubble") {
          handler("");
        }
        return () => {};
      },
    );

    const cleanup = await useMooseStore.getState().initEventListeners();

    expect(useMooseStore.getState().speechBubbleVisible).toBe(false);
    expect(useMooseStore.getState().speechBubbleText).toBeNull();
    cleanup();
  });
  it("routes tray and menu-bar actions through bounded store commands", async () => {
    const handlers = new Map<string, (payload: unknown) => void>();
    vi.spyOn(tauriBridge, "listenEvent").mockImplementation(
      async (eventName: string, handler: (payload: unknown) => void) => {
        handlers.set(eventName, handler);
        return () => {};
      },
    );
    const start = vi
      .spyOn(tauriBridge, "startConversation")
      .mockResolvedValue("test-session");
    const stop = vi
      .spyOn(tauriBridge, "stopConversation")
      .mockResolvedValue(undefined);
    const setMute = vi
      .spyOn(tauriBridge, "setMute")
      .mockResolvedValue(undefined);
    vi.spyOn(tauriBridge, "resizeWindow").mockResolvedValue(undefined);

    useMooseStore.setState({
      isConversationActive: false,
      isMuted: false,
      isSettingsOpen: false,
    });
    const cleanup = await useMooseStore.getState().initEventListeners();
    const trayAction = handlers.get("moose://tray/action");
    const openSettings = handlers.get("moose://ui/open-settings");

    expect(trayAction).toBeDefined();
    expect(openSettings).toBeDefined();

    trayAction?.("start_conversation");
    await vi.waitFor(() => expect(start).toHaveBeenCalledTimes(1));

    useMooseStore.setState({ isConversationActive: true });
    trayAction?.("stop_conversation");
    await vi.waitFor(() => expect(stop).toHaveBeenCalledTimes(1));

    useMooseStore.setState({ isMuted: false });
    trayAction?.("mute");
    await vi.waitFor(() => expect(setMute).toHaveBeenLastCalledWith(true));

    trayAction?.("unmute");
    await vi.waitFor(() => expect(setMute).toHaveBeenLastCalledWith(false));

    openSettings?.(undefined);
    expect(useMooseStore.getState().isSettingsOpen).toBe(true);

    cleanup();
  });
});
