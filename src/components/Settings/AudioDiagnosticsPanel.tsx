import React, { useCallback, useEffect, useState } from "react";
import { RefreshCw, Volume2 } from "lucide-react";
import { tauriBridge } from "../../lib/tauriBridge";
import type { AudioDiagnostics } from "../../types/moose";
import { MicrophonePermissionCard } from "./MicrophonePermissionCard";

const valueOrDash = (value: string | number | null) =>
  value === null ? "—" : String(value);

const percentage = (value: number) => `${Math.round(value * 100)}%`;

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

export const AudioDiagnosticsPanel: React.FC = () => {
  const [diagnostics, setDiagnostics] = useState<AudioDiagnostics | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [isBusy, setIsBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      setDiagnostics(await tauriBridge.getAudioDiagnostics());
      setStatusMessage(null);
    } catch (error) {
      setStatusMessage(String(error));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const runMicrophoneTest = async () => {
    setIsBusy(true);
    setStatusMessage(null);
    try {
      const result = await tauriBridge.testMicrophone();
      setDiagnostics(result.diagnostics);
      setStatusMessage(
        `Microphone test completed. Peak input level: ${percentage(result.peak_level)}.`,
      );
    } catch (error) {
      setStatusMessage(`Microphone test failed: ${String(error)}`);
      await refresh();
    } finally {
      setIsBusy(false);
    }
  };

  const runOutputTest = async () => {
    setIsBusy(true);
    setStatusMessage(null);
    try {
      setDiagnostics(await tauriBridge.testAudioOutput());
      setStatusMessage("Output test tone queued through Rust/CPAL playback.");
    } catch (error) {
      setStatusMessage(`Output test failed: ${String(error)}`);
      await refresh();
    } finally {
      setIsBusy(false);
    }
  };

  return (
    <div className="space-y-4">
      <MicrophonePermissionCard />

      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          onClick={() => void refresh()}
          disabled={isBusy}
          className="px-3 py-1.5 bg-white border-2 border-black rounded font-bold hover:bg-gray-100 flex items-center gap-1.5 disabled:opacity-50"
        >
          <RefreshCw className="w-3.5 h-3.5" />
          Refresh Diagnostics
        </button>
        <button
          type="button"
          onClick={runMicrophoneTest}
          disabled={isBusy}
          className="px-3 py-1.5 bg-white border-2 border-black rounded font-bold hover:bg-gray-100 disabled:opacity-50"
        >
          Test Microphone
        </button>
        <button
          type="button"
          onClick={runOutputTest}
          disabled={isBusy}
          className="px-3 py-1.5 bg-white border-2 border-black rounded font-bold hover:bg-gray-100 flex items-center gap-1.5 disabled:opacity-50"
        >
          <Volume2 className="w-3.5 h-3.5" />
          Test Output
        </button>
      </div>

      {statusMessage && (
        <p
          role="status"
          className="border border-gray-300 bg-gray-50 p-2 rounded text-[11px]"
        >
          {statusMessage}
        </p>
      )}

      {diagnostics && (
        <div className="grid grid-cols-1 xl:grid-cols-2 gap-3">
          <section className="border border-black rounded p-3 bg-[#fbf9f5]">
            <h4 className="font-bold mb-2">Microphone</h4>
            <DiagnosticRow
              label="Configured device"
              value={valueOrDash(diagnostics.configured_input_device)}
            />
            <DiagnosticRow
              label="Actual device"
              value={valueOrDash(diagnostics.capture.selected_device)}
            />
            <DiagnosticRow
              label="Sample rate"
              value={
                diagnostics.capture.sample_rate_hz === null
                  ? "—"
                  : `${diagnostics.capture.sample_rate_hz} Hz`
              }
            />
            <DiagnosticRow
              label="Format"
              value={valueOrDash(diagnostics.capture.sample_format)}
            />
            <DiagnosticRow
              label="Channels"
              value={valueOrDash(diagnostics.capture.channels)}
            />
            <DiagnosticRow
              label="Capture active"
              value={diagnostics.capture.active ? "Yes" : "No"}
            />
            <DiagnosticRow
              label="Input level"
              value={percentage(diagnostics.capture.input_level)}
            />
            <DiagnosticRow
              label="Dropped chunks"
              value={diagnostics.capture.dropped_chunks}
            />
            <DiagnosticRow
              label="Last error"
              value={valueOrDash(diagnostics.capture.last_error)}
            />
          </section>

          <section className="border border-black rounded p-3 bg-[#fbf9f5]">
            <h4 className="font-bold mb-2">Output</h4>
            <DiagnosticRow
              label="Configured device"
              value={valueOrDash(diagnostics.configured_output_device)}
            />
            <DiagnosticRow
              label="Actual device"
              value={valueOrDash(diagnostics.playback.selected_device)}
            />
            <DiagnosticRow
              label="Sample rate"
              value={
                diagnostics.playback.sample_rate_hz === null
                  ? "—"
                  : `${diagnostics.playback.sample_rate_hz} Hz`
              }
            />
            <DiagnosticRow
              label="Format"
              value={valueOrDash(diagnostics.playback.sample_format)}
            />
            <DiagnosticRow
              label="Channels"
              value={valueOrDash(diagnostics.playback.channels)}
            />
            <DiagnosticRow
              label="Playing"
              value={diagnostics.playback.playing ? "Yes" : "No"}
            />
            <DiagnosticRow
              label="Output level"
              value={percentage(diagnostics.playback.output_level)}
            />
            <DiagnosticRow
              label="Queue depth"
              value={`${diagnostics.playback.queue_depth_samples} / ${diagnostics.playback.queue_limit_samples} samples`}
            />
            <DiagnosticRow
              label="Dropped samples"
              value={diagnostics.playback.dropped_samples}
            />
            <DiagnosticRow
              label="Last error"
              value={valueOrDash(diagnostics.playback.last_error)}
            />
          </section>
        </div>
      )}

      <p className="text-[10px] text-gray-500">
        Microphone diagnostics remain local and do not contact Google. Audio
        tests are disabled by the backend while a conversation owns the audio
        graph.
      </p>
    </div>
  );
};
