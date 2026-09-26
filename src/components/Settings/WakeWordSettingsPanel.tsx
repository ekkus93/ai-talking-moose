import React, { useEffect, useState } from "react";
import { AlertCircle, CheckCircle, Mic, RefreshCw, Shield } from "lucide-react";
import { tauriBridge } from "../../lib/tauriBridge";
import { useMooseStore } from "../../stores/mooseStore";
import type {
  WakeWordDiagnostics,
  WakeWordRuntimePhase,
} from "../../types/moose";

const RUNTIME_LABELS: Record<WakeWordRuntimePhase, string> = {
  disabled: "Disabled",
  loading: "Loading",
  listening: "Listening locally",
  triggered: "Triggered / handing off",
  suspended_talking: "Suspended while Moose talks",
  error: "Error",
  shutting_down: "Shutting down",
};

const statusTone = (phase: WakeWordRuntimePhase) => {
  switch (phase) {
    case "listening":
      return "bg-green-50 text-green-900 border-green-800";
    case "loading":
    case "triggered":
    case "suspended_talking":
      return "bg-amber-50 text-amber-900 border-amber-800";
    case "error":
      return "bg-red-50 text-red-900 border-red-800";
    case "disabled":
    case "shutting_down":
    default:
      return "bg-gray-50 text-gray-900 border-gray-500";
  }
};

export const WakeWordSettingsPanel: React.FC = () => {
  const { settings, updateSettingsPatch } = useMooseStore();
  const [diagnostics, setDiagnostics] = useState<WakeWordDiagnostics | null>(
    null,
  );
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshDiagnostics = async () => {
    setIsRefreshing(true);
    setError(null);
    try {
      setDiagnostics(await tauriBridge.getWakeWordDiagnostics());
    } catch (refreshError) {
      setError(String(refreshError));
    } finally {
      setIsRefreshing(false);
    }
  };

  useEffect(() => {
    void refreshDiagnostics();
  }, []);

  if (!settings) return null;

  const phase = diagnostics?.runtime_phase ?? "disabled";
  const enabled = settings.wake_word_enabled;
  const preferenceStatus = enabled
    ? "Enabled — runtime starts only after developer-prepared local KWS artifacts, local Moonshine ASR policy, and lifecycle state permit."
    : "Disabled — manual start remains available.";

  const setEnabled = async (nextEnabled: boolean) => {
    setIsSaving(true);
    setError(null);
    try {
      await updateSettingsPatch({
        wake_word_enabled: nextEnabled,
        wake_word_phrase: "Hey, Moose",
      });
      await refreshDiagnostics();
    } catch (saveError) {
      setError(String(saveError));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between border-b border-black pb-1">
        <h3 className="font-bold text-sm">Wake Word</h3>
        <button
          type="button"
          onClick={() => void refreshDiagnostics()}
          disabled={isRefreshing}
          className="p-1 border border-black rounded hover:bg-gray-100 disabled:opacity-40 focus-visible:ring-2 focus-visible:ring-black"
          title="Refresh Wake Word status"
          aria-label="Refresh Wake Word status"
        >
          <RefreshCw
            aria-hidden="true"
            className={`w-3.5 h-3.5 ${isRefreshing ? "animate-spin" : ""}`}
          />
        </button>
      </div>

      <section className="border border-black rounded p-3 space-y-3">
        <label className="flex items-start gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={enabled}
            disabled={isSaving}
            onChange={(event) => void setEnabled(event.currentTarget.checked)}
            className="mt-1 accent-black focus-visible:ring-2 focus-visible:ring-black"
            aria-describedby="wake-word-toggle-description"
          />
          <span className="flex-1">
            <span className="font-bold block">Enable wake word</span>
            <span
              id="wake-word-toggle-description"
              className="text-[11px] text-gray-700 block"
            >
              Listen locally for the fixed phrase before starting a normal Moose
              interaction.
            </span>
          </span>
        </label>

        <div>
          <div className="font-bold block mb-1 text-[11px]">
            Wake word phrase
          </div>
          <div
            aria-label="Wake word phrase"
            className="w-full px-2 py-1 border border-black rounded bg-gray-100 font-mono"
          >
            Hey, Moose
          </div>
          <p className="mt-1 text-[11px] text-gray-700">
            V1 uses a fixed phrase. Custom phrases and sensitivity controls are
            intentionally not exposed.
          </p>
        </div>
        <p className="text-[11px] font-bold">{preferenceStatus}</p>
      </section>

      <section className="p-3 bg-gray-50 border border-gray-300 rounded text-[11px] space-y-2">
        <div className="flex items-center gap-1 font-bold">
          <Shield className="w-3.5 h-3.5" /> Privacy boundary
        </div>
        <p>Wake Word V1 uses local/offline keyword spotting.</p>
        <p>The microphone remains locally active while listening.</p>
        <p>Wake detection is not full-time cloud transcription.</p>
        <p>Wake-triggered commands require local Moonshine command ASR.</p>
        <p>
          Wake model/runtime artifacts are developer-prepared; a clean install
          fails closed until the pinned artifacts are prepared and verified.
        </p>
        <p>Wake Word V1 has no barge-in support while Moose talks.</p>
        <p>
          Manual start remains available. Existing command ASR still starts only
          through the normal conversation path after Wake Word accepts a
          trigger.
        </p>
      </section>

      <section className="border rounded p-3 space-y-2">
        <div className="flex items-center justify-between gap-2">
          <span className="font-bold flex items-center gap-1">
            <Mic className="w-3.5 h-3.5" /> Runtime status
          </span>
          <span
            role="status"
            aria-live="polite"
            className={`px-2 py-0.5 border rounded font-bold ${statusTone(phase)}`}
          >
            Runtime: {RUNTIME_LABELS[phase]}
          </span>
        </div>
        {diagnostics ? (
          <dl className="grid grid-cols-2 gap-x-3 gap-y-1 text-[11px]">
            <dt className="font-bold">Engine</dt>
            <dd>{diagnostics.engine_id}</dd>
            <dt className="font-bold">Model</dt>
            <dd>{diagnostics.model_id}</dd>
            <dt className="font-bold">Sample format</dt>
            <dd>
              {`${diagnostics.canonical_sample_rate_hz.toLocaleString()} Hz / ${diagnostics.canonical_channels} channel`}
            </dd>
            <dt className="font-bold">Policy</dt>
            <dd>
              {`${diagnostics.inference_threads} thread, threshold ${diagnostics.threshold}, score ${diagnostics.score}`}
            </dd>
          </dl>
        ) : (
          <p className="text-gray-600 text-[11px]">
            Wake Word diagnostics are not loaded yet.
          </p>
        )}
      </section>

      {diagnostics?.last_error && (
        <div
          role="alert"
          className="p-2 border border-red-600 bg-red-50 text-red-800 rounded flex gap-2 items-start"
        >
          <AlertCircle className="w-4 h-4 flex-shrink-0" />
          <span>{diagnostics.last_error}</span>
        </div>
      )}

      {error && (
        <div
          role="alert"
          aria-live="assertive"
          className="p-2 border border-red-600 bg-red-50 text-red-800 rounded flex gap-2 items-start"
        >
          <AlertCircle className="w-4 h-4 flex-shrink-0" />
          <span>{error}</span>
        </div>
      )}

      {!error && enabled && diagnostics?.last_error === null && (
        <div
          className="text-green-800 flex gap-1 items-center text-[11px]"
          aria-live="polite"
        >
          <CheckCircle className="w-3.5 h-3.5" /> Wake Word preference is
          enabled. Runtime availability still depends on developer-prepared local
          KWS artifacts, local Moonshine command ASR policy, and lifecycle state.
        </div>
      )}
    </div>
  );
};
