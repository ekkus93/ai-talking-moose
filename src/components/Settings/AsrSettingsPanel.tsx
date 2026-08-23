import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertCircle,
  CheckCircle,
  Cloud,
  Download,
  HardDrive,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { tauriBridge } from "../../lib/tauriBridge";
import { AsrDiagnosticsPanel } from "./AsrDiagnosticsPanel";
import { useMooseStore } from "../../stores/mooseStore";
import {
  AsrMode,
  AsrModelDescriptor,
  AsrModelInstallState,
  AsrModelProgressEvent,
} from "../../types/moose";

type LocalAsrMode = Exclude<AsrMode, "gemini_live_audio">;

const STATUS_LABELS: Record<AsrModelInstallState, string> = {
  not_installed: "Not installed",
  downloading: "Downloading…",
  verifying: "Verifying…",
  installed: "Installed",
  corrupt: "Corrupt",
  incompatible: "Incompatible",
  failed: "Failed",
};

const formatBytes = (bytes: number) =>
  `${(bytes / 1024 / 1024).toFixed(1)} MiB`;

const replaceModel = (
  models: AsrModelDescriptor[],
  replacement: AsrModelDescriptor,
) =>
  models.map((model) =>
    model.mode === replacement.mode ? replacement : model,
  );

export const AsrSettingsPanel: React.FC = () => {
  const { settings, updateSettings } = useMooseStore();
  const [models, setModels] = useState<AsrModelDescriptor[]>([]);
  const [progress, setProgress] = useState<
    Partial<Record<LocalAsrMode, AsrModelProgressEvent>>
  >({});
  const [isLoading, setIsLoading] = useState(true);
  const [operationMode, setOperationMode] = useState<LocalAsrMode | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadModels = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      setModels(await tauriBridge.getAsrModels());
    } catch (loadError) {
      setError(String(loadError));
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadModels();
    let disposed = false;
    let unlisten: (() => void) | null = null;
    void tauriBridge
      .onAsrModelProgress((event) => {
        if (disposed) return;
        setProgress((current) => ({ ...current, [event.mode]: event }));
        setModels((current) =>
          current.map((model) =>
            model.mode === event.mode
              ? {
                  ...model,
                  install_state: event.install_state,
                  error_message: null,
                }
              : model,
          ),
        );
      })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlisten = cleanup;
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [loadModels]);

  const modelsByMode = useMemo(
    () => new Map(models.map((model) => [model.mode, model])),
    [models],
  );

  if (!settings) return null;

  const selectMode = async (mode: AsrMode) => {
    setError(null);
    try {
      await updateSettings({ ...settings, asr_mode: mode });
      await loadModels();
    } catch (selectionError) {
      setError(String(selectionError));
    }
  };

  const installModel = async (mode: LocalAsrMode) => {
    setOperationMode(mode);
    setError(null);
    setProgress((current) => ({
      ...current,
      [mode]: {
        mode,
        install_state: "downloading",
        downloaded_bytes: 0,
        total_bytes: modelsByMode.get(mode)?.expected_bytes ?? 0,
        current_file: null,
      },
    }));
    try {
      const model = await tauriBridge.installAsrModel(mode);
      setModels((current) => replaceModel(current, model));
      setProgress((current) => ({ ...current, [mode]: undefined }));
    } catch (installError) {
      const message = String(installError);
      setError(message);
      setModels((current) =>
        current.map((model) =>
          model.mode === mode
            ? { ...model, install_state: "failed", error_message: message }
            : model,
        ),
      );
    } finally {
      setOperationMode(null);
    }
  };

  const deleteModel = async (mode: LocalAsrMode) => {
    setOperationMode(mode);
    setError(null);
    try {
      const model = await tauriBridge.deleteAsrModel(mode);
      setModels((current) => replaceModel(current, model));
      setProgress((current) => ({ ...current, [mode]: undefined }));
    } catch (deleteError) {
      setError(String(deleteError));
    } finally {
      setOperationMode(null);
    }
  };

  const renderLocalModel = (
    mode: LocalAsrMode,
    title: string,
    summary: string,
  ) => {
    const model = modelsByMode.get(mode);
    const modelProgress = progress[mode];
    const selected = settings.asr_mode === mode;
    const busy =
      operationMode === mode ||
      model?.install_state === "downloading" ||
      model?.install_state === "verifying";
    const percent =
      modelProgress && modelProgress.total_bytes > 0
        ? Math.min(
            100,
            Math.round(
              (modelProgress.downloaded_bytes / modelProgress.total_bytes) *
                100,
            ),
          )
        : 0;

    return (
      <div
        className={`border rounded p-3 space-y-2 ${selected ? "border-black border-2" : "border-gray-400"}`}
      >
        <label className="flex items-start gap-2 cursor-pointer">
          <input
            type="radio"
            name="asr-mode"
            value={mode}
            checked={selected}
            onChange={() => void selectMode(mode)}
            className="mt-0.5 accent-black focus-visible:ring-2 focus-visible:ring-black"
          />
          <span className="flex-1">
            <span className="font-bold block">{title}</span>
            <span className="text-[11px] text-gray-700 block">{summary}</span>
          </span>
        </label>

        {model ? (
          <div className="ml-5 space-y-2 text-[11px]">
            <div className="flex flex-wrap items-center gap-2">
              <span className="px-1.5 py-0.5 border border-black rounded bg-gray-50 font-bold">
                {STATUS_LABELS[model.install_state]}
              </span>
              {model.active && <span className="font-bold">In use now</span>}
              <span>{formatBytes(model.expected_bytes)}</span>
              <span>Revision {model.revision}</span>
              <span>Runtime {model.runtime_release}</span>
            </div>

            {modelProgress && busy && (
              <div aria-live="polite" className="space-y-1">
                <div
                  className="h-2 border border-black bg-white"
                  aria-label={`${title} download progress`}
                >
                  <div
                    className="h-full bg-black"
                    style={{
                      width: `${modelProgress.install_state === "verifying" ? 100 : percent}%`,
                    }}
                  />
                </div>
                <div className="flex justify-between gap-2 text-gray-600">
                  <span>
                    {modelProgress.install_state === "verifying"
                      ? "Verifying SHA-256/CRC32C and install metadata…"
                      : modelProgress.current_file
                        ? `Downloading ${modelProgress.current_file}`
                        : "Starting download…"}
                  </span>
                  <span>
                    {modelProgress.install_state === "verifying"
                      ? "100%"
                      : `${percent}%`}
                  </span>
                </div>
              </div>
            )}

            {model.error_message && (
              <p className="text-red-700" role="alert">
                {model.error_message}
              </p>
            )}

            <div className="flex gap-2">
              {model.install_state === "installed" ? (
                <button
                  type="button"
                  onClick={() => void deleteModel(mode)}
                  disabled={busy || model.active}
                  title={
                    model.active
                      ? "Stop the active conversation before deleting this model"
                      : `Delete ${title}`
                  }
                  className="px-2 py-1 border border-black rounded font-bold flex items-center gap-1 disabled:opacity-40 focus-visible:ring-2 focus-visible:ring-black"
                >
                  <Trash2 className="w-3 h-3" /> Delete
                </button>
              ) : model.install_state === "incompatible" ? (
                <span className="text-red-700 font-bold">
                  Update the application before using this model.
                </span>
              ) : (
                <button
                  type="button"
                  onClick={() => void installModel(mode)}
                  disabled={busy}
                  className="px-2 py-1 bg-black text-white rounded font-bold flex items-center gap-1 disabled:opacity-40 focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-black"
                >
                  {model.install_state === "corrupt" ||
                  model.install_state === "failed" ? (
                    <RefreshCw className="w-3 h-3" />
                  ) : (
                    <Download className="w-3 h-3" />
                  )}
                  {model.install_state === "corrupt"
                    ? "Reinstall"
                    : model.install_state === "failed"
                      ? "Retry"
                      : "Download"}
                </button>
              )}
            </div>
          </div>
        ) : (
          <div className="ml-5 text-[11px] text-gray-500">
            {isLoading
              ? "Verifying local model state…"
              : "Model status unavailable."}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between border-b border-black pb-1">
        <h3 className="font-bold text-sm">Speech Recognition</h3>
        <button
          type="button"
          onClick={() => void loadModels()}
          disabled={isLoading}
          className="p-1 border border-black rounded hover:bg-gray-100 disabled:opacity-40 focus-visible:ring-2 focus-visible:ring-black"
          title="Re-check local model integrity"
        >
          <RefreshCw
            className={`w-3.5 h-3.5 ${isLoading ? "animate-spin" : ""}`}
          />
        </button>
      </div>

      <fieldset className="space-y-3">
        <legend className="sr-only">Choose speech recognition mode</legend>
        {renderLocalModel(
          "moonshine_tiny_streaming",
          "Moonshine Tiny Streaming",
          "Default. Fast local English speech recognition with the smallest CPU and disk footprint.",
        )}
        {renderLocalModel(
          "moonshine_small_streaming",
          "Moonshine Small Streaming",
          "Higher-capacity local English speech recognition with a larger model and CPU footprint.",
        )}

        <div
          className={`border rounded p-3 ${settings.asr_mode === "gemini_live_audio" ? "border-black border-2" : "border-gray-400"}`}
        >
          <label className="flex items-start gap-2 cursor-pointer">
            <input
              type="radio"
              name="asr-mode"
              value="gemini_live_audio"
              checked={settings.asr_mode === "gemini_live_audio"}
              onChange={() => void selectMode("gemini_live_audio")}
              className="mt-0.5 accent-black focus-visible:ring-2 focus-visible:ring-black"
            />
            <span className="flex-1">
              <span className="font-bold flex items-center gap-1">
                <Cloud className="w-3.5 h-3.5" /> Gemini Live Cloud Audio
              </span>
              <span className="text-[11px] text-gray-700 block">
                Google performs speech recognition from microphone audio sent
                during the active conversation.
              </span>
            </span>
          </label>
        </div>
      </fieldset>

      <div className="p-3 bg-gray-50 border border-gray-300 rounded text-[11px] space-y-2">
        <div className="flex items-center gap-1 font-bold">
          <HardDrive className="w-3.5 h-3.5" /> Audio privacy boundary
        </div>
        {settings.asr_mode === "gemini_live_audio" ? (
          <p>
            <strong>Cloud mode selected:</strong> microphone audio is sent to
            Google only while a conversation is active.
          </p>
        ) : (
          <p>
            <strong>Local mode selected:</strong> microphone PCM stays on this
            computer. Only finalized transcript text is sent to Gemini so the
            Moose can generate a reply.
          </p>
        )}
        <p>
          Local recognition errors never switch to cloud microphone upload
          automatically. Choosing Gemini Live Cloud Audio is always an explicit
          setting change.
        </p>
      </div>

      <AsrDiagnosticsPanel key={settings.asr_mode} />

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

      {!error &&
        settings.asr_mode !== "gemini_live_audio" &&
        modelsByMode.get(settings.asr_mode)?.install_state === "installed" && (
          <div
            className="text-green-800 flex gap-1 items-center text-[11px]"
            aria-live="polite"
          >
            <CheckCircle className="w-3.5 h-3.5" /> Selected local model is
            installed and verified.
          </div>
        )}
    </div>
  );
};
