import { create } from "zustand";
import {
  AppSettings,
  AudioDeviceInfo,
  CharacterState,
  ConversationLifecycle,
  MemoryRecord,
  MouthShape,
  ProviderError,
  TranscriptRecord,
} from "../types/moose";
import { tauriBridge } from "../lib/tauriBridge";

const lifecycleIsActive = (lifecycle: ConversationLifecycle) =>
  lifecycle !== "idle" && lifecycle !== "failed";

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
  settings: AppSettings | null;
  hasApiKey: boolean;

  setCharacterState: (state: CharacterState) => void;
  setMouthShape: (mouth: MouthShape) => void;
  setBlinking: (blinking: boolean) => void;
  showSpeechBubble: (text: string, durationMs?: number) => void;
  hideSpeechBubble: () => void;
  setInputLevel: (level: number) => void;
  setOutputLevel: (level: number) => void;

  toggleSettings: (open?: boolean) => void;
  toggleOnboarding: (open?: boolean) => void;
  toggleTranscript: (open?: boolean) => void;

  startConversation: () => Promise<void>;
  stopConversation: () => Promise<void>;
  bargeIn: () => Promise<void>;
  toggleMute: () => Promise<void>;
  triggerCanned: (type: string) => Promise<void>;
  triggerAmbient: (summary: string) => Promise<void>;

  loadSettings: () => Promise<void>;
  updateSettings: (newSettings: AppSettings) => Promise<void>;
  loadDevices: () => Promise<void>;
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

  triggerAmbient: async (summary: string) => {
    // Ambient speech-bubble visibility is backend-event driven so the Rust lifecycle can
    // clear it before hiding Moose without this promise re-showing stale text afterward.
    await tauriBridge.triggerAmbientRemark(summary);
  },

  loadSettings: async () => {
    const [settings, isMuted, hasKey, conversationLifecycle, characterState] =
      await Promise.all([
        tauriBridge.getSettings(),
        tauriBridge.isMuted(),
        tauriBridge.hasGoogleApiKey(),
        tauriBridge.getConversationLifecycle(),
        tauriBridge.getCharacterState(),
      ]);
    set({
      settings,
      isMuted,
      hasApiKey: hasKey,
      conversationLifecycle,
      characterState,
      isConversationActive: lifecycleIsActive(conversationLifecycle),
    });

    if (!hasKey) {
      set({ isOnboardingOpen: true });
    }
  },

  updateSettings: async (newSettings) => {
    await tauriBridge.updateSettings(newSettings);
    set({ settings: newSettings });
  },

  loadDevices: async () => {
    const [inputDevices, outputDevices] = await tauriBridge.listAudioDevices();
    set({ inputDevices, outputDevices });
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
          id: Date.now(),
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
          id: Date.now(),
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
    };
  },
}));
