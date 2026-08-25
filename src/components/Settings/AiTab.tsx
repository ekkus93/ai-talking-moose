import React from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { tauriBridge } from "../../lib/tauriBridge";
import type { GoogleModelDescriptor } from "../../types/moose";
import { CheckCircle, AlertCircle } from "lucide-react";

export interface AiTabTestResult {
  success: boolean;
  message: string;
}

// This tab's transient state (draft key, test result, in-flight flag) is owned by
// SettingsModalBase rather than held locally: tabs unmount when the user switches
// away, which would otherwise discard a half-typed key and drop the result of a
// connection test that was still in flight.
interface AiTabProps {
  googleModels: GoogleModelDescriptor[];
  apiKeyInput: string;
  setApiKeyInput: (value: string) => void;
  testResult: AiTabTestResult | null;
  setTestResult: (value: AiTabTestResult | null) => void;
  isTesting: boolean;
  setIsTesting: (value: boolean) => void;
}

export const AiTab: React.FC<AiTabProps> = ({
  googleModels,
  apiKeyInput,
  setApiKeyInput,
  testResult,
  setTestResult,
  isTesting,
  setIsTesting,
}) => {
  const { settings, updateSettings, saveGoogleApiKey } = useMooseStore();

  if (!settings) return null;

  const handleSaveApiKey = async () => {
    if (apiKeyInput.trim()) {
      await saveGoogleApiKey(apiKeyInput.trim());
      setApiKeyInput("");
      setTestResult({
        success: true,
        message: "API Key updated successfully!",
      });
    }
  };

  const handleTestConnection = async () => {
    setIsTesting(true);
    setTestResult(null);
    try {
      const res = await tauriBridge.testAiConnection();
      setTestResult(res);
    } catch (e) {
      setTestResult({ success: false, message: String(e) });
    } finally {
      setIsTesting(false);
    }
  };

  return (
    <div className="space-y-4">
      <h3 className="font-bold text-sm border-b border-black pb-1">
        Google Gemini AI Configuration
      </h3>

      <div className="space-y-2">
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
            onChange={(e) => setApiKeyInput(e.target.value)}
            className="flex-1 p-1.5 border border-black rounded"
          />
          <button
            onClick={handleSaveApiKey}
            className="px-3 py-1.5 bg-black text-white rounded font-bold hover:bg-gray-800"
          >
            Save Key
          </button>
        </div>
        <p className="text-[10px] text-gray-700">
          Your key is stored in the operating system secure credential store and
          is never returned to this settings screen.
        </p>
      </div>

      <div className="pt-2 border-t border-gray-200">
        <div className="space-y-3 mb-3">
          <div>
            <label
              htmlFor="settings-live-model"
              className="block mb-1 font-bold"
            >
              Realtime Live Voice Model
            </label>
            <select
              id="settings-live-model"
              value={settings.live_model}
              onChange={(e) =>
                updateSettings({ ...settings, live_model: e.target.value })
              }
              className="w-full p-1.5 border border-black rounded bg-white font-bold"
            >
              {googleModels
                .filter((model) => model.capabilities.includes("live_audio"))
                .map((model) => (
                  <option key={model.id} value={model.id}>
                    {model.display_name} ({model.id})
                  </option>
                ))}
            </select>
          </div>

          <div>
            <label
              htmlFor="settings-text-model"
              className="block mb-1 font-bold"
            >
              Text & Ambient Remark Model
            </label>
            <select
              id="settings-text-model"
              value={settings.text_model}
              onChange={(e) =>
                updateSettings({ ...settings, text_model: e.target.value })
              }
              className="w-full p-1.5 border border-black rounded bg-white font-bold"
            >
              {googleModels
                .filter((model) =>
                  model.capabilities.includes("text_generation"),
                )
                .map((model) => (
                  <option key={model.id} value={model.id}>
                    {model.display_name} ({model.id})
                  </option>
                ))}
            </select>
          </div>
        </div>

        <button
          onClick={handleTestConnection}
          disabled={isTesting}
          className="px-3 py-1.5 bg-white border-2 border-black rounded font-bold hover:bg-gray-100 flex items-center gap-1.5"
        >
          <span>
            {isTesting ? "Testing Connection..." : "Test Gemini Connection"}
          </span>
        </button>

        {testResult && (
          <div
            className={`mt-2 p-2 rounded border flex items-center gap-2 ${
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
      </div>
    </div>
  );
};
