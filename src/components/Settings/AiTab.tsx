import React from "react";
import { AlertCircle, CheckCircle } from "lucide-react";
import { tauriBridge } from "../../lib/tauriBridge";
import { useMooseStore } from "../../stores/mooseStore";
import type { GoogleModelDescriptor, TextProvider } from "../../types/moose";
import { LocalLlmSettingsPanel } from "./LocalLlmSettingsPanel";

export interface AiTabTestResult {
  success: boolean;
  message: string;
}

// This tab's transient Google credential/test state is owned by SettingsModalBase rather than
// held locally: tabs unmount when the user switches away, which would otherwise discard a
// half-typed key and drop the result of a connection test that was still in flight.
interface AiTabProps {
  googleModels: GoogleModelDescriptor[];
  googleModelsStatus: "loading" | "ready" | "error";
  apiKeyInput: string;
  setApiKeyInput: (value: string) => void;
  testResult: AiTabTestResult | null;
  setTestResult: (value: AiTabTestResult | null) => void;
  isTesting: boolean;
  setIsTesting: (value: boolean) => void;
}

export const AiTab: React.FC<AiTabProps> = ({
  googleModels,
  googleModelsStatus,
  apiKeyInput,
  setApiKeyInput,
  testResult,
  setTestResult,
  isTesting,
  setIsTesting,
}) => {
  const {
    settings,
    updateSettings,
    saveGoogleApiKey,
    clearGoogleApiKey,
    hasApiKey,
  } = useMooseStore();

  if (!settings) return null;

  const liveModels = googleModels.filter((model) =>
    model.capabilities.includes("live_audio"),
  );
  const textModels = googleModels.filter((model) =>
    model.capabilities.includes("text_generation"),
  );
  const liveModelAvailable = liveModels.some(
    (model) => model.id === settings.live_model,
  );
  const textModelAvailable = textModels.some(
    (model) => model.id === settings.google_text_model,
  );
  const modelSelectorsDisabled = googleModelsStatus !== "ready";

  const selectTextProvider = (provider: TextProvider) => {
    if (provider === settings.text_provider) return;
    void updateSettings({ ...settings, text_provider: provider });
  };

  const handleSaveApiKey = async () => {
    const trimmed = apiKeyInput.trim();
    if (!trimmed) return;
    setTestResult(null);
    try {
      await saveGoogleApiKey(trimmed);
      setApiKeyInput("");
      setTestResult({
        success: true,
        message: "Google API key updated successfully.",
      });
    } catch (error) {
      setTestResult({ success: false, message: String(error) });
    }
  };

  const handleClearApiKey = async () => {
    setTestResult(null);
    try {
      await clearGoogleApiKey();
      setTestResult({
        success: true,
        message: "Saved Google API key removed.",
      });
    } catch (error) {
      setTestResult({ success: false, message: String(error) });
    }
  };

  const handleTestConnection = async () => {
    setIsTesting(true);
    setTestResult(null);
    try {
      const res = await tauriBridge.testAiConnection();
      setTestResult(res);
    } catch (error) {
      setTestResult({ success: false, message: String(error) });
    } finally {
      setIsTesting(false);
    }
  };

  return (
    <div className="space-y-5">
      <h3 className="font-bold text-sm border-b border-black pb-1">
        AI Text & Voice Configuration
      </h3>

      <section className="space-y-3" aria-labelledby="text-generation-heading">
        <div>
          <h4 id="text-generation-heading" className="font-bold">
            Text Generation
          </h4>
          <p className="text-[10px] text-gray-700">
            This provider is used for typed replies and unsolicited ambient
            remarks.
          </p>
        </div>

        <fieldset className="space-y-2">
          <legend className="font-bold mb-1">Text Generation Provider</legend>
          <label className="flex items-start gap-2 p-2 border border-gray-400 rounded cursor-pointer">
            <input
              type="radio"
              name="text-provider"
              value="local"
              checked={settings.text_provider === "local"}
              onChange={() => selectTextProvider("local")}
            />
            <span>
              <span className="font-bold">Local — on this computer</span>
              <span className="block text-[10px] text-gray-700">
                Runs after you explicitly download a supported GGUF model.
                Generation itself does not require a Google API key.
              </span>
            </span>
          </label>
          <label className="flex items-start gap-2 p-2 border border-gray-400 rounded cursor-pointer">
            <input
              type="radio"
              name="text-provider"
              value="google"
              checked={settings.text_provider === "google"}
              onChange={() => selectTextProvider("google")}
            />
            <span>
              <span className="font-bold">Google Gemini — cloud</span>
              <span className="block text-[10px] text-gray-700">
                Sends text-generation requests to Google using your saved API
                key.
              </span>
            </span>
          </label>
        </fieldset>

        {settings.text_provider === "local" ? (
          <LocalLlmSettingsPanel
            selectedModelId={settings.local_text_model}
            onSelectModel={async (modelId) => {
              await updateSettings({
                ...settings,
                local_text_model: modelId,
              });
            }}
          />
        ) : (
          <div>
            <label
              htmlFor="settings-text-model"
              className="block mb-1 font-bold"
            >
              Gemini Text & Ambient Remark Model
            </label>
            <select
              id="settings-text-model"
              value={settings.google_text_model}
              disabled={modelSelectorsDisabled}
              aria-busy={googleModelsStatus === "loading"}
              onChange={(event) => {
                if (!modelSelectorsDisabled) {
                  void updateSettings({
                    ...settings,
                    google_text_model: event.target.value,
                  });
                }
              }}
              className="w-full p-1.5 border border-black rounded bg-white font-bold disabled:opacity-60"
            >
              {googleModelsStatus === "loading" ? (
                <option value={settings.google_text_model}>
                  Current: {settings.google_text_model} (loading catalog…)
                </option>
              ) : googleModelsStatus === "error" ? (
                <option value={settings.google_text_model}>
                  Current: {settings.google_text_model} (catalog unavailable)
                </option>
              ) : (
                <>
                  {!textModelAvailable && (
                    <option value={settings.google_text_model} disabled>
                      Unavailable: {settings.google_text_model}
                    </option>
                  )}
                  {textModels.map((model) => (
                    <option key={model.id} value={model.id}>
                      {model.display_name} ({model.id})
                    </option>
                  ))}
                </>
              )}
            </select>
          </div>
        )}
      </section>

      <section
        className="pt-3 border-t border-gray-300 space-y-2"
        aria-labelledby="voice-conversation-heading"
      >
        <div>
          <h4 id="voice-conversation-heading" className="font-bold">
            Voice Conversation — Google Gemini Live
          </h4>
          <p className="text-[10px] text-gray-700">
            Voice sessions remain cloud-based in this phase. Choosing Local text
            above does not change the Gemini Live voice provider.
          </p>
        </div>
        <label htmlFor="settings-live-model" className="block mb-1 font-bold">
          Realtime Live Voice Model
        </label>
        <select
          id="settings-live-model"
          value={settings.live_model}
          disabled={modelSelectorsDisabled}
          aria-busy={googleModelsStatus === "loading"}
          onChange={(event) => {
            if (!modelSelectorsDisabled) {
              void updateSettings({
                ...settings,
                live_model: event.target.value,
              });
            }
          }}
          className="w-full p-1.5 border border-black rounded bg-white font-bold disabled:opacity-60"
        >
          {googleModelsStatus === "loading" ? (
            <option value={settings.live_model}>
              Current: {settings.live_model} (loading catalog…)
            </option>
          ) : googleModelsStatus === "error" ? (
            <option value={settings.live_model}>
              Current: {settings.live_model} (catalog unavailable)
            </option>
          ) : (
            <>
              {!liveModelAvailable && (
                <option value={settings.live_model} disabled>
                  Unavailable: {settings.live_model}
                </option>
              )}
              {liveModels.map((model) => (
                <option key={model.id} value={model.id}>
                  {model.display_name} ({model.id})
                </option>
              ))}
            </>
          )}
        </select>
      </section>

      <section
        className="pt-3 border-t border-gray-300 space-y-2"
        aria-labelledby="google-credential-heading"
      >
        <div>
          <h4 id="google-credential-heading" className="font-bold">
            Google Cloud Credential
          </h4>
          <p className="text-[10px] text-gray-700">
            {settings.text_provider === "local"
              ? "Not required for Local text generation. It is still required for Gemini Live voice and Google TTS."
              : "Required for Gemini text generation, Gemini Live voice, and Google TTS."}
          </p>
        </div>

        <label htmlFor="settings-google-api-key" className="block font-bold">
          Google Gemini API Key (BYOK)
        </label>
        <div className="flex gap-2">
          <input
            id="settings-google-api-key"
            type="password"
            autoComplete="off"
            placeholder="Enter AIzaSy... API Key"
            value={apiKeyInput}
            onChange={(event) => setApiKeyInput(event.target.value)}
            className="flex-1 p-1.5 border border-black rounded"
          />
          <button
            type="button"
            onClick={() => void handleSaveApiKey()}
            className="px-3 py-1.5 bg-black text-white rounded font-bold hover:bg-gray-800"
          >
            Save Key
          </button>
        </div>
        <p className="text-[10px] text-gray-700">
          Your key is stored in the operating system secure credential store and
          is never returned to this settings screen.
        </p>
        {hasApiKey ? (
          <button
            type="button"
            onClick={() => void handleClearApiKey()}
            className="px-3 py-1.5 bg-white border border-black rounded font-bold hover:bg-gray-100"
          >
            Remove Saved Key
          </button>
        ) : (
          <p className="text-[10px] text-gray-600">
            No Google API key is currently saved.
          </p>
        )}

        <button
          type="button"
          onClick={() => void handleTestConnection()}
          disabled={isTesting}
          className="px-3 py-1.5 bg-white border-2 border-black rounded font-bold hover:bg-gray-100 flex items-center gap-1.5 disabled:opacity-60"
        >
          {isTesting ? "Testing Gemini Connection..." : "Test Gemini Connection"}
        </button>

        {testResult && (
          <div
            className={`p-2 rounded border flex items-center gap-2 ${
              testResult.success
                ? "bg-green-50 border-green-600 text-green-800"
                : "bg-red-50 border-red-600 text-red-800"
            }`}
          >
            {testResult.success ? (
              <CheckCircle className="w-4 h-4 text-green-600 flex-shrink-0" />
            ) : (
              <AlertCircle className="w-4 h-4 text-red-600 flex-shrink-0" />
            )}
            <span>{testResult.message}</span>
          </div>
        )}
      </section>

      <section
        className="p-3 border-2 border-black rounded bg-[#f5f1e9] space-y-1"
        aria-label="AI privacy boundary"
      >
        <div className="font-bold">Local / cloud privacy boundary</div>
        <p className="text-[10px] text-gray-800">
          Local text runs on this computer after the model has been downloaded
          and verified. Model installation itself uses the network.
        </p>
        <p className="text-[10px] text-gray-800">
          Gemini Live voice remains cloud-based. Google TTS also receives the
          final reply text for speech, including replies generated by a Local
          text model.
        </p>
      </section>
    </div>
  );
};
