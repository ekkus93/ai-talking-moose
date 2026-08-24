import React, { useState } from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { tauriBridge } from "../../lib/tauriBridge";
import { Volume2 } from "lucide-react";

export const VoiceTab: React.FC = () => {
  const { settings, updateSettings, inputDevices, outputDevices } =
    useMooseStore();
  const [isAuditioning, setIsAuditioning] = useState(false);
  if (!settings) return null;

  return (
    <div className="space-y-4">
      <h3 className="font-bold text-sm border-b border-black pb-1">
        Audio & Speech Synthesis
      </h3>

      <div>
        <label htmlFor="settings-input-device" className="block mb-1 font-bold">
          Microphone Input Device
        </label>
        <select
          id="settings-input-device"
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
        <label
          htmlFor="settings-output-device"
          className="block mb-1 font-bold"
        >
          Audio Output Device
        </label>
        <select
          id="settings-output-device"
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
          <label htmlFor="settings-tts-voice" className="block mb-1 font-bold">
            Moose Voice Preset
          </label>
          <select
            id="settings-tts-voice"
            value={settings.tts_voice}
            onChange={(e) =>
              updateSettings({ ...settings, tts_voice: e.target.value })
            }
            className="w-full p-1.5 border border-black rounded bg-white font-bold"
          >
            <option value="Fenrir">
              Fenrir (Deeper, Gravelly Cartoon Moose - Recommended)
            </option>
            <option value="Charon">Charon (Deep, Slow & Deadpan)</option>
            <option value="Orus">Orus (Warm, Low-Pitched & Relaxed)</option>
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
  );
};
