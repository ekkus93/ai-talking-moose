import { create } from "zustand";
import {
  AppSettings,
  AudioDeviceInfo,
  CharacterState,
  MemoryRecord,
  MouthShape,
  TranscriptRecord,
} from "../types/moose";
import { tauriBridge } from "../lib/tauriBridge";

interface MooseStoreState {
  characterState: CharacterState;
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
  memories: MemoryRecord[];
  inputDevices: AudioDeviceInfo[];
  outputDevices: AudioDeviceInfo[];
  settings: AppSettings | null;
  hasApiKey: boolean;

  // Actions
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
  memories: [],
  inputDevices: [],
  outputDevices: [],
  settings: null,
  hasApiKey: false,

  setCharacterState: (characterState) => set({ characterState }),
  setMouthShape: (mouthShape) => set({ mouthShape }),
  setBlinking: (isBlinking) => set({ isBlinking }),

  showSpeechBubble: (text, durationMs = 6000) => {
    const currentTimer = get().speechBubbleTimer;
    if (currentTimer) clearTimeout(currentTimer);

    // Speak aloud with SpeechSynthesis if not muted
    if (
      typeof window !== "undefined" &&
      "speechSynthesis" in window &&
      !get().isMuted
    ) {
      window.speechSynthesis.cancel();
      const utterance = new SpeechSynthesisUtterance(text);
      utterance.pitch = 0.78;
      utterance.rate = 0.92;
      utterance.onstart = () => {
        set({ characterState: "talking", mouthShape: "medium" });
      };
      utterance.onboundary = (e) => {
        if (e.name === "word") {
          const shapes: MouthShape[] = ["small", "medium", "wide"];
          const randomShape = shapes[Math.floor(Math.random() * shapes.length)];
          set({ mouthShape: randomShape });
        }
      };
      utterance.onend = () => {
        set({ characterState: "idle", mouthShape: "closed" });
      };
      window.speechSynthesis.speak(utterance);
    }

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
    if (typeof window !== "undefined" && "speechSynthesis" in window) {
      window.speechSynthesis.cancel();
    }
    set({
      speechBubbleVisible: false,
      speechBubbleText: null,
      speechBubbleTimer: null,
      mouthShape: "closed",
      characterState: "idle",
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
      set({ isConversationActive: true, characterState: "listening" });
      await tauriBridge.startConversation();
    } catch (e) {
      console.error("Failed to start conversation:", e);
      set({ isConversationActive: false, characterState: "error" });
    }
  },

  stopConversation: async () => {
    try {
      await tauriBridge.stopConversation();
      set({
        isConversationActive: false,
        characterState: "idle",
        inputLevel: 0,
        outputLevel: 0,
      });
    } catch (e) {
      console.error("Failed to stop conversation:", e);
    }
  },

  bargeIn: async () => {
    try {
      if (typeof window !== "undefined" && "speechSynthesis" in window) {
        window.speechSynthesis.cancel();
      }
      await tauriBridge.bargeIn();
      set({ mouthShape: "closed", characterState: "interrupted" });
    } catch (e) {
      console.error("Failed to barge in:", e);
    }
  },

  toggleMute: async () => {
    const nextMute = !get().isMuted;
    if (
      nextMute &&
      typeof window !== "undefined" &&
      "speechSynthesis" in window
    ) {
      window.speechSynthesis.cancel();
    }
    await tauriBridge.setMute(nextMute);
    set({ isMuted: nextMute, characterState: nextMute ? "muted" : "idle" });
  },

  triggerCanned: async (type: string) => {
    const text = await tauriBridge.triggerCannedReaction(type);
    if (text) {
      get().showSpeechBubble(text);
    }
  },

  triggerAmbient: async (summary: string) => {
    const text = await tauriBridge.triggerAmbientRemark(summary);
    if (text) {
      get().showSpeechBubble(text);
    }
  },

  loadSettings: async () => {
    const settings = await tauriBridge.getSettings();
    const isMuted = await tauriBridge.isMuted();
    const hasKey = await tauriBridge.hasGoogleApiKey();
    set({ settings, isMuted, hasApiKey: hasKey });

    // Open onboarding if no API key is configured yet
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
    set({ memories: [], transcripts: [] });
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

    const unlistenMouth = await tauriBridge.listenEvent<MouthShape>(
      "moose://mouth",
      (mouth) => {
        set({ mouthShape: mouth });
      },
    );

    const unlistenBubble = await tauriBridge.listenEvent<string>(
      "moose://speech-bubble",
      (text) => {
        get().showSpeechBubble(text);
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
        set((s) => ({ transcripts: [...s.transcripts, entry] }));
      },
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
        set((s) => ({ transcripts: [...s.transcripts, entry] }));
      },
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
      unlistenMouth();
      unlistenBubble();
      unlistenUserInput();
      unlistenMooseOutput();
      unlistenInLvl();
      unlistenOutLvl();
    };
  },
}));
