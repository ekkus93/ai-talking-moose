import React, { useState, useEffect } from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { tauriBridge } from "../../lib/tauriBridge";
import { AudioDiagnosticsPanel } from "./AudioDiagnosticsPanel";
import { MicrophonePermissionCard } from "./MicrophonePermissionCard";
import {
  Sliders,
  Volume2,
  Brain,
  Shield,
  Key,
  Database,
  Activity,
  X,
  Trash2,
  CheckCircle,
  AlertCircle,
  Sparkles,
} from "lucide-react";

export const SettingsModal: React.FC = () => {
  const {
    isSettingsOpen,
    toggleSettings,
    settings,
    updateSettings,
    memories,
    loadMemories,
    deleteMemory,
    forgetEverything,
    inputDevices,
    outputDevices,
    loadDevices,
    triggerCanned,
  } = useMooseStore();

  const [activeTab, setActiveTab] = useState<
    | "general"
    | "behavior"
    | "voice"
    | "personality"
    | "ai"
    | "privacy"
    | "data"
    | "diagnostics"
  >("general");

  const [apiKeyInput, setApiKeyInput] = useState("");
  const [testResult, setTestResult] = useState<{
    success: boolean;
    message: string;
  } | null>(null);
  const [isTesting, setIsTesting] = useState(false);
  const [isAuditioning, setIsAuditioning] = useState(false);

  useEffect(() => {
    if (isSettingsOpen) {
      loadDevices();
      loadMemories();
    }
  }, [isSettingsOpen, loadDevices, loadMemories]);

  if (!isSettingsOpen || !settings) {
    return null;
  }

  const handleSaveApiKey = async () => {
    if (apiKeyInput.trim()) {
      await tauriBridge.setGoogleApiKey(apiKeyInput.trim());
      setApiKeyInput("");
      setTestResult({
        success: true,
        message: "API Key updated successfully!",
      });
    }
  };

  const handleTestConnection = async () => {
    setIsTesting(true);
    setTestResult(null);
    try {
      const res = await tauriBridge.testAiConnection();
      setTestResult(res);
    } catch (e) {
      setTestResult({ success: false, message: String(e) });
    } finally {
      setIsTesting(false);
    }
  };

  return (
    <div
      data-testid="settings-modal"
      className="absolute inset-0 z-50 bg-[#ece7de] flex flex-col font-mono text-xs overflow-hidden select-none animate-in fade-in duration-100"
    >
      {/* Title bar */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-between px-3 py-2 bg-[#dcd6cd] border-b-2 border-black select-none cursor-grab active:cursor-grabbing"
      >
        <div className="flex items-center gap-2 font-bold text-sm">
          <Sliders className="w-4 h-4" />
          <span>TALKING MOOSE CONTROL PANEL</span>
        </div>
        <button
          onClick={() => toggleSettings(false)}
          className="hover:bg-gray-300 p-1 rounded border border-black shadow-[1px_1px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5"
          title="Close Settings"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar Tabs */}
        <div className="w-44 bg-[#ded9cf] border-r-2 border-black flex flex-col py-2 gap-0.5 select-none overflow-y-auto">
          {[
            { id: "general", label: "General", icon: Sliders },
            { id: "behavior", label: "Behavior", icon: Sparkles },
            { id: "voice", label: "Voice & Audio", icon: Volume2 },
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
          {/* General Tab */}
          {activeTab === "general" && (
            <div className="space-y-4">
              <h3 className="font-bold text-sm border-b border-black pb-1">
                Application Preferences
              </h3>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={settings.launch_at_login}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      launch_at_login: e.target.checked,
                    })
                  }
                  className="accent-black"
                />
                <span>Launch Talking Moose at system login</span>
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={settings.always_on_top}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      always_on_top: e.target.checked,
                    })
                  }
                  className="accent-black"
                />
                <span>Keep Moose window always on top</span>
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={settings.restore_position}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      restore_position: e.target.checked,
                    })
                  }
                  className="accent-black"
                />
                <span>Restore desktop window position across restarts</span>
              </label>
            </div>
          )}

          {/* Behavior Tab */}
          {activeTab === "behavior" && (
            <div className="space-y-4">
              <h3 className="font-bold text-sm border-b border-black pb-1">
                Ambient Behavior & Timing
              </h3>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={settings.unsolicited_comments}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      unsolicited_comments: e.target.checked,
                    })
                  }
                  className="accent-black"
                />
                <span className="font-bold">
                  Enable unsolicited ambient remarks
                </span>
              </label>

              <div className="space-y-1">
                <div className="flex justify-between">
                  <span>Talkativeness</span>
                  <span className="font-bold">
                    {(settings.talkativeness * 100).toFixed(0)}%
                  </span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={settings.talkativeness}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      talkativeness: parseFloat(e.target.value),
                    })
                  }
                  className="w-full accent-black"
                />
              </div>

              <div className="space-y-1">
                <div className="flex justify-between">
                  <span>Maximum Ambient Comments Per Hour</span>
                  <span className="font-bold">
                    {settings.max_comments_per_hour}
                  </span>
                </div>
                <input
                  type="range"
                  min="1"
                  max="12"
                  step="1"
                  value={settings.max_comments_per_hour}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      max_comments_per_hour: parseInt(e.target.value),
                    })
                  }
                  className="w-full accent-black"
                />
              </div>

              <div className="border border-black p-3 rounded bg-[#fbf9f5] space-y-2">
                <label className="flex items-center gap-2 font-bold">
                  <input
                    type="checkbox"
                    checked={settings.quiet_hours_enabled}
                    onChange={(e) =>
                      updateSettings({
                        ...settings,
                        quiet_hours_enabled: e.target.checked,
                      })
                    }
                    className="accent-black"
                  />
                  <span>Quiet Hours</span>
                </label>
                <p className="text-gray-600 text-[11px]">
                  Silence all unsolicited remarks between{" "}
                  {settings.quiet_hours_start}:00 and {settings.quiet_hours_end}
                  :00.
                </p>
              </div>
            </div>
          )}

          {/* Voice & Audio Tab */}
          {activeTab === "voice" && (
            <div className="space-y-4">
              <h3 className="font-bold text-sm border-b border-black pb-1">
                Audio & Speech Synthesis
              </h3>

              <div>
                <label className="block mb-1 font-bold">
                  Microphone Input Device
                </label>
                <select
                  value={settings.input_device || ""}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      input_device: e.target.value || null,
                    })
                  }
                  className="w-full p-1.5 border border-black rounded bg-white"
                >
                  <option value="">Default Microphone</option>
                  {inputDevices.map((d) => (
                    <option key={String(d.id)} value={String(d.id)}>
                      {d.name}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block mb-1 font-bold">
                  Audio Output Device
                </label>
                <select
                  value={settings.output_device || ""}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      output_device: e.target.value || null,
                    })
                  }
                  className="w-full p-1.5 border border-black rounded bg-white"
                >
                  <option value="">Default Speakers</option>
                  {outputDevices.map((d) => (
                    <option key={String(d.id)} value={String(d.id)}>
                      {d.name}
                    </option>
                  ))}
                </select>
              </div>

              <div className="space-y-2">
                <div>
                  <label className="block mb-1 font-bold">
                    Moose Voice Preset
                  </label>
                  <select
                    value={settings.tts_voice}
                    onChange={(e) =>
                      updateSettings({ ...settings, tts_voice: e.target.value })
                    }
                    className="w-full p-1.5 border border-black rounded bg-white font-bold"
                  >
                    <option value="Fenrir">
                      Fenrir (Deeper, Gravelly Cartoon Moose - Recommended)
                    </option>
                    <option value="Charon">
                      Charon (Deep, Slow & Deadpan)
                    </option>
                    <option value="Orus">
                      Orus (Warm, Low-Pitched & Relaxed)
                    </option>
                    <option value="Kore">Kore (Smooth & Natural)</option>
                    <option value="Puck">Puck (High-Pitched & Squeaky)</option>
                    <option value="Aoede">Aoede (Higher Register)</option>
                  </select>
                </div>

                <button
                  onClick={async () => {
                    setIsAuditioning(true);
                    try {
                      await tauriBridge.auditionVoice(settings.tts_voice);
                    } finally {
                      setTimeout(() => setIsAuditioning(false), 2500);
                    }
                  }}
                  disabled={isAuditioning}
                  className="px-3 py-1.5 bg-white border-2 border-black rounded font-bold hover:bg-gray-100 flex items-center gap-1.5 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5"
                >
                  <Volume2 className="w-3.5 h-3.5" />
                  <span>
                    {isAuditioning
                      ? "Playing Sample..."
                      : `Audition "${settings.tts_voice}" Voice Sample`}
                  </span>
                </button>
              </div>
            </div>
          )}

          {/* Personality Tab */}
          {activeTab === "personality" && (
            <div className="space-y-3">
              <h3 className="font-bold text-sm border-b border-black pb-1">
                Moose Character Persona
              </h3>

              {[
                { key: "dry", label: "Dry Wit & Deadpan" },
                { key: "sarcastic", label: "Sarcasm & Snark" },
                { key: "friendly", label: "Warmth & Friendliness" },
                { key: "absurd", label: "Absurdity & Goofiness" },
                {
                  key: "helpful",
                  label: "Helpfulness (Keep low for classic Moose!)",
                },
                { key: "verbosity", label: "Verbosity (Response length)" },
              ].map((slider) => (
                <div key={slider.key} className="space-y-1">
                  <div className="flex justify-between text-[11px]">
                    <span>{slider.label}</span>
                    <span className="font-bold">
                      {(
                        (settings[
                          slider.key as keyof typeof settings
                        ] as number) * 100
                      ).toFixed(0)}
                      %
                    </span>
                  </div>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={
                      settings[slider.key as keyof typeof settings] as number
                    }
                    onChange={(e) =>
                      updateSettings({
                        ...settings,
                        [slider.key]: parseFloat(e.target.value),
                      })
                    }
                    className="w-full accent-black"
                  />
                </div>
              ))}
            </div>
          )}

          {/* Gemini AI Tab */}
          {activeTab === "ai" && (
            <div className="space-y-4">
              <h3 className="font-bold text-sm border-b border-black pb-1">
                Google Gemini AI Configuration
              </h3>

              <div className="space-y-2">
                <label className="block font-bold">
                  Google Gemini API Key (BYOK)
                </label>
                <div className="flex gap-2">
                  <input
                    type="password"
                    placeholder="Enter AIzaSy... API Key"
                    value={apiKeyInput}
                    onChange={(e) => setApiKeyInput(e.target.value)}
                    className="flex-1 p-1.5 border border-black rounded"
                  />
                  <button
                    onClick={handleSaveApiKey}
                    className="px-3 py-1.5 bg-black text-white rounded font-bold hover:bg-gray-800"
                  >
                    Save Key
                  </button>
                </div>
                <p className="text-[10px] text-gray-500">
                  Your key is stored in the operating system secure credential
                  store and is never returned to this settings screen.
                </p>
              </div>

              <div className="pt-2 border-t border-gray-200">
                <div className="space-y-3 mb-3">
                  <div>
                    <label className="block mb-1 font-bold">
                      Realtime Live Voice Model
                    </label>
                    <select
                      value={settings.live_model}
                      onChange={(e) =>
                        updateSettings({
                          ...settings,
                          live_model: e.target.value,
                        })
                      }
                      className="w-full p-1.5 border border-black rounded bg-white font-bold"
                    >
                      <option value="gemini-2.5-flash-native-audio-latest">
                        gemini-2.5-flash-native-audio-latest (Recommended Live
                        Voice)
                      </option>
                      <option value="gemini-3.1-flash-live-preview">
                        gemini-3.1-flash-live-preview (Gemini 3 Live Preview)
                      </option>
                      <option value="gemini-flash-latest">
                        gemini-flash-latest
                      </option>
                      <option value="gemini-2.5-flash">gemini-2.5-flash</option>
                      <option value="gemini-2.5-pro">gemini-2.5-pro</option>
                      <option value="gemini-3.7-flash">gemini-3.7-flash</option>
                    </select>
                  </div>

                  <div>
                    <label className="block mb-1 font-bold">
                      Text & Ambient Remark Model
                    </label>
                    <select
                      value={settings.text_model}
                      onChange={(e) =>
                        updateSettings({
                          ...settings,
                          text_model: e.target.value,
                        })
                      }
                      className="w-full p-1.5 border border-black rounded bg-white font-bold"
                    >
                      <option value="gemini-2.5-flash">
                        gemini-2.5-flash (Fast & Responsive)
                      </option>
                      <option value="gemini-flash-latest">
                        gemini-flash-latest
                      </option>
                      <option value="gemini-2.5-pro">
                        gemini-2.5-pro (High Intelligence)
                      </option>
                      <option value="gemini-3.7-flash">
                        gemini-3.7-flash (Gemini 3.7)
                      </option>
                    </select>
                  </div>
                </div>

                <button
                  onClick={handleTestConnection}
                  disabled={isTesting}
                  className="px-3 py-1.5 bg-white border-2 border-black rounded font-bold hover:bg-gray-100 flex items-center gap-1.5"
                >
                  <span>
                    {isTesting
                      ? "Testing Connection..."
                      : "Test Gemini Connection"}
                  </span>
                </button>

                {testResult && (
                  <div
                    className={`mt-2 p-2 rounded border flex items-center gap-2 ${
                      testResult.success
                        ? "bg-green-50 border-green-600 text-green-800"
                        : "bg-red-50 border-red-600 text-red-800"
                    }`}
                  >
                    {testResult.success ? (
                      <CheckCircle className="w-4 h-4 text-green-600 flex-shrink-0" />
                    ) : (
                      <AlertCircle className="w-4 h-4 text-red-600 flex-shrink-0" />
                    )}
                    <span>{testResult.message}</span>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Privacy Tab */}
          {activeTab === "privacy" && (
            <div className="space-y-4">
              <h3 className="font-bold text-sm border-b border-black pb-1">
                Privacy & Permissions
              </h3>

              <MicrophonePermissionCard />

              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={settings.active_app_observation}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      active_app_observation: e.target.checked,
                    })
                  }
                  className="accent-black"
                />
                <span>Allow Moose to observe active application switches</span>
              </label>

              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={settings.memory_enabled}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      memory_enabled: e.target.checked,
                    })
                  }
                  className="accent-black"
                />
                <span>Enable local memory across conversations</span>
              </label>

              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={settings.save_transcripts}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      save_transcripts: e.target.checked,
                    })
                  }
                  className="accent-black"
                />
                <span>Save local conversation transcripts</span>
              </label>

              <div className="p-3 bg-gray-50 border border-gray-300 rounded text-[11px] text-gray-700 space-y-1">
                <p className="font-bold">Microphone privacy:</p>
                <p>
                  - Moonshine local ASR keeps microphone audio on this computer.
                </p>
                <p>
                  - Gemini Live cloud ASR sends microphone audio to Google only
                  during an active conversation.
                </p>
                <p>
                  - Screen contents, OCR, keystrokes, and files are never
                  collected.
                </p>
              </div>
            </div>
          )}

          {/* Memory & Data Tab */}
          {activeTab === "data" && (
            <div className="space-y-4">
              <div className="flex justify-between items-center border-b border-black pb-1">
                <h3 className="font-bold text-sm">
                  Stored Memories ({memories.length})
                </h3>
                <button
                  onClick={forgetEverything}
                  className="px-2 py-1 bg-red-600 text-white rounded font-bold text-[11px] hover:bg-red-700 flex items-center gap-1"
                >
                  <Trash2 className="w-3 h-3" />
                  <span>Forget Everything</span>
                </button>
              </div>

              <div className="max-h-60 overflow-y-auto space-y-1.5">
                {memories.length === 0 ? (
                  <div className="text-gray-500 italic text-center py-6">
                    No memories saved yet.
                  </div>
                ) : (
                  memories.map((m) => (
                    <div
                      key={m.id}
                      className="p-2 border border-black rounded flex justify-between items-center bg-[#fbf9f5]"
                    >
                      <div>
                        <p className="font-bold">{m.fact}</p>
                        <p className="text-[10px] text-gray-500">
                          {m.created_at}
                        </p>
                      </div>
                      <button
                        onClick={() => deleteMemory(m.id)}
                        className="hover:bg-red-100 p-1 rounded text-red-600"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  ))
                )}
              </div>
            </div>
          )}

          {/* Diagnostics Tab */}
          {activeTab === "diagnostics" && (
            <div className="space-y-6">
              <section className="space-y-3">
                <h3 className="font-bold text-sm border-b border-black pb-1">
                  Audio Diagnostics
                </h3>
                <AudioDiagnosticsPanel />
              </section>

              <section className="space-y-3">
                <h3 className="font-bold text-sm border-b border-black pb-1">
                  Offline Canned Utterance Tests
                </h3>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    onClick={() => triggerCanned("greeting")}
                    className="p-2 bg-white border border-black rounded font-bold hover:bg-gray-100"
                  >
                    Test Greeting
                  </button>
                  <button
                    onClick={() => triggerCanned("click")}
                    className="p-2 bg-white border border-black rounded font-bold hover:bg-gray-100"
                  >
                    Test Click Remark
                  </button>
                  <button
                    onClick={() => triggerCanned("dismiss")}
                    className="p-2 bg-white border border-black rounded font-bold hover:bg-gray-100"
                  >
                    Test Dismiss Remark
                  </button>
                  <button
                    onClick={() => triggerCanned("error")}
                    className="p-2 bg-white border border-black rounded font-bold hover:bg-gray-100"
                  >
                    Test Error Remark
                  </button>
                </div>
              </section>
            </div>
          )}
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
