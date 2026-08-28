import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tauriBridge } from "../lib/tauriBridge";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "../stores/mooseStore";
import { frontendDefaultSettings } from "../lib/backendContract";

describe("mooseStore State Management", () => {
  beforeEach(() => {
    resetSettingsPersistenceForTests();
    useMooseStore.setState({
      characterState: "idle",
      conversationLifecycle: "idle",
      mouthShape: "closed",
      isMuted: false,
      inputLevel: 0,
      outputLevel: 0,
      isOnboardingOpen: false,
      settings: null,
      hasApiKey: false,
      transcripts: [],
      partialUserTranscript: null,
      partialMooseTranscript: null,
    });
  });

  afterEach(() => {
    resetSettingsPersistenceForTests();
    vi.useRealTimers();
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

  it("coalesces continuous settings writes while updating local state immediately", async () => {
    vi.useFakeTimers();
    const persist = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockResolvedValue(undefined);
    const initial = frontendDefaultSettings();
    useMooseStore.setState({ settings: initial });

    useMooseStore.getState().updateSettingsContinuous({
      ...initial,
      talkativeness: 0.2,
    });
    useMooseStore.getState().updateSettingsContinuous({
      ...useMooseStore.getState().settings!,
      talkativeness: 0.4,
    });
    useMooseStore.getState().updateSettingsContinuous({
      ...useMooseStore.getState().settings!,
      talkativeness: 0.6,
    });

    expect(useMooseStore.getState().settings?.talkativeness).toBe(0.6);
    expect(persist).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(100);
    expect(persist).toHaveBeenCalledTimes(1);
    expect(persist.mock.calls[0][0].talkativeness).toBe(0.6);
  });

  it("cancels a pending continuous write before a discrete settings write", async () => {
    vi.useFakeTimers();
    const persist = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockResolvedValue(undefined);
    const initial = frontendDefaultSettings();
    useMooseStore.setState({ settings: initial });

    useMooseStore.getState().updateSettingsContinuous({
      ...initial,
      talkativeness: 0.7,
    });
    const latest = useMooseStore.getState().settings!;
    await useMooseStore.getState().updateSettings({
      ...latest,
      unsolicited_comments: false,
    });
    await vi.advanceTimersByTimeAsync(100);

    expect(persist).toHaveBeenCalledTimes(1);
    expect(persist.mock.calls[0][0]).toMatchObject({
      talkativeness: 0.7,
      unsolicited_comments: false,
    });
  });

  it("keeps a continuous edit made while a discrete write is in flight", async () => {
    vi.useFakeTimers();
    const initial = frontendDefaultSettings();
    useMooseStore.setState({ settings: initial });

    let resolveFirstWrite!: () => void;
    const firstWrite = new Promise<void>((resolve) => {
      resolveFirstWrite = resolve;
    });
    const persisted: (typeof initial)[] = [];
    const persist = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockImplementation(async (settings) => {
        persisted.push({ ...settings });
        if (persisted.length === 1) await firstWrite;
      });

    const discrete = useMooseStore.getState().updateSettings({
      ...initial,
      unsolicited_comments: false,
    });
    await vi.waitFor(() => expect(persist).toHaveBeenCalledTimes(1));

    useMooseStore.getState().updateSettingsContinuous({
      ...useMooseStore.getState().settings!,
      talkativeness: 0.8,
    });
    expect(useMooseStore.getState().settings).toMatchObject({
      unsolicited_comments: false,
      talkativeness: 0.8,
    });

    await vi.advanceTimersByTimeAsync(100);
    expect(persist).toHaveBeenCalledTimes(1);

    resolveFirstWrite();
    await discrete;
    await vi.waitFor(() => expect(persist).toHaveBeenCalledTimes(2));

    expect(persisted[1]).toMatchObject({
      unsolicited_comments: false,
      talkativeness: 0.8,
    });
    expect(useMooseStore.getState().settings).toEqual(persisted[1]);
  });

  it("folds a pending continuous edit into a later discrete write", async () => {
    vi.useFakeTimers();
    const initial = frontendDefaultSettings();
    useMooseStore.setState({ settings: initial });
    const persist = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockResolvedValue(undefined);

    useMooseStore.getState().updateSettingsContinuous({
      ...initial,
      talkativeness: 0.73,
    });
    await useMooseStore.getState().updateSettings({
      ...useMooseStore.getState().settings!,
      unsolicited_comments: false,
    });
    await vi.advanceTimersByTimeAsync(100);

    expect(persist).toHaveBeenCalledTimes(1);
    expect(persist).toHaveBeenCalledWith(
      expect.objectContaining({
        talkativeness: 0.73,
        unsolicited_comments: false,
      }),
    );
    expect(useMooseStore.getState().settings).toEqual(persist.mock.calls[0][0]);
  });

  it("rebases later settings edits after a rejected write without logging backend detail", async () => {
    vi.useFakeTimers();
    const initial = frontendDefaultSettings();
    useMooseStore.setState({ settings: initial });
    const privateFailure =
      "SECRET backend failure https://private.invalid/?key=AIzaSyDoNotLog";
    const persist = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockRejectedValueOnce(new Error(privateFailure))
      .mockResolvedValue(undefined);
    const reload = vi
      .spyOn(tauriBridge, "getSettings")
      .mockResolvedValue(initial);
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    const consoleWarn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const consoleLog = vi.spyOn(console, "log").mockImplementation(() => {});

    useMooseStore.getState().updateSettingsContinuous({
      ...initial,
      talkativeness: 0.91,
    });
    await vi.advanceTimersByTimeAsync(100);
    await vi.waitFor(() => expect(reload).toHaveBeenCalledTimes(1));

    expect(useMooseStore.getState().settings?.talkativeness).toBe(
      initial.talkativeness,
    );
    expect(consoleError).not.toHaveBeenCalled();
    expect(consoleWarn).not.toHaveBeenCalled();
    expect(consoleLog).not.toHaveBeenCalled();

    await useMooseStore.getState().updateSettings({
      ...useMooseStore.getState().settings!,
      unsolicited_comments: false,
    });

    expect(persist).toHaveBeenCalledTimes(2);
    expect(persist.mock.calls[1][0]).toMatchObject({
      talkativeness: initial.talkativeness,
      unsolicited_comments: false,
    });
    expect(JSON.stringify(persist.mock.calls[1][0])).not.toContain(
      privateFailure,
    );
  });

  it("reconciles a rejected discrete settings write to persisted state", async () => {
    const initial = frontendDefaultSettings();
    useMooseStore.setState({ settings: initial });
    vi.spyOn(tauriBridge, "updateSettings").mockRejectedValueOnce(
      new Error("private persistence detail"),
    );
    vi.spyOn(tauriBridge, "getSettings").mockResolvedValue(initial);

    await useMooseStore.getState().updateSettings({
      ...initial,
      volume: 0.25,
    });

    expect(useMooseStore.getState().settings).toEqual(initial);
  });

  it("marks the API key present immediately after a successful secure save", async () => {
    const save = vi
      .spyOn(tauriBridge, "setGoogleApiKey")
      .mockResolvedValue(undefined);

    await useMooseStore.getState().saveGoogleApiKey("AIzaSyFreshKey");

    expect(save).toHaveBeenCalledWith("AIzaSyFreshKey");
    expect(useMooseStore.getState().hasApiKey).toBe(true);
  });

  it("allocates collision-free ids for transcript finals in the same millisecond", async () => {
    const handlers = new Map<string, (payload: unknown) => void>();
    vi.spyOn(tauriBridge, "listenEvent").mockImplementation(
      async (eventName: string, handler: (payload: unknown) => void) => {
        handlers.set(eventName, handler);
        return () => {};
      },
    );
    vi.spyOn(Date, "now").mockReturnValue(1_725_000_000_000);

    const cleanup = await useMooseStore.getState().initEventListeners();
    handlers.get("moose://transcript/user")?.("first final");
    handlers.get("moose://transcript/moose")?.("second final");

    const ids = useMooseStore.getState().transcripts.map((entry) => entry.id);
    expect(ids).toHaveLength(2);
    expect(new Set(ids).size).toBe(2);
    expect(ids.every((id) => id < 0)).toBe(true);
    cleanup();
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

  it("clears key-presence state after secure key removal succeeds", async () => {
    const clear = vi
      .spyOn(tauriBridge, "clearGoogleApiKey")
      .mockResolvedValue(undefined);
    useMooseStore.setState({ hasApiKey: true });

    await useMooseStore.getState().clearGoogleApiKey();

    expect(clear).toHaveBeenCalledTimes(1);
    expect(useMooseStore.getState().hasApiKey).toBe(false);
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
  it("opens versioned onboarding even when an API key already exists", async () => {
    const settings = await tauriBridge.getSettings();
    vi.spyOn(tauriBridge, "getSettings").mockResolvedValue(settings);
    vi.spyOn(tauriBridge, "isMuted").mockResolvedValue(false);
    vi.spyOn(tauriBridge, "hasGoogleApiKey").mockResolvedValue(true);
    vi.spyOn(tauriBridge, "getOnboardingStatus").mockResolvedValue({
      current_version: 1,
      acknowledged_version: null,
      needs_acknowledgement: true,
    });
    vi.spyOn(tauriBridge, "getConversationLifecycle").mockResolvedValue("idle");
    vi.spyOn(tauriBridge, "getCharacterState").mockResolvedValue("idle");

    await useMooseStore.getState().loadSettings();

    expect(useMooseStore.getState().hasApiKey).toBe(true);
    expect(useMooseStore.getState().isOnboardingOpen).toBe(true);
  });

  it("keeps a migrated Gemini Live profile in onboarding until the version is acknowledged", async () => {
    const settings = {
      ...(await tauriBridge.getSettings()),
      asr_mode: "gemini_live_audio" as const,
    };
    vi.spyOn(tauriBridge, "getSettings").mockResolvedValue(settings);
    vi.spyOn(tauriBridge, "isMuted").mockResolvedValue(false);
    vi.spyOn(tauriBridge, "hasGoogleApiKey").mockResolvedValue(true);
    vi.spyOn(tauriBridge, "getOnboardingStatus").mockResolvedValue({
      current_version: 1,
      acknowledged_version: null,
      needs_acknowledgement: true,
    });
    vi.spyOn(tauriBridge, "getConversationLifecycle").mockResolvedValue("idle");
    vi.spyOn(tauriBridge, "getCharacterState").mockResolvedValue("idle");

    await useMooseStore.getState().loadSettings();

    expect(useMooseStore.getState().settings?.asr_mode).toBe(
      "gemini_live_audio",
    );
    expect(useMooseStore.getState().isOnboardingOpen).toBe(true);
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
