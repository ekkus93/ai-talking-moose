import React, { useCallback, useEffect, useState } from "react";
import { AlertCircle, Mic2 } from "lucide-react";
import { tauriBridge } from "../../lib/tauriBridge";
import { useMooseStore } from "../../stores/mooseStore";
import type { WakeWordDiagnostics, WakeWordRuntimePhase } from "../../types/moose";

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

const RUNTIME_PHASE_LABELS: Record<WakeWordRuntimePhase, string> = {
  disabled: "Disabled",
  loading: "Loading",
  listening: "Listening",
  triggered: "Triggered",
  suspended_talking: "Suspended while Moose talks",
  error: "Error",
  shutting_down: "Shutting down",
};

const runtimeStatusText = (
  diagnostics: WakeWordDiagnostics | null,
  fallbackEnabled: boolean,
) => {
  if (!diagnostics) {
    return fallbackEnabled ? "Runtime status pending." : "Runtime disabled.";
  }

  return `Runtime: ${RUNTIME_PHASE_LABELS[diagnostics.runtime_phase]}`;
};

export const WakeWordSettingsPanel: React.FC = () => {
  const { settings, updateSettingsPatch } = useMooseStore();
  const [diagnostics, setDiagnostics] = useState<WakeWordDiagnostics | null>(
    null,
  );
  const [diagnosticsError, setDiagnosticsError] = useState<string | null>(null);
  const [diagnosticsLoading, setDiagnosticsLoading] = useState(false);

  const refreshDiagnostics = useCallback(async () => {
    setDiagnosticsLoading(true);
    try {
      setDiagnostics(await tauriBridge.getWakeWordDiagnostics());
      setDiagnosticsError(null);
    } catch {
      setDiagnostics(null);
      setDiagnosticsError("Wake Word runtime status is unavailable.");
    } finally {
      setDiagnosticsLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshDiagnostics();
  }, [refreshDiagnostics, settings?.wake_word_enabled]);

  if (!settings) return null;

  const status = settings.wake_word_enabled
    ? COPY.enabledStatus
    : COPY.disabledStatus;

  const toggleWakeWord = (enabled: boolean) => {
    void updateSettingsPatch({
      wake_word_enabled: enabled,
      wake_word_phrase: WAKE_WORD_PHRASE,
    }).then(refreshDiagnostics);
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
            aria-describedby="wake-word-status wake-word-runtime-status wake-word-disclosure"
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
        <dt className="font-bold">Runtime</dt>
        <dd id="wake-word-runtime-status" aria-live="polite">
          {diagnosticsLoading
            ? "Refreshing runtime status…"
            : runtimeStatusText(diagnostics, settings.wake_word_enabled)}
        </dd>
      </dl>

      {(diagnosticsError || diagnostics?.last_error) && (
        <div
          role="status"
          className="border border-amber-700 bg-amber-50 text-amber-900 rounded p-2 text-[11px]"
        >
          {diagnosticsError ?? diagnostics?.last_error}
        </div>
      )}

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
