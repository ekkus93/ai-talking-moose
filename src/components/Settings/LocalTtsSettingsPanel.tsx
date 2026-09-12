import React, { useCallback, useEffect, useMemo, useState } from "react";
import { AlertCircle, CheckCircle, Download, Trash2, X } from "lucide-react";
import { tauriBridge } from "../../lib/tauriBridge";
import type {
  LocalTtsInstallProgress,
  LocalTtsModelDescriptor,
} from "../../types/localTts";

interface LocalTtsSettingsPanelProps {
  selectedModelId: string;
  onSelectModel: (modelId: string) => Promise<void>;
  onInstalledChange?: (installed: boolean) => void;
}

const formatModelSize = (bytes: number): string =>
  `${Math.round(bytes / (1024 * 1024))} MiB`;

const installStateLabel = (
  state: LocalTtsModelDescriptor["install_state"],
): string => {
  switch (state) {
    case "not_installed":
      return "Not installed";
    case "downloading":
      return "Downloading";
    case "verifying":
      return "Verifying";
    case "promoting":
      return "Finalizing install";
    case "installed":
      return "Installed & verified";
    case "failed":
      return "Install failed";
  }
};

const replaceDescriptor = (
  models: LocalTtsModelDescriptor[],
  descriptor: LocalTtsModelDescriptor,
): LocalTtsModelDescriptor[] =>
  models.map((model) => (model.id === descriptor.id ? descriptor : model));

export const LocalTtsSettingsPanel: React.FC<LocalTtsSettingsPanelProps> = ({
  selectedModelId,
  onSelectModel,
  onInstalledChange,
}) => {
  const [models, setModels] = useState<LocalTtsModelDescriptor[]>([]);
  const [status, setStatus] = useState<"loading" | "ready" | "error">(
    "loading",
  );
  const [progress, setProgress] = useState<LocalTtsInstallProgress | null>(
    null,
  );
  const [selectionPending, setSelectionPending] = useState(false);
  const [installPending, setInstallPending] = useState(false);
  const [deletePending, setDeletePending] = useState(false);
  const [result, setResult] = useState<{
    success: boolean;
    message: string;
  } | null>(null);

  const refreshModels = useCallback(async (showLoading = false) => {
    if (showLoading) setStatus("loading");
    try {
      const nextModels = await tauriBridge.getLocalTtsModels();
      setModels(nextModels ?? []);
      setStatus("ready");
    } catch (error) {
      setStatus("error");
      setResult({
        success: false,
        message: `Could not read Local TTS model status: ${String(error)}`,
      });
    }
  }, []);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;

    void refreshModels(true);
    void tauriBridge
      .onLocalTtsModelProgress((nextProgress) => {
        if (!active) return;
        setProgress(nextProgress);
        setModels((current) =>
          current.map((model) =>
            model.id === nextProgress.model_id
              ? {
                  ...model,
                  install_state: nextProgress.install_state,
                  error: null,
                }
              : model,
          ),
        );
      })
      .then((cleanup) => {
        if (!active) {
          cleanup();
          return;
        }
        unlisten = cleanup;
      })
      .catch((error) => {
        if (!active) return;
        setResult({
          success: false,
          message: `Local TTS progress updates are unavailable: ${String(error)}`,
        });
      });

    return () => {
      active = false;
      unlisten?.();
    };
  }, [refreshModels]);

  const selectedModel = useMemo(
    () => models.find((model) => model.id === selectedModelId) ?? null,
    [models, selectedModelId],
  );
  const selectedProgress =
    progress?.model_id === selectedModelId ? progress : null;
  const isInstallActive =
    installPending ||
    selectedModel?.install_state === "downloading" ||
    selectedModel?.install_state === "verifying" ||
    selectedModel?.install_state === "promoting";
  const modelSelectionDisabled =
    status !== "ready" || selectionPending || isInstallActive || deletePending;

  useEffect(() => {
    onInstalledChange?.(selectedModel?.install_state === "installed");
  }, [onInstalledChange, selectedModel?.install_state]);

  const handleSelectModel = async (modelId: string) => {
    if (modelId === selectedModelId || modelSelectionDisabled) return;
    setSelectionPending(true);
    setResult(null);
    try {
      await onSelectModel(modelId);
    } catch (error) {
      setResult({ success: false, message: String(error) });
    } finally {
      setSelectionPending(false);
    }
  };

  const handleInstall = async () => {
    if (!selectedModel || selectionPending) return;
    setInstallPending(true);
    setResult(null);
    try {
      const installed = await tauriBridge.installLocalTtsModel(
        selectedModel.id,
      );
      setModels((current) => replaceDescriptor(current, installed));
      setProgress(null);
      setResult({
        success: true,
        message: `${installed.display_name} installed and verified.`,
      });
    } catch (error) {
      setResult({ success: false, message: String(error) });
      setProgress(null);
      await refreshModels();
    } finally {
      setInstallPending(false);
    }
  };

  const handleCancel = async () => {
    if (!selectedModel) return;
    try {
      const accepted = await tauriBridge.cancelLocalTtsInstall(
        selectedModel.id,
      );
      setResult(
        accepted
          ? {
              success: true,
              message: `Cancellation requested for ${selectedModel.display_name}.`,
            }
          : {
              success: false,
              message: `No active Local TTS download exists for ${selectedModel.display_name}.`,
            },
      );
    } catch (error) {
      setResult({ success: false, message: String(error) });
    }
  };

  const handleDelete = async () => {
    if (!selectedModel || selectionPending) return;
    const confirmed = window.confirm(
      `Delete the installed Local TTS model “${selectedModel.display_name}”? It will remain selected but cannot speak until downloaded again.`,
    );
    if (!confirmed) return;

    setDeletePending(true);
    setResult(null);
    try {
      const deleted = await tauriBridge.deleteLocalTtsModel(selectedModel.id);
      setModels((current) => replaceDescriptor(current, deleted));
      setProgress(null);
      setResult({
        success: true,
        message: `${deleted.display_name} was deleted.`,
      });
    } catch (error) {
      setResult({ success: false, message: String(error) });
      await refreshModels();
    } finally {
      setDeletePending(false);
    }
  };

  return (
    <div className="space-y-3">
      <div>
        <label
          htmlFor="settings-local-tts-model"
          className="block mb-1 font-bold"
        >
          Local Speech Model
        </label>
        <select
          id="settings-local-tts-model"
          value={selectedModelId}
          disabled={modelSelectionDisabled}
          aria-busy={status === "loading" || selectionPending}
          onChange={(event) => void handleSelectModel(event.target.value)}
          className="w-full p-1.5 border border-black rounded bg-white font-bold disabled:opacity-60"
        >
          {status === "loading" ? (
            <option value={selectedModelId}>
              Current: {selectedModelId} (loading local catalog…)
            </option>
          ) : status === "error" ? (
            <option value={selectedModelId}>
              Current: {selectedModelId} (catalog unavailable)
            </option>
          ) : (
            <>
              {!selectedModel && (
                <option value={selectedModelId} disabled>
                  Unavailable: {selectedModelId}
                </option>
              )}
              {models.map((model) => (
                <option key={model.id} value={model.id}>
                  {model.display_name} v{model.version} —{" "}
                  {formatModelSize(model.expected_bytes)}
                </option>
              ))}
            </>
          )}
        </select>
        <p className="mt-1 text-[10px] text-gray-700">
          Selecting a Local speech model never downloads it. Use the explicit
          Download & Verify action below.
        </p>
      </div>

      {selectedModel && (
        <div className="border border-gray-400 rounded p-3 space-y-2 bg-gray-50">
          <div className="flex items-start justify-between gap-3">
            <div>
              <div className="font-bold">
                {selectedModel.display_name} v{selectedModel.version}
              </div>
              <div className="text-[10px] text-gray-700">
                {formatModelSize(selectedModel.expected_bytes)} download •{" "}
                {selectedModel.license}
              </div>
            </div>
            <span className="text-[10px] font-bold border border-black rounded px-2 py-0.5 bg-white">
              {installStateLabel(selectedModel.install_state)}
            </span>
          </div>

          {selectedProgress && selectedProgress.total_bytes > 0 && (
            <div className="space-y-1" aria-label="Local TTS download progress">
              <div className="h-2 border border-black bg-white">
                <div
                  className="h-full bg-black"
                  style={{
                    width: `${Math.min(
                      100,
                      Math.round(
                        (selectedProgress.downloaded_bytes /
                          selectedProgress.total_bytes) *
                          100,
                      ),
                    )}%`,
                  }}
                />
              </div>
              <div className="text-[10px] text-gray-700">
                {selectedProgress.install_state === "verifying"
                  ? "Verifying SHA-256 and byte counts…"
                  : selectedProgress.install_state === "promoting"
                    ? "Finalizing verified installation…"
                    : `${formatModelSize(selectedProgress.downloaded_bytes)} / ${formatModelSize(selectedProgress.total_bytes)}`}
              </div>
            </div>
          )}

          {selectedModel.error && (
            <div className="text-[10px] text-red-800" role="alert">
              <span className="font-bold">{selectedModel.error.kind}: </span>
              {selectedModel.error.message}
            </div>
          )}

          <div className="flex flex-wrap gap-2">
            {selectedModel.install_state === "installed" ? (
              <button
                type="button"
                onClick={() => void handleDelete()}
                disabled={deletePending || selectionPending}
                className="px-3 py-1.5 bg-white border border-black rounded font-bold disabled:opacity-60 flex items-center gap-1"
              >
                <Trash2 className="w-3.5 h-3.5" />
                {deletePending ? "Deleting…" : "Delete Model"}
              </button>
            ) : isInstallActive ? (
              <button
                type="button"
                onClick={() => void handleCancel()}
                className="px-3 py-1.5 bg-white border border-black rounded font-bold flex items-center gap-1"
              >
                <X className="w-3.5 h-3.5" />
                Cancel Download
              </button>
            ) : (
              <button
                type="button"
                onClick={() => void handleInstall()}
                disabled={status !== "ready" || selectionPending}
                className="px-3 py-1.5 bg-black text-white rounded font-bold disabled:opacity-60 flex items-center gap-1"
              >
                <Download className="w-3.5 h-3.5" />
                {selectedModel.install_state === "failed"
                  ? "Retry Download & Verify"
                  : "Download & Verify"}
              </button>
            )}
          </div>
        </div>
      )}

      {status === "ready" && !selectedModel && (
        <div
          className="p-2 border border-red-600 bg-red-50 text-red-800 rounded"
          role="alert"
        >
          The selected Local TTS model is not in the supported backend catalog.
          No replacement was selected automatically.
        </div>
      )}

      {result && (
        <div
          className={`p-2 rounded border flex items-center gap-2 ${
            result.success
              ? "bg-green-50 border-green-600 text-green-800"
              : "bg-red-50 border-red-600 text-red-800"
          }`}
          role="status"
        >
          {result.success ? (
            <CheckCircle className="w-4 h-4 flex-shrink-0" />
          ) : (
            <AlertCircle className="w-4 h-4 flex-shrink-0" />
          )}
          <span>{result.message}</span>
        </div>
      )}
    </div>
  );
};
