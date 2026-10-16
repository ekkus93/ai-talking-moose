import React, { useEffect, useRef, useState } from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { tauriBridge } from "../../lib/tauriBridge";
import { useModalFocusTrap } from "../../lib/useModalFocusTrap";
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

const WHISPER_MODE = "whisper_small" as const;
type WhisperModel = AsrModelDescriptor | null;

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
  const [whisperModel, setWhisperModel] = useState<WhisperModel>(null);
  const [isInstallingWhisper, setIsInstallingWhisper] = useState(false);
  const [modelStatus, setModelStatus] = useState<string | null>(null);
  const [completionStatus, setCompletionStatus] = useState<string | null>(null);
  const dialogRef = useRef<HTMLElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);

  useModalFocusTrap(isOnboardingOpen, dialogRef, closeButtonRef);

  useEffect(() => {
    if (!isOnboardingOpen) return;
    void tauriBridge
      .getAsrModels()
      .then((models) => {
        const whisper = models.find((model) => model.mode === WHISPER_MODE);
        setWhisperModel(whisper ?? null);
      })
      .catch((error) =>
        setModelStatus(`Could not inspect local models: ${error}`),
      );
  }, [isOnboardingOpen]);

  const asrMode = settings?.asr_mode ?? WHISPER_MODE;
  const isCloudAsr = asrMode === "gemini_live_audio";
  const asrName = isCloudAsr ? "Gemini Live Cloud Audio" : "Whisper Small";

  if (!isOnboardingOpen) {
    return null;
  }

  const handleInstallWhisper = async () => {
    setIsInstallingWhisper(true);
    setModelStatus(null);
    try {
      const installed = await tauriBridge.installAsrModel(WHISPER_MODE);
      setWhisperModel(installed);
      setModelStatus("Whisper Small is ready for local speech recognition.");
    } catch (error) {
      setModelStatus(`Whisper model download failed: ${error}`);
    } finally {
      setIsInstallingWhisper(false);
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
      className="absolute inset-0 z-50 bg-black/70 backdrop-blur-xs flex items-center justify-center p-2 select-none font-mono text-xs"
    >
      <section
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="onboarding-title"
        tabIndex={-1}
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
            ref={closeButtonRef}
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
              Current mode: <strong>{asrName}</strong>
            </p>
            <div className="p-2 bg-white border border-black rounded text-[10px] text-gray-800 space-y-1.5">
              <p>
                <strong>Whisper Small:</strong> local English recognition that
                batches microphone PCM into short transcription windows —
                microphone PCM stays on this computer. Partial transcripts may
                be updated in batched intervals; finalized transcript text is
                processed by the selected conversation provider. The
                whisper-small model is downloaded only when you explicitly
                choose it in AI &amp; Models Settings.
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
              <span>Optional Whisper Small Download</span>
            </div>
            <p className="text-gray-900 leading-snug text-[11px]">
              Whisper Small is the local recognition default. Installing the
              verified model is optional here; onboarding continues if you skip
              it.
            </p>
            <div className="p-2 bg-white border border-black rounded text-[10px] text-gray-800 space-y-1">
              <p>
                Status:{" "}
                <strong>{whisperModel?.install_state ?? "checking"}</strong>
              </p>
              {whisperModel?.expected_bytes ? (
                <p>
                  Expected download: about{" "}
                  {Math.ceil(whisperModel.expected_bytes / 1_000_000)} MB.
                </p>
              ) : null}
              {modelStatus ? <p role="status">{modelStatus}</p> : null}
            </div>
            <button
              type="button"
              onClick={() => void handleInstallWhisper()}
              disabled={
                isInstallingWhisper ||
                whisperModel?.install_state === "installed" ||
                whisperModel?.active === true
              }
              className="w-full py-1.5 bg-white border border-black font-bold rounded hover:bg-gray-100 disabled:opacity-60"
            >
              {isInstallingWhisper
                ? "Downloading Whisper…"
                : whisperModel?.install_state === "installed" ||
                    whisperModel?.active === true
                  ? "Whisper Model Ready ✓"
                  : "Download Whisper Model"}
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
              New profiles use Local text generation by default for Talk, typed
              replies, and ambient remarks. The selected Local text and speech
              models are not downloaded automatically; install them explicitly
              from AI &amp; Models and Voice &amp; Audio Settings.
            </p>
            <p className="text-gray-900 leading-snug text-[11px]">
              A Google AI Studio API key is optional for Local-first Talk. It is
              required for Google text generation, Gemini Live audio ASR, or
              Google TTS when those cloud options are selected.
            </p>
            <p className="text-gray-900 leading-snug text-[11px]">
              On macOS, Voice &amp; Audio can use voices already installed in
              the system through AVSpeechSynthesizer. Speech synthesis stays on
              the Mac; other platforms do not provide a System voice fallback.
            </p>
            <p className="text-gray-900 leading-snug text-[11px]">
              AI &amp; Models can also use OpenAI text generation (cloud, with
              its own OpenAI API key) or an Ollama or llama-server you already
              run on this computer (loopback only; the app never starts a server
              or downloads its models). Each is used only when you select it,
              and a failure never switches to another provider.
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
