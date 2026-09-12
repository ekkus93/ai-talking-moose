import React, { useEffect, useState } from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { tauriBridge } from "../../lib/tauriBridge";
import type { TtsCatalog, TtsProvider } from "../../types/moose";
import { Volume2 } from "lucide-react";

// Owned by SettingsModalBase so the audition debounce survives a tab switch;
// held locally it would reset on unmount and allow overlapping playback.
interface VoiceTabProps {
  isAuditioning: boolean;
  setIsAuditioning: (value: boolean) => void;
}

const providerOptionLabel = (
  provider: NonNullable<TtsCatalog["providers"][number]>,
) => `${provider.is_local ? "Local" : "Cloud"} — ${provider.display_name}`;

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
  } = useMooseStore();
  const [ttsCatalog, setTtsCatalog] = useState<TtsCatalog | null>(null);
  const [catalogError, setCatalogError] = useState(false);

  useEffect(() => {
    let active = true;
    setCatalogError(false);
    void tauriBridge
      .getTtsCatalog()
      .then((catalog) => {
        if (active) setTtsCatalog(catalog);
      })
      .catch(() => {
        if (active) {
          setTtsCatalog(null);
          setCatalogError(true);
        }
      });
    return () => {
      active = false;
    };
  }, []);

  if (!settings) return null;

  const selectedProvider = ttsCatalog?.providers.find(
    (provider) => provider.id === settings.tts_provider,
  );
  const standaloneVoice =
    settings.tts_provider === "google"
      ? settings.google_tts_voice
      : settings.local_tts_voice;
  const standaloneVoices = selectedProvider?.voices ?? [];
  const standaloneVoiceAvailable = standaloneVoices.some(
    (voice) => voice.id === standaloneVoice,
  );
  const liveVoices = ttsCatalog?.gemini_live.voices ?? [];
  const liveVoiceAvailable = liveVoices.some(
    (voice) => voice.id === settings.live_voice,
  );

  return (
    <div className="space-y-4">
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
          onChange={(e) =>
            updateSettingsPatch({
              input_device: e.target.value || null,
            })
          }
          className="w-full p-1.5 border border-black rounded bg-white"
        >
          <option value="">Default Microphone</option>
          {inputDevices.map((d) => (
            <option key={String(d.id)} value={String(d.id)}>
              {d.name}
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
          onChange={(e) =>
            updateSettingsPatch({
              output_device: e.target.value || null,
            })
          }
          className="w-full p-1.5 border border-black rounded bg-white"
        >
          <option value="">Default Speakers</option>
          {outputDevices.map((d) => (
            <option key={String(d.id)} value={String(d.id)}>
              {d.name}
            </option>
          ))}
        </select>
      </div>

      <section className="border border-black rounded bg-[#fbf9f5] p-3 space-y-3">
        <div>
          <label
            htmlFor="settings-tts-provider"
            className="block mb-1 font-bold"
          >
            Standalone Speech Provider
          </label>
          <select
            id="settings-tts-provider"
            value={settings.tts_provider}
            disabled={!ttsCatalog}
            onChange={(e) =>
              updateSettingsPatch({
                tts_provider: e.target.value as TtsProvider,
              })
            }
            className="w-full p-1.5 border border-black rounded bg-white font-bold disabled:opacity-60"
          >
            {ttsCatalog ? (
              ttsCatalog.providers.map((provider) => (
                <option key={provider.id} value={provider.id}>
                  {providerOptionLabel(provider)}
                </option>
              ))
            ) : (
              <option value={settings.tts_provider}>
                {catalogError
                  ? "Speech catalog unavailable"
                  : "Loading speech catalog…"}
              </option>
            )}
          </select>
          <p className="mt-1 text-[11px] text-gray-700">
            This controls ambient remarks, typed replies, canned reactions, and
            other standalone speech. It does not change Gemini Live spoken
            conversations.
          </p>
        </div>

        {selectedProvider && (
          <div className="text-[11px] border-l-2 border-black pl-2 space-y-0.5">
            <p className="font-bold">
              {selectedProvider.is_local
                ? "Local / on-device"
                : "Cloud service"}
            </p>
            <p>
              {selectedProvider.is_local
                ? "Standalone text stays on this computer after the Local model has been installed and verified."
                : "Standalone speech text is sent to Google Gemini TTS for synthesis."}
            </p>
            {selectedProvider.install_required && (
              <p>
                Local model installation is required before synthesis can run.
              </p>
            )}
          </div>
        )}

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
            value={standaloneVoice}
            disabled={!selectedProvider}
            onChange={(e) => {
              if (settings.tts_provider === "google") {
                void updateSettingsPatch({ google_tts_voice: e.target.value });
              } else {
                void updateSettingsPatch({ local_tts_voice: e.target.value });
              }
            }}
            className="w-full p-1.5 border border-black rounded bg-white font-bold disabled:opacity-60"
          >
            {!standaloneVoiceAvailable && (
              <option value={standaloneVoice} disabled>
                Unavailable: {standaloneVoice}
              </option>
            )}
            {standaloneVoices.map((voice) => (
              <option key={voice.id} value={voice.id}>
                {voice.display_name}
                {voice.style ? ` (${voice.style})` : ""}
              </option>
            ))}
          </select>
        </div>

        {selectedProvider?.supports_speaking_rate && (
          <div className="space-y-1">
            <div className="flex justify-between">
              <label htmlFor="settings-speaking-rate">Speaking Rate</label>
              <span className="font-bold">
                {settings.speaking_rate.toFixed(2)}×
              </span>
            </div>
            <input
              id="settings-speaking-rate"
              type="range"
              min="0.25"
              max="4"
              step="0.05"
              value={settings.speaking_rate}
              onChange={(e) =>
                updateSettingsContinuousPatch({
                  speaking_rate: Number(e.target.value),
                })
              }
              className="w-full accent-black"
            />
            <p className="text-[11px] text-gray-700">
              {settings.tts_provider === "local"
                ? "Local KittenTTS passes this value directly to the model's speed control."
                : "Google Gemini TTS treats this as performance steering for a slower or faster delivery."}
            </p>
          </div>
        )}

        {selectedProvider?.supports_pitch ? (
          <div className="space-y-1">
            <div className="flex justify-between">
              <label htmlFor="settings-pitch">Google Pitch Steering</label>
              <span className="font-bold">{settings.pitch.toFixed(1)}</span>
            </div>
            <input
              id="settings-pitch"
              type="range"
              min="-20"
              max="20"
              step="0.5"
              value={settings.pitch}
              onChange={(e) =>
                updateSettingsContinuousPatch({
                  pitch: Number(e.target.value),
                })
              }
              className="w-full accent-black"
            />
            <p className="text-[11px] text-gray-700">
              This steers Gemini toward a lower or higher vocal register; it is
              not a DSP pitch-shift control.
            </p>
          </div>
        ) : selectedProvider ? (
          <div className="border border-black/40 rounded bg-white p-2 text-[11px] text-gray-700">
            Local KittenTTS does not expose a truthful pitch control. Your stored
            Google pitch preference ({settings.pitch.toFixed(1)}) is preserved and
            will be used again if you switch back to Google standalone speech.
          </div>
        ) : null}

        {settings.tts_provider === "google" ? (
          <button
            onClick={async () => {
              setIsAuditioning(true);
              try {
                await tauriBridge.auditionVoice(settings.google_tts_voice);
              } finally {
                setTimeout(() => setIsAuditioning(false), 2500);
              }
            }}
            disabled={isAuditioning || !selectedProvider}
            className="px-3 py-1.5 bg-white border-2 border-black rounded font-bold hover:bg-gray-100 flex items-center gap-1.5 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5 disabled:opacity-60"
          >
            <Volume2 className="w-3.5 h-3.5" />
            <span>
              {isAuditioning
                ? "Playing Sample..."
                : `Audition Google "${settings.google_tts_voice}"`}
            </span>
          </button>
        ) : (
          <p className="text-[11px] text-gray-700">
            Local voice audition is enabled only after the Local model lifecycle
            path can verify that the selected Kitten model is installed and ready.
          </p>
        )}
      </section>

      <section className="border border-black rounded bg-[#fbf9f5] p-3 space-y-2">
        <div>
          <label
            htmlFor="settings-live-voice"
            className="block mb-1 font-bold"
          >
            Gemini Live Conversation Voice
          </label>
          <select
            id="settings-live-voice"
            value={settings.live_voice}
            disabled={!ttsCatalog}
            onChange={(e) =>
              updateSettingsPatch({
                live_voice: e.target.value,
              })
            }
            className="w-full p-1.5 border border-black rounded bg-white font-bold disabled:opacity-60"
          >
            {!liveVoiceAvailable && (
              <option value={settings.live_voice} disabled>
                Unavailable: {settings.live_voice}
              </option>
            )}
            {liveVoices.map((voice) => (
              <option key={voice.id} value={voice.id}>
                {voice.display_name}
                {voice.style ? ` (${voice.style})` : ""}
              </option>
            ))}
          </select>
        </div>
        <p className="text-[11px] text-gray-700">
          Gemini Live is a separate Google cloud conversation path. Changing the
          standalone provider or Local KittenTTS voice does not change this voice.
        </p>
      </section>
    </div>
  );
};
