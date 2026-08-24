import React from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { MicrophonePermissionCard } from "./MicrophonePermissionCard";
import { Trash2, RotateCcw } from "lucide-react";

export const PrivacyTab: React.FC = () => {
  const { settings, updateSettings, memories, forgetEverything } =
    useMooseStore();
  if (!settings) return null;

  const resetPrivacyDefaults = async () => {
    await updateSettings({
      ...settings,
      active_app_observation: false,
      window_title_observation: false,
      memory_enabled: false,
      save_transcripts: false,
    });
  };

  return (
    <div className="space-y-4">
      <h3 className="font-bold text-sm border-b border-black pb-1">
        Privacy & Permissions
      </h3>

      <section
        className="border-2 border-black rounded bg-[#fbf9f5] p-3 space-y-2"
        aria-labelledby="privacy-summary-heading"
      >
        <h4 id="privacy-summary-heading" className="font-bold">
          Active privacy summary
        </h4>
        <ul className="text-[11px] space-y-1 list-disc pl-4">
          <li>
            Microphone: used only during active conversations or tests; live OS
            permission is shown below.
          </li>
          <li>
            Active-app observation:{" "}
            <strong>{settings.active_app_observation ? "On" : "Off"}</strong>.
          </li>
          <li>
            Window titles: <strong>Off</strong> in V1; title observation remains
            fail-closed.
          </li>
          <li>
            Cross-conversation memory:{" "}
            <strong>{settings.memory_enabled ? "On" : "Off"}</strong> (
            {memories.length} stored {memories.length === 1 ? "fact" : "facts"}
            ).
          </li>
          <li>
            Transcript retention:{" "}
            <strong>{settings.save_transcripts ? "On" : "Off"}</strong>. When
            Off, finalized transcript text is not retained after the live
            session.
          </li>
        </ul>
        <div className="flex flex-wrap gap-2 pt-1">
          <button
            type="button"
            onClick={() => void forgetEverything()}
            className="px-2 py-1 bg-red-700 text-white border border-black rounded font-bold flex items-center gap-1"
          >
            <Trash2 className="w-3 h-3" aria-hidden="true" />
            Forget stored data
          </button>
          <button
            type="button"
            onClick={() => void resetPrivacyDefaults()}
            className="px-2 py-1 bg-white border border-black rounded font-bold flex items-center gap-1"
          >
            <RotateCcw className="w-3 h-3" aria-hidden="true" />
            Reset privacy defaults
          </button>
        </div>
      </section>

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

      <div className="p-3 bg-gray-50 border border-gray-400 rounded text-[11px] text-gray-800 space-y-1">
        <p className="font-bold">Microphone privacy:</p>
        {settings.asr_mode === "gemini_live_audio" ? (
          <p>
            - Gemini Live Cloud Audio is selected: microphone audio is sent to
            Google only during an active conversation.
          </p>
        ) : (
          <p>
            - Moonshine local ASR is selected: microphone PCM stays on this
            computer; only finalized transcript text is sent to Gemini for a
            reply.
          </p>
        )}
        <p>
          - Local ASR failures never silently switch to cloud microphone upload.
        </p>
        <p>
          - Screen contents, OCR, keystrokes, window titles, and files are not
          collected in V1.
        </p>
      </div>
    </div>
  );
};
