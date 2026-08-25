import React, { useCallback, useEffect, useState } from "react";
import { Activity, RefreshCw } from "lucide-react";
import { tauriBridge } from "../../lib/tauriBridge";
import type { AsrDiagnostics } from "../../types/moose";

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

const valueOrDash = (value: string | number | null) =>
  value === null ? "—" : String(value);

const millisecondsOrDash = (value: number | null) =>
  value === null ? "—" : `${value} ms`;

const memoryOrDash = (bytes: number | null) =>
  bytes === null ? "—" : `${(bytes / 1024 / 1024).toFixed(1)} MiB`;

const percentOrDash = (value: number | null) =>
  value === null ? "—" : `${value.toFixed(1)}%`;

const metricsSource = (diagnostics: AsrDiagnostics) => {
  if (diagnostics.model_id === null) return "Cloud mode — no local metrics";
  if (diagnostics.metrics_snapshot) return "Last completed local session";
  if (diagnostics.streaming) return "Active local session";
  return "No local session measurements yet";
};

const memoryDeltaOrDash = (value: number | null, baseline: number | null) => {
  if (value === null || baseline === null) return "—";
  const delta = Math.max(0, value - baseline);
  return `${(delta / 1024 / 1024).toFixed(1)} MiB`;
};

export const AsrDiagnosticsPanel: React.FC = () => {
  const [diagnostics, setDiagnostics] = useState<AsrDiagnostics | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setDiagnostics(await tauriBridge.getAsrDiagnostics());
      setError(null);
    } catch (refreshError) {
      setError(String(refreshError));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <section className="border border-black rounded p-3 bg-[#fbf9f5] text-[11px] space-y-2">
      <div className="flex items-center justify-between gap-2">
        <h4 className="font-bold flex items-center gap-1">
          <Activity className="w-3.5 h-3.5" /> ASR Diagnostics
        </h4>
        <button
          type="button"
          onClick={() => void refresh()}
          disabled={loading}
          className="p-1 border border-black rounded hover:bg-gray-100 disabled:opacity-40 focus-visible:ring-2 focus-visible:ring-black"
          title="Refresh speech-recognition diagnostics"
          aria-label="Refresh ASR diagnostics"
        >
          <RefreshCw
            aria-hidden="true"
            className={`w-3 h-3 ${loading ? "animate-spin" : ""}`}
          />
        </button>
      </div>

      {error && (
        <p role="alert" className="text-red-700">
          {error}
        </p>
      )}

      {diagnostics && (
        <div>
          <DiagnosticRow label="Engine" value={diagnostics.engine_name} />
          <DiagnosticRow
            label="Model"
            value={valueOrDash(diagnostics.model_id)}
          />
          <DiagnosticRow
            label="Revision"
            value={valueOrDash(diagnostics.model_revision)}
          />
          <DiagnosticRow
            label="Install status"
            value={valueOrDash(diagnostics.install_state)}
          />
          <DiagnosticRow
            label="Metrics source"
            value={metricsSource(diagnostics)}
          />
          <DiagnosticRow
            label="Streaming"
            value={diagnostics.streaming ? "Active" : "Inactive"}
          />
          <DiagnosticRow
            label="Input rate"
            value={`${diagnostics.input_sample_rate_hz} Hz`}
          />
          <DiagnosticRow
            label="Available CPU threads"
            value={valueOrDash(diagnostics.cpu_threads)}
          />
          <DiagnosticRow
            label="Inference queue"
            value={
              diagnostics.queue_capacity > 0
                ? `${diagnostics.queue_depth} / ${diagnostics.queue_capacity} chunks`
                : "—"
            }
          />
          <DiagnosticRow
            label="Dropped mic chunks"
            value={diagnostics.dropped_chunks}
          />
          <DiagnosticRow
            label="First useful partial"
            value={millisecondsOrDash(diagnostics.first_partial_latency_ms)}
          />
          <DiagnosticRow
            label="First final"
            value={millisecondsOrDash(diagnostics.first_final_latency_ms)}
          />
          <DiagnosticRow
            label="Last native decode"
            value={millisecondsOrDash(
              diagnostics.last_transcription_latency_ms,
            )}
          />
          <DiagnosticRow
            label="Processed audio"
            value={`${diagnostics.processed_audio_ms} ms`}
          />
          <DiagnosticRow
            label="Inference wall time"
            value={`${diagnostics.inference_wall_time_ms} ms`}
          />
          <DiagnosticRow
            label="Real-time factor"
            value={
              diagnostics.real_time_factor === null
                ? "—"
                : `${diagnostics.real_time_factor.toFixed(3)}×`
            }
          />
          <DiagnosticRow
            label="Process CPU time"
            value={millisecondsOrDash(diagnostics.process_cpu_time_ms)}
          />
          <DiagnosticRow
            label="Average process CPU"
            value={percentOrDash(diagnostics.average_cpu_utilization_percent)}
          />
          <DiagnosticRow
            label="RSS before local model"
            value={memoryOrDash(diagnostics.baseline_resident_memory_bytes)}
          />
          <DiagnosticRow
            label={
              diagnostics.metrics_snapshot
                ? "Session-final process RSS"
                : "Current process RSS"
            }
            value={memoryOrDash(diagnostics.resident_memory_bytes)}
          />
          <DiagnosticRow
            label={
              diagnostics.metrics_snapshot
                ? "Session-final RSS increase"
                : "Current RSS increase"
            }
            value={memoryDeltaOrDash(
              diagnostics.resident_memory_bytes,
              diagnostics.baseline_resident_memory_bytes,
            )}
          />
          <DiagnosticRow
            label="Highest sampled process RSS"
            value={memoryOrDash(diagnostics.peak_resident_memory_bytes)}
          />
          <DiagnosticRow
            label="Highest sampled RSS increase"
            value={memoryDeltaOrDash(
              diagnostics.peak_resident_memory_bytes,
              diagnostics.baseline_resident_memory_bytes,
            )}
          />
          <DiagnosticRow
            label="Last ASR error"
            value={
              diagnostics.last_error === null
                ? "—"
                : `${diagnostics.last_error.kind}: ${diagnostics.last_error.message}`
            }
          />
        </div>
      )}

      <p className="text-[10px] text-gray-500">
        For local Moonshine modes, a real-time factor below 1× means cumulative
        inference is faster than the amount of audio processed. CPU and RSS are
        process-level measurements while the local ASR pipeline is active; RSS
        increases are relative to the pre-model baseline rather than claimed as
        model-exclusive allocations. Completed-session values are retained from
        immediately before local-model teardown and labeled as snapshots.
      </p>
    </section>
  );
};
