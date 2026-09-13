import React, { useCallback, useEffect, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";
import { tauriBridge } from "../../lib/tauriBridge";
import type { LocalTtsDiagnostics } from "../../types/localTts";

const POLL_INTERVAL_MS = 1_500;

const valueOrDash = (value: string | number | null) =>
  value === null ? "—" : String(value);

const durationOrDash = (value: number | null) =>
  value === null ? "—" : `${Math.round(value)} ms`;

const rtfOrDash = (value: number | null) =>
  value === null ? "—" : value.toFixed(3);

interface DiagnosticRowProps {
  label: string;
  value: React.ReactNode;
}

const DiagnosticRow: React.FC<DiagnosticRowProps> = ({ label, value }) => (
  <div className="flex justify-between gap-4 border-b border-gray-200 py-1 last:border-b-0">
    <span className="text-gray-600">{label}</span>
    <span className="font-bold text-right break-all">{value}</span>
  </div>
);

export const LocalTtsDiagnosticsPanel: React.FC = () => {
  const [diagnostics, setDiagnostics] = useState<LocalTtsDiagnostics | null>(
    null,
  );
  const [unavailable, setUnavailable] = useState(false);
  const inFlight = useRef(false);
  const mounted = useRef(true);

  const refresh = useCallback(async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    try {
      const next = await tauriBridge.getLocalTtsDiagnostics();
      if (mounted.current) {
        setDiagnostics(next);
        setUnavailable(false);
      }
    } catch {
      if (mounted.current) setUnavailable(true);
    } finally {
      inFlight.current = false;
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    const interval = window.setInterval(() => void refresh(), POLL_INTERVAL_MS);
    return () => {
      mounted.current = false;
      window.clearInterval(interval);
    };
  }, [refresh]);

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          onClick={() => void refresh()}
          className="px-3 py-1.5 bg-white border-2 border-black rounded font-bold hover:bg-gray-100 flex items-center gap-1.5"
        >
          <RefreshCw className="w-3.5 h-3.5" />
          Refresh Local TTS
        </button>
        <span className="text-[10px] text-gray-500">
          Refreshes automatically while this diagnostics view is mounted.
        </span>
      </div>

      {unavailable && (
        <p
          role="status"
          className="border border-gray-300 bg-gray-50 p-2 rounded text-[11px]"
        >
          Local TTS diagnostics unavailable.
        </p>
      )}

      {diagnostics && (
        <div className="grid grid-cols-1 xl:grid-cols-2 gap-3">
          <section className="border border-black rounded p-3 bg-[#fbf9f5]">
            <h4 className="font-bold mb-2">Selection &amp; Install</h4>
            <DiagnosticRow label="Provider" value={diagnostics.provider} />
            <DiagnosticRow
              label="Selected model"
              value={diagnostics.selected_model_id}
            />
            <DiagnosticRow
              label="Selected voice"
              value={diagnostics.selected_voice_id}
            />
            <DiagnosticRow
              label="Install state"
              value={diagnostics.install_state}
            />
            <DiagnosticRow
              label="Expected bytes"
              value={diagnostics.expected_bytes}
            />
            <DiagnosticRow
              label="Installed bytes"
              value={valueOrDash(diagnostics.installed_bytes)}
            />
            <DiagnosticRow
              label="Installer error"
              value={valueOrDash(diagnostics.installer_error_category)}
            />
            <DiagnosticRow
              label="Installer retryable"
              value={
                diagnostics.installer_error_retryable === null
                  ? "—"
                  : diagnostics.installer_error_retryable
                    ? "Yes"
                    : "No"
              }
            />
          </section>

          <section className="border border-black rounded p-3 bg-[#fbf9f5]">
            <h4 className="font-bold mb-2">Runtime</h4>
            <DiagnosticRow label="Phase" value={diagnostics.runtime.phase} />
            <DiagnosticRow
              label="Loaded model"
              value={valueOrDash(diagnostics.runtime.loaded_model_id)}
            />
            <DiagnosticRow
              label="Sample rate"
              value={
                diagnostics.runtime.sample_rate_hz === null
                  ? "—"
                  : `${diagnostics.runtime.sample_rate_hz} Hz`
              }
            />
            <DiagnosticRow
              label="Inference threads"
              value={valueOrDash(diagnostics.runtime.inference_thread_count)}
            />
            <DiagnosticRow
              label="Model load"
              value={durationOrDash(
                diagnostics.runtime.last_model_load_duration_ms,
              )}
            />
            <DiagnosticRow
              label="Synthesis"
              value={durationOrDash(
                diagnostics.runtime.last_synthesis_duration_ms,
              )}
            />
            <DiagnosticRow
              label="Generated audio"
              value={durationOrDash(
                diagnostics.runtime.last_generated_audio_duration_ms,
              )}
            />
            <DiagnosticRow
              label="RTF"
              value={rtfOrDash(diagnostics.runtime.last_real_time_factor)}
            />
            <DiagnosticRow
              label="Runtime error"
              value={valueOrDash(diagnostics.runtime.last_error_category)}
            />
          </section>
        </div>
      )}

      <p className="text-[10px] text-gray-500">
        This surface exposes model/runtime state and timing only. It does not
        include utterance text, raw audio, credentials, or model filesystem
        paths.
      </p>
    </div>
  );
};
