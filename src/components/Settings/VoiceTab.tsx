import React, { useEffect, useMemo, useState } from "react";
import { AlertCircle, Volume2 } from "lucide-react";
import { useMooseStore } from "../../stores/mooseStore";
import { tauriBridge } from "../../lib/tauriBridge";
import type { TtsCatalog, TtsProvider } from "../../types/moose";
import { LocalTtsSettingsPanel } from "./LocalTtsSettingsPanel";

// Owned by SettingsModalBase so the audition debounce survives a tab switch;
// held locally it would reset on unmount and allow overlapping playback.
interface VoiceTabProps {
  isAuditioning: boolean;
  setIsAuditioning: (value: boolean) => void;
}

const providerLabel = (provider: TtsProvider): string =>
  provider === "google"
    ? "Google Gemini TTS — cloud"
    : "Local KittenTTS — on this computer";

export const VoiceTab: React.FC<VoiceTabProps> = ({
  isAuditioning,
  setIsAuditioning,
}) => {
  const {
    settings,
    updateSettingsPatch,
    updateSettingsContinuousPatch,
    inputDevices,
    outputDevices,
    hasApiKey,
    isConversationActive,
  } = useMooseStore();
  const [catalog, setCatalog] = useState<TtsCatalog | null>(null);
  const [catalogError, setCatalogError] = useState<string | null>(null);
  const [localModelInstalled, setLocalModelInstalled] = useState(false);
  const [voiceError, setVoiceError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void tauriBridge
      .getTtsCatalog()
      .then((nextCatalog) => {
        if (!active) return;
        setCatalog(nextCatalog);
        setCatalogError(null);
      })
      .catch(() => {
        if (!active) return;
        setCatalog(null);
        setCatalogError("Speech provider catalog is unavailable.");
      });
    return () => {
      active = false;
    };
  }, []);

  const selectedProvider = useMemo(
    () =>
      catalog?.providers.find(
        (provider) => provider.id === settings?.tts_provider,
      ) ?? null,
    [catalog, settings?.tts_provider],
  );

  if (!settings) return null;

  const selectedVoice =
    settings.tts_provider === "google"
      ? settings.google_tts_voice
      : settings.local_tts_voice;
  const selectedVoiceAvailable =
    selectedProvider?.voices.some((voice) => voice.id === selectedVoice) ??
    false;
  const liveVoiceAvailable =
    catalog?.gemini_live.voices.some(
      (voice) => voice.id === settings.live_voice,
    ) ?? false;
  const auditionBlockedReason =
    settings.tts_provider === "local" && !localModelInstalled
      ? "Download and verify the Local KittenTTS model before auditioning a Local voice."
      : settings.tts_provider === "google" && !hasApiKey
        ? "Google TTS requires a saved Google API key in AI & Models."
        : null;

  const selectProvider = async (provider: TtsProvider) => {
    if (provider === settings.tts_provider) return;
    setVoiceError(null);
    try {
      // Standalone and Gemini Live share physical playback. Never flush an active
      // Live session just because the future standalone provider changed.
      if (!isConversationActive) {
        await tauriBridge.cancelStandaloneSpeech();
      }
      await updateSettingsPatch({ tts_provider: provider });
    } catch (error) {
      setVoiceError(
        `Could not switch standalone speech provider: ${String(error)}`,
      );
    }
  };

  const selectStandaloneVoice = async (voice: string) => {
    setVoiceError(null);
    if (!isConversationActive) {
      try {
        await tauriBridge.cancelStandaloneSpeech();
      } catch (error) {
        setVoiceError(
          `Could not preempt the previous standalone voice: ${String(error)}`,
        );
        return;
      }
    }
    if (settings.tts_provider === "google") {
      await updateSettingsPatch({ google_tts_voice: voice });
    } else {
      await updateSettingsPatch({ local_tts_voice: voice });
    }
  };

  const auditionSelectedVoice = async () => {
    if (auditionBlockedReason) return;
    setIsAuditioning(true);
    setVoiceError(null);
    try {
      await tauriBridge.auditionTtsVoice(settings.tts_provider, selectedVoice);
    } catch (error) {
      setVoiceError(
        settings.tts_provider === "local"
          ? `Local KittenTTS audition failed: ${String(error)}`
          : `Google TTS audition failed: ${String(error)}`,
      );
    } finally {
      setTimeout(() => setIsAuditioning(false), 2500);
    }
  };

  return (
    <div className="space-y-5">
      <h3 className="font-bold text-sm border-b border-black pb-1">
        Audio & Speech Synthesis
      </h3>

      <div>
        <label htmlFor="settings-input-device" className="block mb-1 font-bold">
          Microphone Input Device
        </label>
        <select
          id="settings-input-device"
          value={settings.input_device || ""}
          onChange={(event) =>
            void updateSettingsPatch({
              input_device: event.target.value || null,
            })
          }
          className="w-full p-1.5 border border-black rounded bg-white"
        >
          <option value="">Default Microphone</option>
          {inputDevices.map((device) => (
            <option key={String(device.id)} value={String(device.id)}>
              {device.name}
            </option>
          ))}
        </select>
      </div>

      <div>
        <label
          htmlFor="settings-output-device"
          className="block mb-1 font-bold"
        >
          Audio Output Device
        </label>
        <select
          id="settings-output-device"
          value={settings.output_device || ""}
          onChange={(event) =>
            void updateSettingsPatch({
              output_device: event.target.value || null,
            })
          }
          className="w-full p-1.5 border border-black rounded bg-white"
        >
          <option value="">Default Speakers</option>
          {outputDevices.map((device) => (
            <option key={String(device.id)} value={String(device.id)}>
              {device.name}
            </option>
          ))}
        </select>
      </div>

      <section
        className="pt-2 border-t border-gray-300 space-y-3"
        aria-labelledby="standalone-speech-heading"
      >
        <div>
          <h4 id="standalone-speech-heading" className="font-bold">
            Standalone Speech
          </h4>
          <p className="text-[10px] text-gray-700">
            This provider speaks typed replies, ambient remarks, canned
            reactions, and voice auditions. It does not change Gemini Live
            conversations.
          </p>
        </div>

        <fieldset className="space-y-2">
          <legend className="font-bold mb-1">Standalone Speech Provider</legend>
          {(["google", "local"] as const).map((provider) => (
            <label
              key={provider}
              className="flex items-start gap-2 p-2 border border-gray-400 rounded cursor-pointer"
            >
              <input
                type="radio"
                name="tts-provider"
                value={provider}
                checked={settings.tts_provider === provider}
                onChange={() => void selectProvider(provider)}
              />
              <span>
                <span className="font-bold">{providerLabel(provider)}</span>
                <span className="block text-[10px] text-gray-700">
                  {provider === "google"
                    ? "Standalone speech text is sent to Google and requires your saved API key."
                    : "After an explicit model download, synthesis runs on this computer and stays offline."}
                </span>
              </span>
            </label>
          ))}
        </fieldset>

        {catalogError && (
          <div className="text-[10px] text-red-800" role="alert">
            {catalogError}
          </div>
        )}

        {selectedProvider && (
          <div>
            <label
              htmlFor="settings-standalone-voice"
              className="block mb-1 font-bold"
            >
              {settings.tts_provider === "google"
                ? "Google Standalone Voice"
                : "Local KittenTTS Voice"}
            </label>
            <select
              id="settings-standalone-voice"
              value={selectedVoice}
              onChange={(event) =>
                void selectStandaloneVoice(event.target.value)
              }
              className="w-full p-1.5 border border-black rounded bg-white font-bold"
            >
              {!selectedVoiceAvailable && (
                <option value={selectedVoice} disabled>
                  Unavailable: {selectedVoice}
                </option>
              )}
              {selectedProvider.voices.map((voice) => (
                <option key={voice.id} value={voice.id}>
                  {voice.display_name}
                  {voice.style ? ` (${voice.style})` : ""}
                </option>
              ))}
            </select>
          </div>
        )}

        {settings.tts_provider === "local" && (
          <LocalTtsSettingsPanel
            selectedModelId={settings.local_tts_model}
            onSelectModel={async (modelId) => {
              if (!isConversationActive) {
                await tauriBridge.cancelStandaloneSpeech();
              }
              await updateSettingsPatch({ local_tts_model: modelId });
            }}
            onInstalledChange={setLocalModelInstalled}
          />
        )}

        <div>
          <label
            htmlFor="settings-speaking-rate"
            className="block mb-1 font-bold"
          >
            Speaking Rate: {settings.speaking_rate.toFixed(2)}×
          </label>
          <input
            id="settings-speaking-rate"
            type="range"
            min="0.25"
            max="4"
            step="0.05"
            value={settings.speaking_rate}
            disabled={!selectedProvider?.supports_speaking_rate}
            onChange={(event) =>
              updateSettingsContinuousPatch({
                speaking_rate: Number(event.target.value),
              })
            }
            className="w-full"
          />
          <p className="text-[10px] text-gray-700">
            {settings.tts_provider === "google"
              ? "Google receives this as performance pacing guidance for standalone speech."
              : "KittenTTS uses this as the model synthesis speed setting."}
          </p>
        </div>

        {settings.tts_provider === "google" ? (
          <div>
            <label htmlFor="settings-pitch" className="block mb-1 font-bold">
              Google Pitch Guidance: {settings.pitch.toFixed(1)}
            </label>
            <input
              id="settings-pitch"
              type="range"
              min="-20"
              max="20"
              step="0.5"
              value={settings.pitch}
              disabled={!selectedProvider?.supports_pitch}
              onChange={(event) =>
                updateSettingsContinuousPatch({
                  pitch: Number(event.target.value),
                })
              }
              className="w-full"
            />
            <p className="text-[10px] text-gray-700">
              This is natural-language performance guidance for Google TTS.
            </p>
          </div>
        ) : (
          <div className="p-2 border border-gray-400 rounded bg-gray-50 text-[10px] text-gray-700">
            KittenTTS Mini does not expose a truthful pitch control. Your saved
            Google pitch preference ({settings.pitch.toFixed(1)}) is preserved
            and will be used again if you switch back to Google.
          </div>
        )}

        {auditionBlockedReason && (
          <div className="text-[10px] text-amber-800" role="status">
            {auditionBlockedReason}
          </div>
        )}
        {voiceError && (
          <div
            className="p-2 border border-red-600 bg-red-50 text-red-800 rounded flex items-start gap-2"
            role="alert"
          >
            <AlertCircle className="w-4 h-4 flex-shrink-0" />
            <span>{voiceError}</span>
          </div>
        )}
        <button
          type="button"
          onClick={() => void auditionSelectedVoice()}
          disabled={
            isAuditioning || Boolean(auditionBlockedReason) || !selectedProvider
          }
          className="px-3 py-1.5 bg-white border-2 border-black rounded font-bold hover:bg-gray-100 flex items-center gap-1.5 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5 disabled:opacity-60"
        >
          <Volume2 className="w-3.5 h-3.5" />
          <span>
            {isAuditioning
              ? "Playing Sample..."
              : `Audition ${providerLabel(settings.tts_provider)} — “${selectedVoice}”`}
          </span>
        </button>
      </section>

      <section
        className="pt-3 border-t border-gray-300 space-y-2"
        aria-labelledby="live-voice-heading"
      >
        <div>
          <h4 id="live-voice-heading" className="font-bold">
            Gemini Live Conversation Voice — cloud
          </h4>
          <p className="text-[10px] text-gray-700">
            This separate voice belongs only to realtime Gemini Live sessions.
            Standalone Google/Local voice changes do not alter it.
          </p>
        </div>
        <label htmlFor="settings-live-voice" className="block mb-1 font-bold">
          Gemini Live Voice
        </label>
        <select
          id="settings-live-voice"
          value={settings.live_voice}
          disabled={!catalog}
          onChange={(event) =>
            void updateSettingsPatch({ live_voice: event.target.value })
          }
          className="w-full p-1.5 border border-black rounded bg-white font-bold disabled:opacity-60"
        >
          {!liveVoiceAvailable && (
            <option value={settings.live_voice} disabled>
              Unavailable: {settings.live_voice}
            </option>
          )}
          {catalog?.gemini_live.voices.map((voice) => (
            <option key={voice.id} value={voice.id}>
              {voice.display_name}
              {voice.style ? ` (${voice.style})` : ""}
            </option>
          ))}
        </select>
      </section>
    </div>
  );
};
