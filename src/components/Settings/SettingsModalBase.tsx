import React, { useState, useEffect } from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { tauriBridge } from "../../lib/tauriBridge";
import type { GoogleModelDescriptor } from "../../types/moose";
import { GeneralTab } from "./GeneralTab";
import { BehaviorTab } from "./BehaviorTab";
import { VoiceTab } from "./VoiceTab";
import { AsrSettingsPanel } from "./AsrSettingsPanel";
import { PersonalityTab } from "./PersonalityTab";
import { AiTab } from "./AiTab";
import { PrivacyTab } from "./PrivacyTab";
import { DataTab } from "./DataTab";
import { DiagnosticsTab } from "./DiagnosticsTab";
import {
  Sliders,
  Volume2,
  Mic,
  Brain,
  Shield,
  Key,
  Database,
  Activity,
  X,
  Sparkles,
} from "lucide-react";

export const SettingsModal: React.FC = () => {
  const {
    isSettingsOpen,
    toggleSettings,
    settings,
    loadMemories,
    loadDevices,
  } = useMooseStore();

  const [activeTab, setActiveTab] = useState<
    | "general"
    | "behavior"
    | "voice"
    | "speech"
    | "personality"
    | "ai"
    | "privacy"
    | "data"
    | "diagnostics"
  >("general");

  const [googleModels, setGoogleModels] = useState<GoogleModelDescriptor[]>([]);

  useEffect(() => {
    if (isSettingsOpen) {
      loadDevices();
      loadMemories();
      void tauriBridge
        .getGoogleModels()
        .then((models) => setGoogleModels(models ?? []));
    }
  }, [isSettingsOpen, loadDevices, loadMemories]);

  if (!isSettingsOpen || !settings) {
    return null;
  }

  return (
    <div
      data-testid="settings-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
      className="absolute inset-0 z-50 bg-[#ece7de] flex flex-col font-mono text-xs overflow-hidden select-none animate-in fade-in duration-100"
    >
      {/* Title bar */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-between px-3 py-2 bg-[#dcd6cd] border-b-2 border-black select-none cursor-grab active:cursor-grabbing"
      >
        <div className="flex items-center gap-2 font-bold text-sm">
          <Sliders className="w-4 h-4" />
          <span id="settings-title">TALKING MOOSE CONTROL PANEL</span>
        </div>
        <button
          onClick={() => toggleSettings(false)}
          className="hover:bg-gray-300 p-1 rounded border border-black shadow-[1px_1px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5"
          title="Close Settings"
          aria-label="Close settings"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar Tabs */}
        <div
          role="tablist"
          aria-label="Settings sections"
          className="w-44 bg-[#ded9cf] border-r-2 border-black flex flex-col py-2 gap-0.5 select-none overflow-y-auto"
        >
          {[
            { id: "general", label: "General", icon: Sliders },
            { id: "behavior", label: "Behavior", icon: Sparkles },
            { id: "voice", label: "Voice & Audio", icon: Volume2 },
            { id: "speech", label: "Speech Recognition", icon: Mic },
            { id: "personality", label: "Personality", icon: Brain },
            { id: "ai", label: "Gemini AI", icon: Key },
            { id: "privacy", label: "Privacy", icon: Shield },
            { id: "data", label: "Memory & Data", icon: Database },
            { id: "diagnostics", label: "Diagnostics", icon: Activity },
          ].map((tab) => {
            const Icon = tab.icon;
            const isCurrent = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                role="tab"
                aria-selected={isCurrent}
                onClick={() => setActiveTab(tab.id as typeof activeTab)}
                className={`flex items-center gap-2 px-3 py-2 text-left font-bold transition-colors ${
                  isCurrent
                    ? "bg-black text-white"
                    : "hover:bg-gray-300 text-gray-800"
                }`}
              >
                <Icon className="w-3.5 h-3.5" />
                <span>{tab.label}</span>
              </button>
            );
          })}
        </div>

        {/* Main Content Area */}
        <div className="flex-1 p-4 overflow-y-auto bg-white">
          {activeTab === "general" && <GeneralTab />}
          {activeTab === "behavior" && <BehaviorTab />}
          {activeTab === "voice" && <VoiceTab />}
          {activeTab === "speech" && <AsrSettingsPanel />}
          {activeTab === "personality" && <PersonalityTab />}
          {activeTab === "ai" && <AiTab googleModels={googleModels} />}
          {activeTab === "privacy" && <PrivacyTab />}
          {activeTab === "data" && <DataTab />}
          {activeTab === "diagnostics" && <DiagnosticsTab />}
        </div>
      </div>

      {/* Footer */}
      <div className="px-4 py-2 bg-[#ded9cf] border-t-2 border-black flex justify-between items-center select-none">
        <span className="text-[11px] text-gray-600">
          The Talking Moose AI • Version 0.1.0
        </span>
        <button
          onClick={() => toggleSettings(false)}
          className="px-4 py-1.5 bg-black text-white rounded font-bold hover:bg-gray-800 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5"
        >
          Done
        </button>
      </div>
    </div>
  );
};
