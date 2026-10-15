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
  openaiKeyInput: string;
  setOpenaiKeyInput: (value: string) => void;
  hasOpenaiKey: boolean;
  setHasOpenaiKey: (value: boolean) => void;
  openaiKeyError: string | null;
  setOpenaiKeyError: (value: string | null) => void;
  serverStatus: string | null;
  setServerStatus: (value: string | null) => void;
  testResult: AiTabTestResult | null;
  setTestResult: (value: AiTabTestResult | null) => void;
  isTesting: boolean;
  setIsTesting: (value: boolean) => void;
}

// Controlled draft that commits on blur and when the tab unmounts, so a model ID
// typed without leaving the field is not lost on a tab switch.
const ExternalModelInput: React.FC<{
  committed: string;
  onCommit: (value: string) => void;
}> = ({ committed, onCommit }) => {
  const [draft, setDraft] = React.useState(committed);
  const latest = React.useRef({ draft, committed, onCommit });
  latest.current = { draft, committed, onCommit };

  const commit = () => {
    const value = latest.current.draft.trim();
    if (!value) {
      setDraft(latest.current.committed);
      return;
    }
    if (value !== latest.current.committed) latest.current.onCommit(value);
  };

  React.useEffect(
    () => () => {
      const {
        draft: pending,
        committed: saved,
        onCommit: save,
      } = latest.current;
      const value = pending.trim();
      if (value && value !== saved) save(value);
    },
    [],
  );

  return (
    <input
      id="external-text-model"
      value={draft}
      maxLength={128}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={commit}
      className="w-full p-1.5 border border-black rounded bg-white"
    />
  );
};

export const AiTab: React.FC<AiTabProps> = ({
  googleModels,
  googleModelsStatus,
  apiKeyInput,
  setApiKeyInput,
  openaiKeyInput,
  setOpenaiKeyInput,
  hasOpenaiKey,
  setHasOpenaiKey,
  openaiKeyError,
  setOpenaiKeyError,
  serverStatus,
  setServerStatus,
  testResult,
  setTestResult,
  isTesting,
  setIsTesting,
}) => {
  const {
    settings,
    updateSettingsPatch,
    saveGoogleApiKey,
    clearGoogleApiKey,
    hasApiKey,
  } = useMooseStore();
  const serverCheckSequence = React.useRef(0);

  if (!settings) return null;

  // The backend chooses Gemini Live only for Gemini Live audio ASR.
  // Google text with local ASR still uses non-Live Talk and standalone TTS.
  const standardTalkSelected = settings.asr_mode !== "gemini_live_audio";
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
    serverCheckSequence.current += 1;
    setServerStatus(null);
    void updateSettingsPatch({ text_provider: provider });
  };

  const handleSaveOpenaiKey = async () => {
    const key = openaiKeyInput.trim();
    if (!key) return;
    setOpenaiKeyError(null);
    try {
      await tauriBridge.setOpenaiApiKey(key);
      setOpenaiKeyInput("");
      setHasOpenaiKey(true);
    } catch {
      setOpenaiKeyError("Could not securely store the OpenAI key.");
    }
  };

  const handleClearOpenaiKey = async () => {
    setOpenaiKeyError(null);
    try {
      await tauriBridge.clearOpenaiApiKey();
      setHasOpenaiKey(false);
    } catch {
      setOpenaiKeyError("Could not clear the OpenAI key.");
    }
  };

  const handleTestServer = async (provider: "ollama" | "llama_server") => {
    const requested = useMooseStore.getState().settings;
    if (!requested || requested.text_provider !== provider) return;
    const address =
      provider === "ollama"
        ? requested.ollama_server_address
        : requested.llama_server_address;
    const model =
      provider === "ollama"
        ? requested.ollama_text_model
        : requested.llama_server_text_model;
    const sequence = ++serverCheckSequence.current;
    const isCurrent = () => {
      const current = useMooseStore.getState().settings;
      return (
        sequence === serverCheckSequence.current &&
        current?.text_provider === provider &&
        (provider === "ollama"
          ? current.ollama_server_address
          : current.llama_server_address) === address &&
        (provider === "ollama"
          ? current.ollama_text_model
          : current.llama_server_text_model) === model
      );
    };
    setServerStatus("Checking loopback server and selected model…");
    try {
      const state = await tauriBridge.testLocalTextServer(provider);
      if (!isCurrent()) return;
      setServerStatus(
        state === "model_ready"
          ? "Connected: selected model is available."
          : state === "unreachable"
            ? "Server is unreachable on the configured loopback address."
            : "Server reachable, but the selected model is not available.",
      );
    } catch {
      if (isCurrent()) {
        setServerStatus("Could not check the selected local server.");
      }
    }
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
            This provider is used for typed replies, unsolicited ambient
            remarks, and non-Live Talk when local ASR is selected.
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
          {(
            [
              [
                "openai",
                "OpenAI — cloud",
                "Sends prompts to OpenAI with your own key.",
              ],
              [
                "ollama",
                "Ollama — loopback",
                "Sends text to an independently managed loopback Ollama server, which may keep its own logs.",
              ],
              [
                "llama_server",
                "llama-server — loopback",
                "Sends text to an independently managed loopback llama.cpp server, which may keep its own logs.",
              ],
            ] as const
          ).map(([id, label, description]) => (
            <label
              key={id}
              className="flex items-start gap-2 p-2 border border-gray-400 rounded cursor-pointer"
            >
              <input
                type="radio"
                name="text-provider"
                value={id}
                checked={settings.text_provider === id}
                onChange={() => selectTextProvider(id)}
              />
              <span>
                <span className="font-bold">{label}</span>
                <span className="block text-[10px] text-gray-700">
                  {description}
                </span>
              </span>
            </label>
          ))}
        </fieldset>
        <p className="text-[10px] text-gray-700">
          A selected provider failure is reported without switching models or
          sending the request to another provider.
        </p>

        {settings.text_provider === "local" ? (
          <LocalLlmSettingsPanel
            selectedModelId={settings.local_text_model}
            onSelectModel={async (modelId) => {
              await updateSettingsPatch({
                local_text_model: modelId,
              });
            }}
          />
        ) : settings.text_provider === "google" ? (
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
                  void updateSettingsPatch({
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
        ) : (
          <div className="space-y-3">
            <label htmlFor="external-text-model" className="block font-bold">
              {settings.text_provider === "openai"
                ? "OpenAI model ID"
                : settings.text_provider === "ollama"
                  ? "Ollama model ID"
                  : "llama-server model ID"}
            </label>
            <ExternalModelInput
              key={settings.text_provider}
              committed={
                settings.text_provider === "openai"
                  ? settings.openai_text_model
                  : settings.text_provider === "ollama"
                    ? settings.ollama_text_model
                    : settings.llama_server_text_model
              }
              onCommit={(value) => {
                serverCheckSequence.current += 1;
                setServerStatus(null);
                const field =
                  settings.text_provider === "openai"
                    ? "openai_text_model"
                    : settings.text_provider === "ollama"
                      ? "ollama_text_model"
                      : "llama_server_text_model";
                void updateSettingsPatch({ [field]: value });
              }}
            />
            {settings.text_provider !== "openai" && (
              <>
                <label
                  htmlFor="external-server-url"
                  className="block font-bold"
                >
                  HTTP loopback server (explicit port required)
                </label>
                <input
                  id="external-server-url"
                  key={settings.text_provider + "-server"}
                  defaultValue={
                    settings.text_provider === "ollama"
                      ? settings.ollama_server_address
                      : settings.llama_server_address
                  }
                  onBlur={(event) => {
                    const value = event.target.value.trim();
                    serverCheckSequence.current += 1;
                    setServerStatus(null);
                    const field =
                      settings.text_provider === "ollama"
                        ? "ollama_server_address"
                        : "llama_server_address";
                    void updateSettingsPatch({ [field]: value });
                  }}
                  className="w-full p-1.5 border border-black rounded bg-white"
                />
                <button
                  type="button"
                  onClick={() =>
                    void handleTestServer(
                      settings.text_provider === "ollama"
                        ? "ollama"
                        : "llama_server",
                    )
                  }
                  className="px-2 py-1 border border-black rounded font-bold"
                >
                  Check Server and Model
                </button>
                {serverStatus && (
                  <p role="status" className="text-[10px]">
                    {serverStatus}
                  </p>
                )}
              </>
            )}
            {settings.text_provider === "openai" && (
              <div className="space-y-2">
                <label
                  htmlFor="settings-openai-api-key"
                  className="block font-bold"
                >
                  OpenAI API key (BYOK)
                </label>
                <div className="flex gap-2">
                  <input
                    id="settings-openai-api-key"
                    type="password"
                    autoComplete="off"
                    value={openaiKeyInput}
                    onChange={(event) => setOpenaiKeyInput(event.target.value)}
                    placeholder="Enter API key"
                    className="flex-1 p-1.5 border border-black rounded"
                  />
                  <button
                    type="button"
                    onClick={() => void handleSaveOpenaiKey()}
                    className="px-2 py-1 bg-black text-white rounded font-bold"
                  >
                    Save OpenAI Key
                  </button>
                </div>
                {hasOpenaiKey ? (
                  <p className="text-[10px]" role="status">
                    OpenAI key saved. The model ID is not verified until a
                    request is made.
                  </p>
                ) : null}
                {hasOpenaiKey ? (
                  <button
                    type="button"
                    onClick={() => void handleClearOpenaiKey()}
                    className="px-2 py-1 border border-black rounded font-bold"
                  >
                    Remove Saved OpenAI Key
                  </button>
                ) : (
                  <p className="text-[10px]">No OpenAI key is configured.</p>
                )}
                {openaiKeyError && <p role="alert">{openaiKeyError}</p>}
                <p className="text-[10px] text-gray-700">
                  macOS stores the key in Keychain. Linux retains it only until
                  this app exits; the key is never written to Settings or
                  SQLite.
                </p>
              </div>
            )}
          </div>
        )}
      </section>

      <section
        className="pt-3 border-t border-gray-300 space-y-2"
        aria-labelledby="voice-conversation-heading"
      >
        <div>
          <h4 id="voice-conversation-heading" className="font-bold">
            {standardTalkSelected
              ? "Talk Conversation — selected text and standalone speech"
              : "Talk Conversation — Google Gemini Live"}
          </h4>
          <p className="text-[10px] text-gray-700">
            {standardTalkSelected
              ? "Non-Live Talk uses the selected text provider and standalone TTS; audio recognition stays on this computer. Cloud providers receive the selected text content."
              : "Gemini Live is a separate Google cloud-audio mode and may send microphone audio to Google."}
          </p>
        </div>
        {!standardTalkSelected && (
          <>
            <label
              htmlFor="settings-live-model"
              className="block mb-1 font-bold"
            >
              Realtime Live Voice Model
            </label>
            <select
              id="settings-live-model"
              value={settings.live_model}
              disabled={modelSelectorsDisabled}
              aria-busy={googleModelsStatus === "loading"}
              onChange={(event) => {
                if (!modelSelectorsDisabled) {
                  void updateSettingsPatch({
                    live_model: event.target.value,
                  });
                }
              }}
              className="w-full p-1.5 border border-black rounded bg-white font-bold disabled:opacity-60"
            >
              {googleModelsStatus === "loading" ? (
                <option value={settings.live_model} disabled>
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
          </>
        )}
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
            {settings.asr_mode === "gemini_live_audio" ||
            settings.text_provider === "google"
              ? "Required for Gemini Live or Google text generation. Google standalone TTS also requires this key when selected."
              : settings.tts_provider === "google"
                ? "Required for the selected Google standalone TTS, not for the selected text provider."
                : "Not required for the selected text provider or standalone TTS. Google cloud features require it only when selected."}
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
          macOS stores this key in Keychain. On Linux, the key is session-only.
          It is never returned to this settings screen.
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
          {isTesting
            ? "Testing Gemini Connection..."
            : "Test Gemini Connection"}
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
          Local-first Talk keeps text and speech on this computer when both the
          text and TTS providers are Local. OpenAI and Google text or TTS send
          content to their respective cloud services when selected. Ollama and
          llama-server use only your configured loopback server. Gemini Live is
          a separate Google cloud-audio mode.
        </p>
      </section>
    </div>
  );
};
