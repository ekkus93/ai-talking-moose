import React from "react";
import { AlertCircle, Mic2 } from "lucide-react";
import { useMooseStore } from "../../stores/mooseStore";

export const WakeWordSettingsPanel: React.FC = () => {
  const { settings, updateSettingsPatch } = useMooseStore();
  if (!settings) return null;

  const status = settings.wake_word_enabled
    ? "Enabled — runtime starts when Wake Word lifecycle integration is active."
    : "Disabled — manual conversation start remains available.";

  const toggleWakeWord = (enabled: boolean) => {
    void updateSettingsPatch({
      wake_word_enabled: enabled,
      wake_word_phrase: "Hey, Moose",
    });
  };

  return (
    <section
      className="border border-black rounded bg-[#fbf9f5] p-3 space-y-3"
      aria-labelledby="wake-word-settings-heading"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="space-y-1">
          <div
            id="wake-word-settings-heading"
            className="flex items-center gap-1.5 font-bold"
          >
            <Mic2 className="w-3.5 h-3.5" aria-hidden="true" />
            Wake Word
          </div>
          <p className="text-[11px] text-gray-700">
            Say <strong>Hey, Moose</strong> to start a normal command once the
            Wake Word runtime is listening.
          </p>
        </div>
        <label className="flex items-center gap-2 cursor-pointer font-bold whitespace-nowrap">
          <input
            type="checkbox"
            checked={settings.wake_word_enabled}
            onChange={(event) => toggleWakeWord(event.target.checked)}
            className="accent-black focus-visible:ring-2 focus-visible:ring-black"
            aria-describedby="wake-word-status wake-word-disclosure"
          />
          <span>Enable wake word</span>
        </label>
      </div>

      <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-[11px]">
        <dt className="font-bold">Phrase</dt>
        <dd aria-label="Wake word phrase">Hey, Moose</dd>
        <dt className="font-bold">Status</dt>
        <dd id="wake-word-status" aria-live="polite">
          {status}
        </dd>
      </dl>

      <div
        id="wake-word-disclosure"
        className="space-y-1 text-[11px] text-gray-700"
      >
        <p>
          Wake Word V1 uses local/offline keyword spotting. It does not expose an
          editable phrase or sensitivity control.
        </p>
        <p>
          When enabled and listening, the microphone remains locally active so
          Moose can detect the fixed phrase. Wake Word detection itself is not
          full-time cloud transcription.
        </p>
        <p>
          After a trigger, the wake phrase plus immediate command audio may be
          handed to the normal speech-recognition path selected above.
        </p>
      </div>

      <div className="flex gap-2 text-[11px] text-amber-800 bg-amber-50 border border-amber-300 rounded p-2">
        <AlertCircle className="w-3.5 h-3.5 flex-shrink-0" aria-hidden="true" />
        <span>
          Wake Word V1 is disabled by default and has no barge-in support; Moose
          will not listen for wake phrases while it is talking.
        </span>
      </div>
    </section>
  );
};
