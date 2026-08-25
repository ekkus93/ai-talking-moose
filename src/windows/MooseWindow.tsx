import React, { useEffect } from "react";
import { useMooseStore } from "../stores/mooseStore";
import { MooseController } from "../components/Moose/MooseController";
import { SpeechBubble } from "../components/SpeechBubble/SpeechBubble";
import { TranscriptDrawer } from "../components/Transcript/TranscriptDrawer";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { OnboardingModal } from "../components/Onboarding/OnboardingModal";
import {
  Mic,
  MicOff,
  Volume2,
  VolumeX,
  Sliders,
  Terminal,
  Square,
  Minus,
  Key,
} from "lucide-react";

export const MooseWindow: React.FC = () => {
  const {
    characterState,
    inputLevel,
    outputLevel,
    isMuted,
    isConversationActive,
    hasApiKey,
    isSettingsOpen,
    isOnboardingOpen,
    isTranscriptOpen,
    toggleMute,
    toggleSettings,
    toggleOnboarding,
    toggleTranscript,
    startConversation,
    stopConversation,
    loadSettings,
    initEventListeners,
  } = useMooseStore();

  useEffect(() => {
    loadSettings();
    const unlistenPromise = initEventListeners();
    return () => {
      unlistenPromise.then((unlisten: () => void) => unlisten());
    };
  }, [loadSettings, initEventListeners]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target;
      const isEditing =
        target instanceof Element &&
        target.matches("input, textarea, select, [contenteditable='true']");

      if (event.key === "Escape") {
        if (isOnboardingOpen) {
          toggleOnboarding(false);
        } else if (isSettingsOpen) {
          toggleSettings(false);
        } else if (isTranscriptOpen) {
          toggleTranscript(false);
        } else {
          return;
        }
        event.preventDefault();
        return;
      }

      if (isEditing || isOnboardingOpen) return;
      const command = event.ctrlKey || event.metaKey;
      if (!command) return;

      if (event.key === ",") {
        event.preventDefault();
        toggleSettings(true);
        return;
      }

      if (event.shiftKey && event.key.toLowerCase() === "m") {
        event.preventDefault();
        void toggleMute();
        return;
      }

      if (event.key === "Enter" && !isSettingsOpen && !isTranscriptOpen) {
        event.preventDefault();
        if (isConversationActive) {
          void stopConversation();
        } else {
          void startConversation();
        }
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    isConversationActive,
    isOnboardingOpen,
    isSettingsOpen,
    isTranscriptOpen,
    startConversation,
    stopConversation,
    toggleMute,
    toggleOnboarding,
    toggleSettings,
    toggleTranscript,
  ]);

  // State badge styling and label
  const getStateBadge = () => {
    switch (characterState) {
      case "listening":
        return {
          label: "LISTENING...",
          color: "bg-green-700 text-white animate-pulse",
        };
      case "thinking":
        return {
          label: "THINKING...",
          color: "bg-amber-700 text-white animate-pulse",
        };
      case "talking":
        return { label: "TALKING", color: "bg-blue-600 text-white" };
      case "interrupted":
        return { label: "INTERRUPTED", color: "bg-purple-600 text-white" };
      case "muted":
        return { label: "MUTED", color: "bg-gray-600 text-white" };
      case "annoyed":
        return { label: "ANNOYED", color: "bg-orange-700 text-white" };
      case "sleeping":
        return { label: "SLEEPING", color: "bg-indigo-600 text-white" };
      case "error":
        return { label: "ERROR", color: "bg-red-600 text-white" };
      case "idle":
      default:
        return { label: "IDLE", color: "bg-black text-white" };
    }
  };

  const badge = getStateBadge();

  const handleStartDrag = async (e: React.MouseEvent) => {
    if (
      e.button === 0 &&
      !(e.target as HTMLElement).closest("button, input, select")
    ) {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().startDragging();
      } catch (err) {
        console.error("Window drag error:", err);
      }
    }
  };

  return (
    <div
      data-testid="moose-window"
      className="w-full h-screen bg-[#dcd6cd] flex flex-col border-2 border-black select-none overflow-hidden font-mono shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] relative"
    >
      {/* Retro Title Bar with Horizontal Pinstripes & Drag Region */}
      <div
        data-tauri-drag-region
        onMouseDown={handleStartDrag}
        className="h-7 bg-[#ded9cf] border-b-2 border-black flex items-center justify-between px-2 cursor-grab active:cursor-grabbing relative overflow-hidden"
      >
        {/* Decorative background horizontal stripes */}
        <div className="absolute inset-0 opacity-20 pointer-events-none bg-[repeating-linear-gradient(0deg,#000,#000_1px,transparent_1px,transparent_3px)]" />

        {/* Close box */}
        <div className="flex items-center gap-1.5 z-10">
          <button
            onClick={() => toggleMute()}
            className="w-3.5 h-3.5 bg-white border border-black rounded-[2px] shadow-[1px_1px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5 flex items-center justify-center"
            title={isMuted ? "Unmute Moose" : "Mute Moose"}
            aria-label={isMuted ? "Unmute Moose" : "Mute Moose"}
          >
            <Square className="w-2 h-2 fill-current" />
          </button>
        </div>

        {/* Centered Window Title */}
        <span
          data-tauri-drag-region
          className="z-10 font-bold text-xs tracking-wider bg-[#ded9cf] px-2 border-x border-black/30"
        >
          The Talking Moose
        </span>

        {/* Collapse / Minimize */}
        <div className="flex items-center gap-1.5 z-10">
          <button
            onClick={() => toggleSettings(true)}
            className="w-3.5 h-3.5 bg-white border border-black rounded-[2px] shadow-[1px_1px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5 flex items-center justify-center"
            title="Settings"
            aria-label="Open Settings"
          >
            <Minus className="w-2.5 h-2.5" />
          </button>
        </div>
      </div>

      {/* Main Character Display Area */}
      <div className="flex-1 flex flex-col justify-between p-2 bg-[#ece7de] relative overflow-hidden">
        {/* Status Badge, API Key Status, & Audio VU Meters */}
        <div className="flex items-center justify-between z-10 gap-1.5 flex-wrap">
          <div className="flex items-center gap-1.5">
            <span
              role="status"
              aria-live="polite"
              className={`text-[10px] font-bold px-2 py-0.5 rounded border border-black shadow-[1px_1px_0px_0px_rgba(0,0,0,1)] ${badge.color}`}
            >
              {badge.label}
            </span>

            {/* Google API Key Status Indicator */}
            <button
              onClick={() => toggleSettings(true)}
              className={`text-[9px] font-bold px-1.5 py-0.5 rounded border flex items-center gap-1 shadow-[1px_1px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5 ${
                hasApiKey
                  ? "bg-green-100 text-green-900 border-green-800"
                  : "bg-amber-100 text-amber-900 border-amber-800"
              }`}
              aria-label={
                hasApiKey
                  ? "Gemini API key configured; open Settings"
                  : "Gemini API key missing; open Settings"
              }
              title={
                hasApiKey
                  ? "Google Gemini API Key Active (Click to Configure)"
                  : "No Google Gemini API Key - Click to Setup"
              }
            >
              <Key className="w-2.5 h-2.5" />
              <span>{hasApiKey ? "AI: READY" : "AI: NO KEY"}</span>
            </button>
          </div>

          {/* Real-Time Audio Level VU Meters */}
          <div className="flex items-center gap-2 bg-white px-2 py-0.5 border border-black rounded shadow-[1px_1px_0px_0px_rgba(0,0,0,1)]">
            <div className="flex items-center gap-1 text-[9px] font-bold">
              <Mic className="w-2.5 h-2.5" />
              <div
                role="meter"
                aria-label="Microphone input level"
                aria-valuemin={0}
                aria-valuemax={1}
                aria-valuenow={Math.min(1, Math.max(0, inputLevel))}
                className="w-8 h-1.5 bg-gray-200 border border-gray-400 rounded-sm overflow-hidden"
              >
                <div
                  className="h-full bg-green-500 transition-all duration-75"
                  style={{
                    width: `${Math.min(100, Math.max(0, inputLevel * 300))}%`,
                  }}
                />
              </div>
            </div>

            <div className="flex items-center gap-1 text-[9px] font-bold">
              <Volume2 className="w-2.5 h-2.5" />
              <div
                role="meter"
                aria-label="Audio output level"
                aria-valuemin={0}
                aria-valuemax={1}
                aria-valuenow={Math.min(1, Math.max(0, outputLevel))}
                className="w-8 h-1.5 bg-gray-200 border border-gray-400 rounded-sm overflow-hidden"
              >
                <div
                  className="h-full bg-blue-500 transition-all duration-75"
                  style={{
                    width: `${Math.min(100, Math.max(0, outputLevel * 300))}%`,
                  }}
                />
              </div>
            </div>
          </div>
        </div>

        {/* Speech Bubble overlay */}
        <SpeechBubble />

        {/* Character Moose Sprite - Dynamically scales with window */}
        <div className="flex-1 min-h-0 w-full flex items-center justify-center p-1 overflow-hidden">
          <MooseController />
        </div>

        {/* Bottom Control Bar */}
        <div className="flex items-center justify-between gap-1.5 pt-1 border-t border-black/20 z-10">
          {/* Talk / Stop Conversation Button */}
          <button
            onClick={() => {
              if (isConversationActive) {
                stopConversation();
              } else {
                startConversation();
              }
            }}
            aria-label={
              isConversationActive ? "Stop conversation" : "Start conversation"
            }
            className={`flex-1 py-1.5 px-2 rounded border-2 border-black font-bold text-xs flex items-center justify-center gap-1.5 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5 transition-colors ${
              isConversationActive
                ? "bg-red-600 text-white hover:bg-red-700"
                : "bg-white hover:bg-gray-100 text-black"
            }`}
          >
            {isConversationActive ? (
              <>
                <MicOff className="w-3.5 h-3.5" />
                <span>Stop</span>
              </>
            ) : (
              <>
                <Mic className="w-3.5 h-3.5" />
                <span>Talk</span>
              </>
            )}
          </button>

          {/* Debug Terminal Button */}
          <button
            onClick={() => toggleTranscript()}
            className="p-1.5 bg-white border-2 border-black rounded shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5 hover:bg-gray-100 flex items-center gap-1"
            title="Open Debug Terminal / Type Message"
            aria-label="Open transcript and text terminal"
          >
            <Terminal className="w-3.5 h-3.5 text-black" />
          </button>

          {/* Mute Toggle Button */}
          <button
            onClick={() => toggleMute()}
            className={`p-1.5 border-2 border-black rounded shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5 ${
              isMuted ? "bg-amber-400 text-black" : "bg-white hover:bg-gray-100"
            }`}
            title={isMuted ? "Unmute Moose" : "Mute Moose"}
            aria-label={isMuted ? "Unmute Moose" : "Mute Moose"}
          >
            {isMuted ? (
              <VolumeX className="w-3.5 h-3.5" />
            ) : (
              <Volume2 className="w-3.5 h-3.5" />
            )}
          </button>

          {/* Settings Button */}
          <button
            onClick={() => toggleSettings(true)}
            className="p-1.5 bg-white border-2 border-black rounded shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5 hover:bg-gray-100"
            title="Open Settings"
            aria-label="Open Settings"
          >
            <Sliders className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Slide-in Drawers and Modals */}
      <TranscriptDrawer />
      <SettingsModal />
      <OnboardingModal />
    </div>
  );
};
