import React, { useState } from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { tauriBridge } from "../../lib/tauriBridge";
import { Sparkles, Mic, Key, CheckCircle, ArrowRight, X } from "lucide-react";

export const OnboardingModal: React.FC = () => {
  const { isOnboardingOpen, toggleOnboarding, triggerCanned, settings } =
    useMooseStore();
  const [step, setStep] = useState(1);
  const [apiKey, setApiKey] = useState("");
  const [tested, setTested] = useState(false);

  const asrMode = settings?.asr_mode ?? "moonshine_tiny_streaming";
  const localAsr = asrMode !== "gemini_live_audio";
  const selectedAsrName =
    asrMode === "moonshine_small_streaming"
      ? "Moonshine Small Streaming"
      : asrMode === "gemini_live_audio"
        ? "Gemini Live Cloud Audio"
        : "Moonshine Tiny Streaming";

  if (!isOnboardingOpen) {
    return null;
  }

  const handleSaveKey = async () => {
    if (apiKey.trim()) {
      await tauriBridge.setGoogleApiKey(apiKey.trim());
      setTested(true);
    }
  };

  const handleFinish = async () => {
    toggleOnboarding(false);
    await triggerCanned("greeting");
  };

  return (
    <div
      data-testid="onboarding-modal"
      className="absolute inset-0 z-50 bg-black/70 backdrop-blur-xs flex items-center justify-center p-2 animate-in fade-in duration-150 select-none font-mono text-xs"
    >
      <div className="bg-[#ece7de] w-full max-h-[96%] border-2 border-black rounded shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] p-3.5 flex flex-col justify-between overflow-y-auto space-y-3">
        {/* Title bar */}
        <div className="flex items-center justify-between border-b border-black pb-1.5">
          <div>
            <h2 className="font-bold text-xs">TALKING MOOSE AI</h2>
            <p className="text-gray-600 text-[10px]">
              1986 Macintosh Reimagined
            </p>
          </div>
          <button
            onClick={() => toggleOnboarding(false)}
            className="p-1 hover:bg-gray-300 rounded border border-black"
            title="Skip Onboarding"
          >
            <X className="w-3 h-3" />
          </button>
        </div>

        {/* Step 1: Intro */}
        {step === 1 && (
          <div className="space-y-2.5">
            <div className="flex items-center gap-1.5 font-bold text-xs text-blue-900">
              <Sparkles className="w-3.5 h-3.5" />
              <span>Lives on Your Desktop</span>
            </div>
            <p className="text-gray-800 leading-snug text-[11px]">
              The Moose is a classic dry-witted cartoon companion. He lives
              quietly in your desktop window, making witty observations and
              holding spoken conversations.
            </p>
            <button
              onClick={() => setStep(2)}
              className="w-full mt-2 py-1.5 bg-black text-white rounded font-bold hover:bg-gray-800 flex items-center justify-center gap-1 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5"
            >
              <span>Next: Voice</span>
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        {/* Step 2: Microphone */}
        {step === 2 && (
          <div className="space-y-2.5">
            <div className="flex items-center gap-1.5 font-bold text-xs text-green-900">
              <Mic className="w-3.5 h-3.5" />
              <span>Click & Talk</span>
            </div>
            <p className="text-gray-800 leading-snug text-[11px]">
              Click the Moose to speak with him. Your selected speech
              recognition mode is <strong>{selectedAsrName}</strong>. You can
              interrupt him whenever he is talking.
            </p>
            <div className="p-2 bg-white border border-black rounded text-[10px] text-gray-700 space-y-1">
              <p>
                Microphone is only active when you click to talk. No hidden
                continuous recording.
              </p>
              {localAsr ? (
                <p>
                  Moonshine processes microphone audio locally. Only finalized
                  transcript text is sent to Gemini to generate the Moose&apos;s
                  reply.
                </p>
              ) : (
                <p>
                  Gemini Live Cloud Audio sends microphone audio to Google
                  during the active conversation.
                </p>
              )}
              <p>
                Local ASR errors never automatically switch to cloud microphone
                upload.
              </p>
            </div>
            <button
              onClick={() => setStep(3)}
              className="w-full mt-2 py-1.5 bg-black text-white rounded font-bold hover:bg-gray-800 flex items-center justify-center gap-1 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5"
            >
              <span>Next: Gemini API Key</span>
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        {/* Step 3: Google AI Key */}
        {step === 3 && (
          <div className="space-y-2.5">
            <div className="flex items-center gap-1.5 font-bold text-xs text-amber-900">
              <Key className="w-3.5 h-3.5" />
              <span>Gemini AI Studio Key</span>
            </div>
            <p className="text-gray-800 leading-snug text-[11px]">
              Enter your Google AI Studio API key (or skip to use offline
              phrases):
            </p>

            <div className="space-y-1">
              <input
                type="password"
                placeholder="AIzaSy... API Key"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                className="w-full p-1.5 border border-black rounded bg-white text-[11px]"
              />
              {apiKey.trim() && (
                <button
                  onClick={handleSaveKey}
                  className="w-full py-1 bg-white border border-black font-bold rounded hover:bg-gray-100"
                >
                  {tested ? "Key Saved ✓" : "Save Key"}
                </button>
              )}
            </div>

            <button
              onClick={handleFinish}
              className="w-full mt-2 py-1.5 bg-black text-white rounded font-bold hover:bg-gray-800 flex items-center justify-center gap-1 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5"
            >
              <CheckCircle className="w-3.5 h-3.5" />
              <span>Meet the Moose!</span>
            </button>
          </div>
        )}
      </div>
    </div>
  );
};
