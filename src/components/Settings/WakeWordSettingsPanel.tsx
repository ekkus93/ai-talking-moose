import React from "react";
import { AlertCircle, Mic2 } from "lucide-react";
import { useMooseStore } from "../../stores/mooseStore";

const WAKE_WORD_PHRASE = "Hey, Moose";

const COPY = {
  enabledStatus: "Enabled — runtime starts.",
  disabledStatus: "Disabled — manual start remains available.",
  summary: "Say Hey, Moose to start a normal command once listening.",
  local: "Wake Word V1 uses local/offline keyword spotting.",
  fixed: "The phrase is fixed and sensitivity is not exposed in V1.",
  microphone: "The microphone remains locally active while listening.",
  cloud: "Wake detection is not full-time cloud transcription.",
  handoff: "The wake phrase plus immediate command may enter normal ASR.",
  noBargeIn: "Wake Word V1 has no barge-in support while Moose talks.",
};

export const WakeWordSettingsPanel: React.FC = () => {
  const { settings, updateSettingsPatch } = useMooseStore();
  if (!settings) return null;

  const status = settings.wake_word_enabled
    ? COPY.enabledStatus
    : COPY.disabledStatus;

  const toggleWakeWord = (enabled: boolean) => {
    void updateSettingsPatch({
      wake_word_enabled: enabled,
      wake_word_phrase: WAKE_WORD_PHRASE,
    });
  };

  return (
    <section
      aria-labelledby="wake-word-settings-heading"
      className="border border-black rounded p-3 space-y-3"
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
          <p className="text-[11px] text-gray-700">{COPY.summary}</p>
        </div>

        <label className="flex items-center gap-2 font-bold">
          <input
            type="checkbox"
            checked={settings.wake_word_enabled}
            onChange={(event) => toggleWakeWord(event.target.checked)}
            aria-describedby="wake-word-status wake-word-disclosure"
          />
          <span>Enable wake word</span>
        </label>
      </div>

      <dl className="grid grid-cols-[auto_1fr] gap-x-3 text-[11px]">
        <dt className="font-bold">Phrase</dt>
        <dd aria-label="Wake word phrase">{WAKE_WORD_PHRASE}</dd>
        <dt className="font-bold">Status</dt>
        <dd id="wake-word-status" aria-live="polite">
          {status}
        </dd>
      </dl>

      <div id="wake-word-disclosure" className="space-y-1 text-[11px]">
        <p>{COPY.local}</p>
        <p>{COPY.fixed}</p>
        <p>{COPY.microphone}</p>
        <p>{COPY.cloud}</p>
        <p>{COPY.handoff}</p>
      </div>

      <div className="flex gap-2 text-[11px]">
        <AlertCircle className="w-3.5 h-3.5" aria-hidden="true" />
        <span>{COPY.noBargeIn}</span>
      </div>
    </section>
  );
};
