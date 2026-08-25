import React, { useEffect, useRef, useState } from "react";
import { Volume2, X } from "lucide-react";
import { tauriBridge } from "../../lib/tauriBridge";
import { useMooseStore } from "../../stores/mooseStore";

export const P6VoiceAcceptancePanel: React.FC = () => {
  const isSettingsOpen = useMooseStore((state) => state.isSettingsOpen);
  const settings = useMooseStore((state) => state.settings);
  const voices = useMooseStore((state) => state.googleTtsVoices);
  const [isOpen, setIsOpen] = useState(false);
  const [selectedVoice, setSelectedVoice] = useState("Fenrir");
  const [isAuditioning, setIsAuditioning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const launcherRef = useRef<HTMLButtonElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const restoreLauncherFocusRef = useRef(false);

  useEffect(() => {
    if (!isSettingsOpen) {
      setIsOpen(false);
      setIsAuditioning(false);
      return;
    }
    setSelectedVoice(settings?.tts_voice ?? "Fenrir");
  }, [isSettingsOpen, settings?.tts_voice]);

  useEffect(() => {
    if (!isOpen) {
      if (restoreLauncherFocusRef.current) {
        launcherRef.current?.focus();
        restoreLauncherFocusRef.current = false;
      }
      return;
    }

    restoreLauncherFocusRef.current = true;
    closeButtonRef.current?.focus();

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopImmediatePropagation();
      setIsOpen(false);
    };

    window.addEventListener("keydown", handleEscape, true);
    return () => window.removeEventListener("keydown", handleEscape, true);
  }, [isOpen]);

  if (!isSettingsOpen) return null;

  if (!isOpen) {
    return (
      <button
        ref={launcherRef}
        type="button"
        onClick={() => setIsOpen(true)}
        className="fixed right-4 bottom-12 z-[60] px-3 py-1.5 bg-black text-white border-2 border-black rounded font-mono text-xs font-bold shadow-[2px_2px_0px_0px_rgba(0,0,0,1)]"
      >
        P6 Voice Acceptance
      </button>
    );
  }

  return (
    <section
      role="dialog"
      aria-labelledby="p6-voice-acceptance-title"
      className="fixed right-4 bottom-12 z-[60] w-80 bg-[#fbf9f5] border-2 border-black rounded p-3 font-mono text-xs shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]"
    >
      <div className="flex items-center justify-between gap-2 mb-2">
        <div
          id="p6-voice-acceptance-title"
          className="flex items-center gap-1.5 font-bold"
        >
          <Volume2 className="w-4 h-4" aria-hidden="true" />
          P6 Voice Acceptance
        </div>
        <button
          ref={closeButtonRef}
          type="button"
          onClick={() => setIsOpen(false)}
          aria-label="Close P6 Voice Acceptance"
          className="p-1 border border-black rounded bg-white"
        >
          <X className="w-3 h-3" aria-hidden="true" />
        </button>
      </div>

      <p className="text-[10px] text-gray-700 mb-2">
        Audition the authoritative Gemini TTS catalog with one fixed Moose
        script. Fenrir remains provisional until the listening pass is recorded.
      </p>

      <label htmlFor="p6-voice-select" className="block font-bold mb-1">
        Voice under test
      </label>
      <select
        id="p6-voice-select"
        aria-label="P6 voice under test"
        value={selectedVoice}
        onChange={(event) => setSelectedVoice(event.target.value)}
        className="w-full p-1.5 border border-black rounded bg-white mb-2"
      >
        {voices.length === 0 ? (
          <option value={selectedVoice}>{selectedVoice}</option>
        ) : (
          voices.map((voice) => (
            <option key={voice.id} value={voice.id}>
              {voice.id} ({voice.style})
              {voice.id === "Fenrir" ? " - Provisional default" : ""}
            </option>
          ))
        )}
      </select>

      <div className="flex gap-2">
        <button
          type="button"
          disabled={isAuditioning}
          onClick={async () => {
            setError(null);
            setIsAuditioning(true);
            let queued = false;
            try {
              await tauriBridge.auditionVoice(selectedVoice);
              queued = true;
            } catch (reason) {
              const message = String(reason);
              if (!message.includes("standalone speech cancelled")) {
                setError(message);
              }
            } finally {
              if (queued) {
                window.setTimeout(() => setIsAuditioning(false), 10_000);
              } else {
                setIsAuditioning(false);
              }
            }
          }}
          className="flex-1 px-2 py-1.5 bg-black text-white border border-black rounded font-bold disabled:opacity-60"
        >
          {isAuditioning ? "Playing Sample..." : `Audition ${selectedVoice}`}
        </button>
        {isAuditioning && (
          <button
            type="button"
            onClick={async () => {
              try {
                await tauriBridge.cancelStandaloneSpeech();
              } catch (reason) {
                setError(String(reason));
              } finally {
                setIsAuditioning(false);
              }
            }}
            className="px-2 py-1.5 bg-white border-2 border-black rounded font-bold"
          >
            Stop Sample
          </button>
        )}
      </div>

      {error && (
        <p role="alert" className="mt-2 text-red-700 text-[10px]">
          Voice audition failed: {error}
        </p>
      )}
    </section>
  );
};
