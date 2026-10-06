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
      conversationError: null,
      mouthShape: "closed",
      isMuted: false,
      inputLevel: 0,
      outputLevel: 0,
      isOnboardingOpen: false,
      settings: null,
      settingsPersistenceError: null,
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

    useMooseStore.getState().updateSettingsContinuousPatch({
      talkativeness: 0.2,
    });
    useMooseStore.getState().updateSettingsContinuousPatch({
      talkativeness: 0.4,
    });
    useMooseStore.getState().updateSettingsContinuousPatch({
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

    useMooseStore.getState().updateSettingsContinuousPatch({
      talkativeness: 0.7,
    });
    await useMooseStore.getState().updateSettingsPatch({
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

    const discrete = useMooseStore.getState().updateSettingsPatch({
      unsolicited_comments: false,
    });
    await vi.waitFor(() => expect(persist).toHaveBeenCalledTimes(1));

    useMooseStore.getState().updateSettingsContinuousPatch({
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

    useMooseStore.getState().updateSettingsContinuousPatch({
      talkativeness: 0.73,
    });
    await useMooseStore.getState().updateSettingsPatch({
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

    useMooseStore.getState().updateSettingsContinuousPatch({
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

    await useMooseStore.getState().updateSettingsPatch({
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

  it("reconciles a rejected discrete settings write and reports rollback", async () => {
    const initial = frontendDefaultSettings();
    useMooseStore.setState({ settings: initial });
    vi.spyOn(tauriBridge, "updateSettings").mockRejectedValueOnce(
      new Error("private persistence detail"),
    );
    vi.spyOn(tauriBridge, "getSettings").mockResolvedValue(initial);

    const result = await useMooseStore.getState().updateSettingsPatch({
      volume: 0.25,
    });

    expect(result).toEqual({
      status: "rolled_back",
      message:
        "Settings could not be saved. Your last persisted settings were restored.",
    });
    expect(useMooseStore.getState().settings).toEqual(initial);
    expect(useMooseStore.getState().settingsPersistenceError).toBe(
      "Settings could not be saved. Your last persisted settings were restored.",
    );
    expect(JSON.stringify(result)).not.toContain("private persistence detail");
  });

  it("handles multiple sequential persistence failures deterministically", async () => {
    const initial = frontendDefaultSettings();
    useMooseStore.setState({ settings: initial });
    const persist = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockRejectedValueOnce(new Error("private failure one"))
      .mockRejectedValueOnce(new Error("private failure two"));
    const reload = vi
      .spyOn(tauriBridge, "getSettings")
      .mockResolvedValue(initial);

    const first = await useMooseStore.getState().updateSettingsPatch({
      volume: 0.31,
    });
    const second = await useMooseStore.getState().updateSettingsPatch({
      volume: 0.62,
    });

    expect(first).toEqual({
      status: "rolled_back",
      message:
        "Settings could not be saved. Your last persisted settings were restored.",
    });
    expect(second).toEqual(first);
    expect(persist).toHaveBeenCalledTimes(2);
    expect(reload).toHaveBeenCalledTimes(2);
    expect(useMooseStore.getState().settings).toEqual(initial);
    expect(useMooseStore.getState().settingsPersistenceError).toBe(
      "Settings could not be saved. Your last persisted settings were restored.",
    );
    expect(JSON.stringify({ first, second })).not.toContain("private failure");
  });

  it("reports persisted success and clears an earlier persistence error", async () => {
    const initial = frontendDefaultSettings();
    useMooseStore.setState({
      settings: initial,
      settingsPersistenceError: "previous failure",
    });
    vi.spyOn(tauriBridge, "updateSettings").mockResolvedValueOnce(undefined);

    const result = await useMooseStore.getState().updateSettingsPatch({
      volume: 0.42,
    });

    expect(result).toEqual({ status: "persisted" });
    expect(useMooseStore.getState().settingsPersistenceError).toBeNull();
  });

  it.each(["moose://state", "moose://transcript/moose", "moose://tray/action"])(
    "cleans up partial event registration when %s fails",
    async (failureEvent) => {
      const disposers: Array<ReturnType<typeof vi.fn>> = [];
      vi.spyOn(tauriBridge, "listenEvent").mockImplementation(
        async (eventName: string) => {
          if (eventName === failureEvent) {
            throw new Error("private listener registration detail");
          }
          const dispose = vi.fn();
          disposers.push(dispose);
          return dispose;
        },
      );

      await expect(
        useMooseStore.getState().initEventListeners(),
      ).rejects.toThrow(
        "Application event listeners could not be initialized.",
      );
      for (const dispose of disposers) {
        expect(dispose).toHaveBeenCalledTimes(1);
      }
    },
  );

  it("makes successful event-listener cleanup idempotent", async () => {
    const disposers: Array<ReturnType<typeof vi.fn>> = [];
    vi.spyOn(tauriBridge, "listenEvent").mockImplementation(async () => {
      const dispose = vi.fn();
      disposers.push(dispose);
      return dispose;
    });

    const cleanup = await useMooseStore.getState().initEventListeners();
    cleanup();
    cleanup();

    expect(disposers.length).toBeGreaterThan(0);
    for (const dispose of disposers) {
      expect(dispose).toHaveBeenCalledTimes(1);
    }
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

  it("surfaces a rejected local ASR startup error in conversationError", async () => {
    const userFacingError =
      "Local Moonshine ASR is selected, but the model installer is unavailable. No microphone audio was sent.";

    // The Rust side sends a user-facing message and owns the connecting/listening
    // lifecycle; it emits the failed lifecycle event before the command rejects,
    // so seed the lifecycle to pass the idle guard.
    vi.spyOn(tauriBridge, "startConversation").mockRejectedValueOnce(
      new Error(userFacingError),
    );
    useMooseStore.setState({ conversationLifecycle: "failed" });

    await useMooseStore.getState().startConversation();

    expect(useMooseStore.getState().conversationError).toBe(userFacingError);
  });

  it("normalizes a raw string rejection into conversationError", async () => {
    const rawString = "Local Moonshine ASR could not start";

    // Tauri commands return Result<(), String>, so a plain string rejection is
    // the real-world path; the typeof-e===string branch is what runs.
    vi.spyOn(tauriBridge, "startConversation").mockRejectedValueOnce(rawString);
    useMooseStore.setState({ conversationLifecycle: "failed" });

    await useMooseStore.getState().startConversation();

    expect(useMooseStore.getState().conversationError).toBe(rawString);
  });

  it("preserves the startup error on a failed lifecycle and clears it on every other lifecycle transition", async () => {
    const handlers = new Map<string, (payload: unknown) => void>();
    vi.spyOn(tauriBridge, "listenEvent").mockImplementation(
      async (eventName: string, handler: (payload: unknown) => void) => {
        handlers.set(eventName, handler);
        return () => {};
      },
    );

    const cleanup = await useMooseStore.getState().initEventListeners();

    useMooseStore.setState({
      conversationLifecycle: "failed",
      conversationError: "Local Moonshine ASR could not start",
    });

    // A "failed" transition must not wipe the startup error.
    handlers.get("moose://conversation/lifecycle")?.("failed");
    expect(useMooseStore.getState().conversationError).toBe(
      "Local Moonshine ASR could not start",
    );

    // Every other lifecycle transition clears it.
    for (const lifecycle of [
      "idle",
      "connecting",
      "listening",
      "responding",
      "stopping",
    ]) {
      handlers.get("moose://conversation/lifecycle")?.(lifecycle);
      expect(useMooseStore.getState().conversationError).toBeNull();
    }

    cleanup();
  });

  it("does not set conversationError when the lifecycle is already idle (stale banner guard)", async () => {
    vi.spyOn(tauriBridge, "startConversation").mockRejectedValueOnce(
      new Error("Local Moonshine ASR could not start"),
    );
    useMooseStore.setState({ conversationLifecycle: "idle" });

    await useMooseStore.getState().startConversation();

    expect(useMooseStore.getState().conversationError).toBeNull();
  });

  it("never logs a rejected startConversation rejection to the console", async () => {
    const privateFailure =
      "SECRET backend failure https://private.invalid/?key=AIzaSyDoNotLog";

    vi.spyOn(tauriBridge, "startConversation").mockRejectedValueOnce(
      new Error(privateFailure),
    );

    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    const consoleWarn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const consoleLog = vi.spyOn(console, "log").mockImplementation(() => {});

    useMooseStore.setState({ conversationLifecycle: "failed" });
    await useMooseStore.getState().startConversation();

    // Backend detail stays out of the frontend console; the user-facing string is
    // carried in state (the display channel), not logged.
    expect(consoleError).not.toHaveBeenCalled();
    expect(consoleWarn).not.toHaveBeenCalled();
    expect(consoleLog).not.toHaveBeenCalled();
  });
});
