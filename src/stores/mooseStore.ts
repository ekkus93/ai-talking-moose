import { create } from "zustand";
import {
  AppSettings,
  AudioDeviceInfo,
  CharacterState,
  ConversationLifecycle,
  GoogleTtsVoiceDescriptor,
  MemoryRecord,
  MouthShape,
  ProviderError,
  TranscriptRecord,
} from "../types/moose";
import { tauriBridge } from "../lib/tauriBridge";

const lifecycleIsActive = (lifecycle: ConversationLifecycle) =>
  lifecycle !== "idle" && lifecycle !== "failed";

const CONTINUOUS_SETTINGS_WRITE_DELAY_MS = 100;
type SettingsPatch = Partial<AppSettings>;

interface QueuedSettingsWrite {
  patch: SettingsPatch;
  complete: () => void;
}

let continuousSettingsWriteTimer: ReturnType<typeof setTimeout> | null = null;
let pendingContinuousSettingsPatch: SettingsPatch | null = null;
let settingsWriteQueue: QueuedSettingsWrite[] = [];
let settingsWriteWorkerRunning = false;
let lastPersistedSettings: AppSettings | null = null;

const cloneSettings = (settings: AppSettings): AppSettings => ({ ...settings });

const settingsPatch = (
  current: AppSettings,
  next: AppSettings,
): SettingsPatch => {
  const patch: SettingsPatch = {};
  for (const key of Object.keys(next) as Array<keyof AppSettings>) {
    if (!Object.is(current[key], next[key])) {
      Object.assign(patch, { [key]: next[key] });
    }
  }
  return patch;
};

const applySettingsPatch = (
  settings: AppSettings,
  patch: SettingsPatch,
): AppSettings => ({ ...settings, ...patch });

const patchIsEmpty = (patch: SettingsPatch) => Object.keys(patch).length === 0;

const ensurePersistedSettingsBaseline = (settings: AppSettings | null) => {
  if (lastPersistedSettings === null && settings !== null) {
    lastPersistedSettings = cloneSettings(settings);
  }
};

const rebuildOptimisticSettings = (persisted: AppSettings): AppSettings => {
  let rebuilt = cloneSettings(persisted);
  for (const queued of settingsWriteQueue) {
    rebuilt = applySettingsPatch(rebuilt, queued.patch);
  }
  if (pendingContinuousSettingsPatch) {
    rebuilt = applySettingsPatch(rebuilt, pendingContinuousSettingsPatch);
  }
  return rebuilt;
};

const reconcileSettingsAfterWriteFailure = async () => {
  let authoritative = lastPersistedSettings;
  try {
    authoritative = await tauriBridge.getSettings();
  } catch {
    // The last successfully persisted snapshot is the safe fallback. Backend
    // error details intentionally stay out of the frontend console.
  }

  if (!authoritative) return;
  lastPersistedSettings = cloneSettings(authoritative);
  useMooseStore.setState({
    settings: rebuildOptimisticSettings(authoritative),
  });
};

const processSettingsWriteQueue = async () => {
  if (settingsWriteWorkerRunning) return;
  settingsWriteWorkerRunning = true;

  try {
    while (settingsWriteQueue.length > 0) {
      const queued = settingsWriteQueue[0];
      const baseline =
        lastPersistedSettings ?? useMooseStore.getState().settings;
      if (!baseline) {
        settingsWriteQueue.shift();
        queued.complete();
        continue;
      }

      const candidate = applySettingsPatch(baseline, queued.patch);
      try {
        await tauriBridge.updateSettings(candidate);
        lastPersistedSettings = cloneSettings(candidate);
        settingsWriteQueue.shift();
        queued.complete();
      } catch {
        settingsWriteQueue.shift();
        queued.complete();
        await reconcileSettingsAfterWriteFailure();
      }
    }
  } finally {
    settingsWriteWorkerRunning = false;
    if (settingsWriteQueue.length > 0) {
      void processSettingsWriteQueue();
    }
  }
};

const enqueueSettingsPatch = (patch: SettingsPatch): Promise<void> => {
  if (patchIsEmpty(patch)) return Promise.resolve();
  return new Promise((complete) => {
    settingsWriteQueue.push({ patch, complete });
    void processSettingsWriteQueue();
  });
};

const cancelPendingContinuousSettingsTimer = () => {
  if (continuousSettingsWriteTimer) {
    clearTimeout(continuousSettingsWriteTimer);
  }
  continuousSettingsWriteTimer = null;
};

const takePendingContinuousSettingsPatch = (): SettingsPatch => {
  cancelPendingContinuousSettingsTimer();
  const patch = pendingContinuousSettingsPatch ?? {};
  pendingContinuousSettingsPatch = null;
  return patch;
};

const scheduleContinuousSettingsWrite = (patch: SettingsPatch) => {
  pendingContinuousSettingsPatch = {
    ...(pendingContinuousSettingsPatch ?? {}),
    ...patch,
  };
  cancelPendingContinuousSettingsTimer();
  continuousSettingsWriteTimer = setTimeout(() => {
    const pending = pendingContinuousSettingsPatch;
    continuousSettingsWriteTimer = null;
    pendingContinuousSettingsPatch = null;
    if (pending && !patchIsEmpty(pending)) {
      void enqueueSettingsPatch(pending);
    }
  }, CONTINUOUS_SETTINGS_WRITE_DELAY_MS);
};

/** @internal Test isolation for the module-level persistence coordinator. */
export const resetSettingsPersistenceForTests = () => {
  cancelPendingContinuousSettingsTimer();
  pendingContinuousSettingsPatch = null;
  settingsWriteQueue = [];
  settingsWriteWorkerRunning = false;
  lastPersistedSettings = null;
};

// Persisted SQLite transcript ids are positive. Frontend-only active-session rows use
// a decreasing negative sequence so rapid finalizations cannot collide with each other
// or with records loaded from persistence.
let nextLocalTranscriptId = -1;
const allocateLocalTranscriptId = () => nextLocalTranscriptId--;

interface MooseStoreState {
  characterState: CharacterState;
  conversationLifecycle: ConversationLifecycle;
  mouthShape: MouthShape;
  isBlinking: boolean;
  speechBubbleText: string | null;
  speechBubbleVisible: boolean;
  speechBubbleTimer: NodeJS.Timeout | null;

  inputLevel: number;
  outputLevel: number;
  isMuted: boolean;
  isConversationActive: boolean;

  isSettingsOpen: boolean;
  isOnboardingOpen: boolean;
  isTranscriptOpen: boolean;

  transcripts: TranscriptRecord[];
  partialUserTranscript: string | null;
  partialMooseTranscript: string | null;
  memories: MemoryRecord[];
  inputDevices: AudioDeviceInfo[];
  outputDevices: AudioDeviceInfo[];
  googleTtsVoices: GoogleTtsVoiceDescriptor[];
  settings: AppSettings | null;
  hasApiKey: boolean;

  setCharacterState: (state: CharacterState) => void;
  setMouthShape: (mouth: MouthShape) => void;
  setBlinking: (blinking: boolean) => void;
  showSpeechBubble: (text: string, durationMs?: number) => void;
  hideSpeechBubble: () => void;
  setInputLevel: (level: number) => void;
  setOutputLevel: (level: number) => void;
  saveGoogleApiKey: (apiKey: string) => Promise<void>;
  clearGoogleApiKey: () => Promise<void>;

  toggleSettings: (open?: boolean) => void;
  toggleOnboarding: (open?: boolean) => void;
  toggleTranscript: (open?: boolean) => void;

  startConversation: () => Promise<void>;
  stopConversation: () => Promise<void>;
  bargeIn: () => Promise<void>;
  toggleMute: () => Promise<void>;
  triggerCanned: (type: string) => Promise<void>;

  loadSettings: () => Promise<void>;
  updateSettings: (newSettings: AppSettings) => Promise<void>;
  updateSettingsContinuous: (newSettings: AppSettings) => void;
  loadDevices: () => Promise<void>;
  loadGoogleTtsVoices: () => Promise<void>;
  loadMemories: () => Promise<void>;
  deleteMemory: (id: number) => Promise<void>;
  forgetEverything: () => Promise<void>;
  loadTranscripts: () => Promise<void>;
  sendTextMessage: (message: string) => Promise<string>;

  initEventListeners: () => Promise<() => void>;
}

export const useMooseStore = create<MooseStoreState>((set, get) => ({
  characterState: "idle",
  conversationLifecycle: "idle",
  mouthShape: "closed",
  isBlinking: false,
  speechBubbleText: null,
  speechBubbleVisible: false,
  speechBubbleTimer: null,

  inputLevel: 0,
  outputLevel: 0,
  isMuted: false,
  isConversationActive: false,

  isSettingsOpen: false,
  isOnboardingOpen: false,
  isTranscriptOpen: false,

  transcripts: [],
  partialUserTranscript: null,
  partialMooseTranscript: null,
  memories: [],
  inputDevices: [],
  outputDevices: [],
  googleTtsVoices: [],
  settings: null,
  hasApiKey: false,

  setCharacterState: (characterState) => set({ characterState }),
  setMouthShape: (mouthShape) => set({ mouthShape }),
  setBlinking: (isBlinking) => set({ isBlinking }),

  // Speech bubbles are presentation only. Audio playback and mouth/state changes are
  // authoritative Rust/Tauri responsibilities and arrive through backend events.
  showSpeechBubble: (text, durationMs = 6000) => {
    const currentTimer = get().speechBubbleTimer;
    if (currentTimer) clearTimeout(currentTimer);

    const timer = setTimeout(() => {
      set({
        speechBubbleVisible: false,
        speechBubbleText: null,
        speechBubbleTimer: null,
      });
    }, durationMs);

    set({
      speechBubbleText: text,
      speechBubbleVisible: true,
      speechBubbleTimer: timer,
    });
  },

  hideSpeechBubble: () => {
    const currentTimer = get().speechBubbleTimer;
    if (currentTimer) clearTimeout(currentTimer);
    set({
      speechBubbleVisible: false,
      speechBubbleText: null,
      speechBubbleTimer: null,
    });
  },

  setInputLevel: (inputLevel) => set({ inputLevel }),
  setOutputLevel: (outputLevel) => set({ outputLevel }),

  saveGoogleApiKey: async (apiKey) => {
    await tauriBridge.setGoogleApiKey(apiKey);
    set({ hasApiKey: true });
  },

  clearGoogleApiKey: async () => {
    await tauriBridge.clearGoogleApiKey();
    set({ hasApiKey: false });
  },

  toggleSettings: (open) => {
    const shouldOpen = open !== undefined ? open : !get().isSettingsOpen;
    if (shouldOpen) {
      tauriBridge.resizeWindow(820, 620);
    } else {
      tauriBridge.resizeWindow(340, 450);
    }
    set({ isSettingsOpen: shouldOpen });
  },
  toggleOnboarding: (open) =>
    set((state) => ({
      isOnboardingOpen: open !== undefined ? open : !state.isOnboardingOpen,
    })),
  toggleTranscript: (open) =>
    set((state) => ({
      isTranscriptOpen: open !== undefined ? open : !state.isTranscriptOpen,
    })),

  startConversation: async () => {
    try {
      // Rust owns Connecting/Listening and emits both lifecycle and character state.
      await tauriBridge.startConversation();
    } catch (e) {
      console.error("Failed to start conversation:", e);
    }
  },

  stopConversation: async () => {
    try {
      // Rust owns teardown and the final lifecycle/character state.
      await tauriBridge.stopConversation();
    } catch (e) {
      console.error("Failed to stop conversation:", e);
    }
  },

  bargeIn: async () => {
    try {
      // Mouth and character transitions come from the backend playback/lifecycle path.
      await tauriBridge.bargeIn();
    } catch (e) {
      console.error("Failed to barge in:", e);
    }
  },

  toggleMute: async () => {
    const nextMute = !get().isMuted;
    await tauriBridge.setMute(nextMute);
    set({ isMuted: nextMute });
  },

  triggerCanned: async (type: string) => {
    const text = await tauriBridge.triggerCannedReaction(type);
    if (text) {
      get().showSpeechBubble(text);
    }
  },

  loadSettings: async () => {
    const [
      settings,
      isMuted,
      hasKey,
      onboardingStatus,
      conversationLifecycle,
      characterState,
    ] = await Promise.all([
      tauriBridge.getSettings(),
      tauriBridge.isMuted(),
      tauriBridge.hasGoogleApiKey(),
      tauriBridge.getOnboardingStatus(),
      tauriBridge.getConversationLifecycle(),
      tauriBridge.getCharacterState(),
    ]);
    lastPersistedSettings = cloneSettings(settings);
    set({
      settings,
      isMuted,
      hasApiKey: hasKey,
      conversationLifecycle,
      characterState,
      isConversationActive: lifecycleIsActive(conversationLifecycle),
      isOnboardingOpen: onboardingStatus.needs_acknowledgement,
    });
  },

  updateSettings: async (newSettings) => {
    const current = get().settings;
    ensurePersistedSettingsBaseline(current);
    const discretePatch = current ? settingsPatch(current, newSettings) : {};
    const patch = {
      ...takePendingContinuousSettingsPatch(),
      ...discretePatch,
    };

    // Discrete controls update optimistically too. Persistence is serialized, so
    // an older completion can never overwrite a newer local edit.
    set({ settings: newSettings });
    await enqueueSettingsPatch(patch);
  },

  updateSettingsContinuous: (newSettings) => {
    const current = get().settings;
    ensurePersistedSettingsBaseline(current);
    const patch = current ? settingsPatch(current, newSettings) : {};

    // Continuous controls stay immediate and coalesce only the fields changed
    // during the pointer/input burst. The patch is later rebased onto the last
    // successfully persisted snapshot if an older write fails.
    set({ settings: newSettings });
    if (!patchIsEmpty(patch)) {
      scheduleContinuousSettingsWrite(patch);
    }
  },

  loadDevices: async () => {
    const [inputDevices, outputDevices] = await tauriBridge.listAudioDevices();
    set({ inputDevices, outputDevices });
  },

  loadGoogleTtsVoices: async () => {
    const googleTtsVoices = await tauriBridge.getGoogleTtsVoices();
    set({ googleTtsVoices });
  },

  loadMemories: async () => {
    const memories = await tauriBridge.getMemories();
    set({ memories });
  },

  deleteMemory: async (id) => {
    await tauriBridge.deleteMemory(id);
    set((state) => ({ memories: state.memories.filter((m) => m.id !== id) }));
  },

  forgetEverything: async () => {
    await tauriBridge.forgetEverything();
    set({
      memories: [],
      transcripts: [],
      partialUserTranscript: null,
      partialMooseTranscript: null,
    });
  },

  loadTranscripts: async () => {
    const transcripts = await tauriBridge.getTranscripts();
    set({ transcripts });
  },

  sendTextMessage: async (message: string) => {
    const reply = await tauriBridge.sendTextMessage(message);
    return reply;
  },

  initEventListeners: async () => {
    const unlistenState = await tauriBridge.listenEvent<CharacterState>(
      "moose://state",
      (state) => {
        set({ characterState: state });
      },
    );

    const unlistenLifecycle =
      await tauriBridge.listenEvent<ConversationLifecycle>(
        "moose://conversation/lifecycle",
        (conversationLifecycle) => {
          set({
            conversationLifecycle,
            isConversationActive: lifecycleIsActive(conversationLifecycle),
            ...(conversationLifecycle === "idle" ||
            conversationLifecycle === "failed"
              ? { partialUserTranscript: null, partialMooseTranscript: null }
              : {}),
          });
        },
      );

    const unlistenProviderError = await tauriBridge.listenEvent<ProviderError>(
      "moose://conversation/error",
      (error) => {
        get().showSpeechBubble(error.message, 8000);
      },
    );

    const unlistenMouth = await tauriBridge.listenEvent<MouthShape>(
      "moose://mouth",
      (mouth) => {
        set({ mouthShape: mouth });
      },
    );

    const unlistenBubble = await tauriBridge.listenEvent<string>(
      "moose://speech-bubble",
      (text) => {
        if (text.trim()) {
          get().showSpeechBubble(text);
        } else {
          get().hideSpeechBubble();
        }
      },
    );

    const unlistenUserInput = await tauriBridge.listenEvent<string>(
      "moose://transcript/user",
      (text) => {
        const entry: TranscriptRecord = {
          id: allocateLocalTranscriptId(),
          session_id: "active",
          role: "user",
          text,
          created_at: new Date().toLocaleTimeString(),
        };
        set((s) => ({
          transcripts: [...s.transcripts, entry],
          partialUserTranscript: null,
        }));
      },
    );

    const unlistenUserPartial = await tauriBridge.listenEvent<string>(
      "moose://transcript/user_partial",
      (text) => set({ partialUserTranscript: text || null }),
    );

    const unlistenMooseOutput = await tauriBridge.listenEvent<string>(
      "moose://transcript/moose",
      (text) => {
        const entry: TranscriptRecord = {
          id: allocateLocalTranscriptId(),
          session_id: "active",
          role: "moose",
          text,
          created_at: new Date().toLocaleTimeString(),
        };
        set((s) => ({
          transcripts: [...s.transcripts, entry],
          partialMooseTranscript: null,
        }));
      },
    );

    const unlistenMoosePartial = await tauriBridge.listenEvent<string>(
      "moose://transcript/moose_partial",
      (text) => set({ partialMooseTranscript: text || null }),
    );

    const unlistenInLvl = await tauriBridge.listenEvent<number>(
      "moose://audio/input-level",
      (level) => {
        set({ inputLevel: level });
      },
    );

    const unlistenOutLvl = await tauriBridge.listenEvent<number>(
      "moose://audio/output-level",
      (level) => {
        set({ outputLevel: level });
      },
    );

    const unlistenOpenSettings = await tauriBridge.listenEvent<void>(
      "moose://ui/open-settings",
      () => {
        get().toggleSettings(true);
      },
    );

    const unlistenTrayAction = await tauriBridge.listenEvent<string>(
      "moose://tray/action",
      (action) => {
        if (action === "start_conversation" && !get().isConversationActive) {
          void get().startConversation();
        } else if (
          action === "stop_conversation" &&
          get().isConversationActive
        ) {
          void get().stopConversation();
        } else if (action === "mute" && !get().isMuted) {
          void get().toggleMute();
        } else if (action === "unmute" && get().isMuted) {
          void get().toggleMute();
        }
      },
    );

    return () => {
      unlistenState();
      unlistenLifecycle();
      unlistenProviderError();
      unlistenMouth();
      unlistenBubble();
      unlistenUserInput();
      unlistenUserPartial();
      unlistenMooseOutput();
      unlistenMoosePartial();
      unlistenInLvl();
      unlistenOutLvl();
      unlistenOpenSettings();
      unlistenTrayAction();
    };
  },
}));
