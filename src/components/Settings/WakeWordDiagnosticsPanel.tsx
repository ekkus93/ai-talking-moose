import React, { useCallback, useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { tauriBridge } from "../../lib/tauriBridge";
import type { WakeWordDiagnostics } from "../../types/moose";

const show = (value: string | number | null) =>
  value === null ? "—" : String(value);

const Row: React.FC<{ label: string; value: React.ReactNode }> = ({
  label,
  value,
}) => (
  <div className="flex justify-between gap-4 border-b border-gray-200 py-1 last:border-b-0">
    <span className="text-gray-600">{label}</span>
    <span className="font-bold text-right break-all">{value}</span>
  </div>
);

export const WakeWordDiagnosticsPanel: React.FC = () => {
  const [diagnostics, setDiagnostics] = useState<WakeWordDiagnostics | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setDiagnostics(await tauriBridge.getWakeWordDiagnostics());
      setError(null);
    } catch (cause) {
      setError(String(cause));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <div className="space-y-3">
      <button
        type="button"
        onClick={() => void refresh()}
        className="px-3 py-1.5 bg-white border-2 border-black rounded font-bold hover:bg-gray-100 flex items-center gap-1.5"
      >
        <RefreshCw className="w-3.5 h-3.5" />
        Refresh Wake Diagnostics
      </button>
      {error && (
        <p role="status" className="text-[11px]">
          {error}
        </p>
      )}
      {diagnostics && (
        <div className="border border-black rounded p-3 bg-[#fbf9f5] text-[11px]">
          <Row label="Enabled" value={diagnostics.enabled ? "Yes" : "No"} />
          <Row label="State" value={diagnostics.state} />
          <Row
            label="Engine"
            value={`${diagnostics.engine} ${diagnostics.runtime_version}`}
          />
          <Row
            label="Platform"
            value={`${diagnostics.platform} / ${diagnostics.architecture}`}
          />
          <Row
            label="Inference threads"
            value={diagnostics.inference_threads}
          />
          <Row
            label="Audio"
            value={`${diagnostics.sample_rate_hz} Hz / ${diagnostics.channels} ch`}
          />
          <Row
            label="Ring buffer"
            value={`${diagnostics.ring_buffer_seconds}s / ${diagnostics.ring_buffer_capacity_samples} samples`}
          />
          <Row label="Keyword score" value={diagnostics.keywords_score} />
          <Row
            label="Trigger threshold"
            value={diagnostics.keywords_threshold}
          />
          <Row label="Triggers" value={diagnostics.trigger_count} />
          <Row
            label="Last trigger (Unix ms)"
            value={show(diagnostics.last_trigger_unix_ms)}
          />
          <Row
            label="Init duration"
            value={
              diagnostics.initialization_duration_ms === null
                ? "—"
                : `${diagnostics.initialization_duration_ms} ms`
            }
          />
          <Row
            label="Trigger → handoff"
            value={
              diagnostics.last_handoff_latency_ms === null
                ? "—"
                : `${diagnostics.last_handoff_latency_ms} ms`
            }
          />
          <Row
            label="Pre-roll replay"
            value={
              diagnostics.last_handoff_replay_duration_us === null
                ? "—"
                : `${diagnostics.last_handoff_replay_samples} samples / ${diagnostics.last_handoff_replay_duration_us} µs`
            }
          />
          <Row
            label="Suspended for Talking"
            value={diagnostics.suspended_for_talking ? "Yes" : "No"}
          />
          <Row
            label="Dropped KWS chunks"
            value={diagnostics.dropped_engine_chunks}
          />
          <Row
            label="Dropped command chunks"
            value={diagnostics.dropped_command_chunks}
          />
          <Row label="Last error" value={show(diagnostics.last_error)} />
        </div>
      )}
      <p className="text-[10px] text-gray-500">
        Wake diagnostics contain lifecycle/performance metadata only. Raw PCM
        and ring-buffer contents are never exposed here.
      </p>
    </div>
  );
};
