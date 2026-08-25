import React, { useEffect, useState } from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { tauriBridge } from "../../lib/tauriBridge";
import type { AsrModelDescriptor } from "../../types/moose";
import {
  ArrowRight,
  CheckCircle,
  Download,
  Key,
  Mic,
  Shield,
  Sparkles,
  X,
} from "lucide-react";

const TINY_MODE = "moonshine_tiny_streaming" as const;

export const OnboardingModal: React.FC = () => {
  const {
    isOnboardingOpen,
    toggleOnboarding,
    triggerCanned,
    settings,
    saveGoogleApiKey,
  } = useMooseStore();
  const [step, setStep] = useState(1);
  const [apiKey, setApiKey] = useState("");
  const [keyStatus, setKeyStatus] = useState<string | null>(null);
  const [tinyModel, setTinyModel] = useState<AsrModelDescriptor | null>(null);
  const [isInstallingTiny, setIsInstallingTiny] = useState(false);
  const [modelStatus, setModelStatus] = useState<string | null>(null);
  const [completionStatus, setCompletionStatus] = useState<string | null>(null);

  useEffect(() => {
    if (!isOnboardingOpen) return;
    void tauriBridge
      .getAsrModels()
      .then((models) => {
        setTinyModel(models.find((model) => model.mode === TINY_MODE) ?? null);
      })
      .catch((error) =>
        setModelStatus(`Could not inspect local models: ${error}`),
      );
  }, [isOnboardingOpen]);

  const asrMode = settings?.asr_mode ?? TINY_MODE;
  const selectedAsrName =
    asrMode === "moonshine_small_streaming"
      ? "Moonshine Small Streaming"
      : asrMode === "gemini_live_audio"
        ? "Gemini Live Cloud Audio"
        : "Moonshine Tiny Streaming";

  if (!isOnboardingOpen) {
    return null;
  }

  const handleInstallTiny = async () => {
    setIsInstallingTiny(true);
    setModelStatus(null);
    try {
      const installed = await tauriBridge.installAsrModel(TINY_MODE);
      setTinyModel(installed);
      setModelStatus("Moonshine Tiny is ready for local speech recognition.");
    } catch (error) {
      setModelStatus(`Tiny model download failed: ${error}`);
    } finally {
      setIsInstallingTiny(false);
    }
  };

  const handleSaveKey = async () => {
    if (!apiKey.trim()) return;
    setKeyStatus(null);
    try {
      await saveGoogleApiKey(apiKey.trim());
      setApiKey("");
      setKeyStatus("Key saved securely.");
    } catch (error) {
      setKeyStatus(`Could not save key: ${error}`);
    }
  };

  const closeAfterAcknowledgement = async (greet: boolean) => {
    setCompletionStatus(null);
    try {
      await tauriBridge.acknowledgeOnboarding();
      toggleOnboarding(false);
      if (greet) {
        await triggerCanned("greeting");
      }
    } catch (error) {
      setCompletionStatus(`Could not save onboarding completion: ${error}`);
    }
  };

  const handleFinish = async () => {
    await closeAfterAcknowledgement(true);
  };

  return (
    <div
      data-testid="onboarding-modal"
      className="absolute inset-0 z-50 bg-black/70 backdrop-blur-xs flex items-center justify-center p-2 animate-in fade-in duration-150 select-none font-mono text-xs"
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="onboarding-title"
        className="bg-[#ece7de] w-full max-h-[96%] border-2 border-black rounded shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] p-3.5 flex flex-col justify-between overflow-y-auto space-y-3"
      >
        <div className="flex items-center justify-between border-b border-black pb-1.5">
          <div>
            <h2 id="onboarding-title" className="font-bold text-xs">
              TALKING MOOSE AI
            </h2>
            <p className="text-gray-700 text-[10px]">
              1986 Macintosh Reimagined
            </p>
          </div>
          <button
            type="button"
            onClick={() => void closeAfterAcknowledgement(false)}
            className="p-1 hover:bg-gray-300 rounded border border-black"
            title="Skip Onboarding"
            aria-label="Skip onboarding"
          >
            <X className="w-3 h-3" aria-hidden="true" />
          </button>
        </div>

        {completionStatus ? <p role="status">{completionStatus}</p> : null}

        {step === 1 && (
          <div className="space-y-2.5">
            <div className="flex items-center gap-1.5 font-bold text-xs text-blue-900">
              <Sparkles className="w-3.5 h-3.5" aria-hidden="true" />
              <span>Lives on Your Desktop</span>
            </div>
            <p className="text-gray-900 leading-snug text-[11px]">
              The Moose is a dry-witted desktop companion. He can make ambient
              remarks and hold spoken conversations, but microphone capture does
              not run continuously in the background.
            </p>
            <div className="p-2 bg-white border border-black rounded text-[10px] text-gray-800">
              Conservative defaults: active-app observation, cross-conversation
              memory, and transcript retention start Off. You can enable them
              later from Privacy Settings.
            </div>
            <button
              type="button"
              onClick={() => setStep(2)}
              className="w-full mt-2 py-1.5 bg-black text-white rounded font-bold hover:bg-gray-800 flex items-center justify-center gap-1 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5"
            >
              <span>Next: Voice</span>
              <ArrowRight className="w-3.5 h-3.5" aria-hidden="true" />
            </button>
          </div>
        )}

        {step === 2 && (
          <div className="space-y-2.5">
            <div className="flex items-center gap-1.5 font-bold text-xs text-green-900">
              <Mic className="w-3.5 h-3.5" aria-hidden="true" />
              <span>Microphone Lifecycle & ASR Privacy</span>
            </div>
            <p className="text-gray-900 leading-snug text-[11px]">
              Click the Moose to start a conversation. The microphone opens only
              for that active conversation, or for an explicit microphone test
              you start in Settings, and closes when the session stops.
            </p>
            <p className="text-[11px]">
              Current mode: <strong>{selectedAsrName}</strong>
            </p>
            <div className="p-2 bg-white border border-black rounded text-[10px] text-gray-800 space-y-1.5">
              <p>
                <strong>Moonshine Tiny Streaming:</strong> processes microphone
                audio locally, so microphone audio stays on this computer. Only
                finalized transcript text is sent to Gemini for the Moose&apos;s
                reply. It is the smallest local model.
              </p>
              <p>
                <strong>Moonshine Small Streaming:</strong> the same local-audio
                privacy boundary, with a larger local model for higher accuracy
                at greater disk/CPU cost.
              </p>
              <p>
                <strong>Gemini Live Cloud Audio:</strong> sends microphone audio
                to Google during the active conversation so Gemini performs
                speech recognition and dialogue in the cloud.
              </p>
              <p>
                Local ASR errors never automatically switch to cloud microphone
                upload. Changing to Gemini Live requires an explicit setting.
              </p>
            </div>
            <button
              type="button"
              onClick={() => setStep(3)}
              className="w-full mt-2 py-1.5 bg-black text-white rounded font-bold hover:bg-gray-800 flex items-center justify-center gap-1 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5"
            >
              <span>Next: Local Model</span>
              <ArrowRight className="w-3.5 h-3.5" aria-hidden="true" />
            </button>
          </div>
        )}

        {step === 3 && (
          <div className="space-y-2.5">
            <div className="flex items-center gap-1.5 font-bold text-xs text-indigo-900">
              <Download className="w-3.5 h-3.5" aria-hidden="true" />
              <span>Optional Moonshine Tiny Download</span>
            </div>
            <p className="text-gray-900 leading-snug text-[11px]">
              Tiny is the recommended privacy-first starting point. Downloading
              it is optional; onboarding will continue even if you skip it.
            </p>
            <div className="p-2 bg-white border border-black rounded text-[10px] text-gray-800 space-y-1">
              <p>
                Status:{" "}
                <strong>{tinyModel?.install_state ?? "checking"}</strong>
              </p>
              {tinyModel?.expected_bytes ? (
                <p>
                  Expected download: about{" "}
                  {Math.ceil(tinyModel.expected_bytes / 1_000_000)} MB.
                </p>
              ) : null}
              {modelStatus ? <p role="status">{modelStatus}</p> : null}
            </div>
            <button
              type="button"
              onClick={() => void handleInstallTiny()}
              disabled={
                isInstallingTiny ||
                tinyModel?.install_state === "installed" ||
                tinyModel?.active === true
              }
              className="w-full py-1.5 bg-white border border-black font-bold rounded hover:bg-gray-100 disabled:opacity-60"
            >
              {isInstallingTiny
                ? "Downloading Tiny…"
                : tinyModel?.install_state === "installed" ||
                    tinyModel?.active === true
                  ? "Tiny Model Ready ✓"
                  : "Download Tiny Model"}
            </button>
            <button
              type="button"
              onClick={() => setStep(4)}
              className="w-full py-1.5 bg-black text-white rounded font-bold hover:bg-gray-800 flex items-center justify-center gap-1 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5"
            >
              <span>Continue to Gemini Key</span>
              <ArrowRight className="w-3.5 h-3.5" aria-hidden="true" />
            </button>
          </div>
        )}

        {step === 4 && (
          <div className="space-y-2.5">
            <div className="flex items-center gap-1.5 font-bold text-xs text-amber-900">
              <Key className="w-3.5 h-3.5" aria-hidden="true" />
              <span>Gemini AI Studio Key</span>
            </div>
            <p className="text-gray-900 leading-snug text-[11px]">
              Enter your Google AI Studio API key, or skip this step and use
              offline canned phrases until you configure Gemini later.
            </p>
            <div className="flex gap-1.5 p-2 bg-white border border-black rounded text-[10px] text-gray-800">
              <Shield
                className="w-3.5 h-3.5 flex-shrink-0"
                aria-hidden="true"
              />
              <p>
                The key is never stored in the app settings database. On macOS
                it is stored in Keychain; development builds on other platforms
                keep it only in process memory.
              </p>
            </div>

            <div className="space-y-1">
              <label htmlFor="onboarding-google-key" className="sr-only">
                Google Gemini API key
              </label>
              <input
                id="onboarding-google-key"
                type="password"
                autoComplete="off"
                placeholder="AIzaSy... API Key"
                value={apiKey}
                onChange={(event) => setApiKey(event.target.value)}
                className="w-full p-1.5 border border-black rounded bg-white text-[11px] select-text"
              />
              {apiKey.trim() && (
                <button
                  type="button"
                  onClick={() => void handleSaveKey()}
                  className="w-full py-1 bg-white border border-black font-bold rounded hover:bg-gray-100"
                >
                  Save Key Securely
                </button>
              )}
              {keyStatus ? <p role="status">{keyStatus}</p> : null}
            </div>

            <button
              type="button"
              onClick={() => void handleFinish()}
              className="w-full mt-2 py-1.5 bg-black text-white rounded font-bold hover:bg-gray-800 flex items-center justify-center gap-1 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5"
            >
              <CheckCircle className="w-3.5 h-3.5" aria-hidden="true" />
              <span>Meet the Moose!</span>
            </button>
          </div>
        )}
      </section>
    </div>
  );
};
