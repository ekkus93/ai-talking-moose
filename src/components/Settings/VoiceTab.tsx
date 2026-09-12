import React from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { tauriBridge } from "../../lib/tauriBridge";
import type {
  TtsCatalog,
  TtsProvider,
  TtsVoiceDescriptor,
} from "../../types/moose";
import { Volume2 } from "lucide-react";

// Owned by SettingsModalBase so the audition debounce survives a tab switch;
// held locally it would reset on unmount and allow overlapping playback.
interface VoiceTabProps {
  isAuditioning: boolean;
  setIsAuditioning: (value: boolean) => void;
  ttsCatalog: TtsCatalog | null;
  ttsCatalogStatus: "loading" | "ready" | "error";
}

const voiceLabel = (voice: TtsVoiceDescriptor) =>
  voice.style
    ? `${voice.display_name} (${voice.style})`
    : voice.display_name;

export const VoiceTab: React.FC<VoiceTabProps> = ({
  isAuditioning,
  setIsAuditioning,
  ttsCatalog,
  ttsCatalogStatus,
}) => {
  const {
    settings,
    updateSettingsPatch,
    updateSettingsContinuousPatch,
    inputDevices,
    outputDevices,
  } = useMooseStore();
  if (!settings) return null;

  const provider = ttsCatalog?.providers.find(
    (candidate) => candidate.id === settings.tts_provider,
  );
  const standaloneVoice =
    settings.tts_provider === "local"
      ? settings.local_tts_voice
      : settings.google_tts_voice;
  const providerCatalogReady = ttsCatalogStatus === "ready" && provider !== undefined;
  const standaloneVoiceAvailable =
    provider?.voices.some((voice) => voice.id === standaloneVoice) ?? false;
  const liveVoiceAvailable =
    ttsCatalog?.gemini_live.voices.some(
      (voice) => voice.id === settings.live_voice,
    ) ?? false;

  const selectProvider = async (nextProvider: TtsProvider) => {
    if (nextProvider === settings.tts_provider) return;
    await tauriBridge.cancelStandaloneSpeech();
    await updateSettingsPatch({ tts_provider: nextProvider });
  };

  const updateStandaloneVoice = async (voiceId: string) => {
    if (settings.tts_provider === "local") {
      await updateSettingsPatch({ local_tts_voice: voiceId });
    } else {
      await updateSettingsPatch({ google_tts_voice: voiceId });
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

      <section
        className="border-2 border-black rounded bg-[#fbf9f5] p-3 space-y-3"
        aria-labelledby="standalone-speech-heading"
      >
        <div>
          <h4 id="standalone-speech-heading" className="font-bold">
            Standalone Speech
          </h4>
          <p className="text-[10px] text-gray-700">
            This provider receives text for typed replies, ambient remarks, canned
            reactions, and voice auditions. It does not change Gemini Live Talk
            sessions.
          </p>
        </div>

        <fieldset className="space-y-2">
          <legend className="font-bold mb-1">Standalone Speech Provider</legend>
          <label className="flex items-start gap-2 p-2 border border-gray-400 rounded cursor-pointer">
            <input
              type="radio"
              name="tts-provider"
              value="google"
              checked={settings.tts_provider === "google"}
              onChange={() => void selectProvider("google")}
            />
            <span>
              <span className="font-bold">Google Gemini TTS — cloud</span>
              <span className="block text-[10px] text-gray-700">
                Sends standalone speech text to Google using your saved API key.
              </span>
            </span>
          </label>
          <label className="flex items-start gap-2 p-2 border border-gray-400 rounded cursor-pointer">
            <input
              type="radio"
              name="tts-provider"
              value="local"
              checked={settings.tts_provider === "local"}
              onChange={() => void selectProvider("local")}
            />
            <span>
              <span className="font-bold">Local KittenTTS — on this computer</span>
              <span className="block text-[10px] text-gray-700">
                After the pinned model is installed and verified, synthesis runs
                locally and does not send the utterance to Google.
              </span>
            </span>
          </label>
        </fieldset>

        <div>
          <label
            htmlFor="settings-standalone-tts-voice"
            className="block mb-1 font-bold"
          >
            {settings.tts_provider === "local"
              ? "Local KittenTTS Voice"
              : "Google Standalone TTS Voice"}
          </label>
          <select
            id="settings-standalone-tts-voice"
            value={standaloneVoice}
            disabled={!providerCatalogReady}
            aria-busy={ttsCatalogStatus === "loading"}
            onChange={(e) => void updateStandaloneVoice(e.target.value)}
            className="w-full p-1.5 border border-black rounded bg-white font-bold disabled:opacity-60"
          >
            {ttsCatalogStatus === "loading" ? (
              <option value={standaloneVoice}>
                Current: {standaloneVoice} (loading catalog…)
              </option>
            ) : ttsCatalogStatus === "error" || !provider ? (
              <option value={standaloneVoice}>
                Current: {standaloneVoice} (catalog unavailable)
              </option>
            ) : (
              <>
                {!standaloneVoiceAvailable && (
                  <option value={standaloneVoice} disabled>
                    Unavailable: {standaloneVoice}
                  </option>
                )}
                {provider.voices.map((voice) => (
                  <option key={voice.id} value={voice.id}>
                    {voiceLabel(voice)}
                  </option>
                ))}
              </>
            )}
          </select>
        </div>

        <button
          type="button"
          onClick={async () => {
            setIsAuditioning(true);
            try {
              await tauriBridge.auditionVoice(standaloneVoice);
            } finally {
              setTimeout(() => setIsAuditioning(false), 2500);
            }
          }}
          disabled={isAuditioning || !providerCatalogReady}
          className="px-3 py-1.5 bg-white border-2 border-black rounded font-bold hover:bg-gray-100 flex items-center gap-1.5 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] active:translate-y-0.5 disabled:opacity-60"
        >
          <Volume2 className="w-3.5 h-3.5" />
          <span>
            {isAuditioning
              ? "Playing Sample..."
              : `Audition ${settings.tts_provider === "local" ? "Local" : "Google"} Voice “${standaloneVoice}”`}
          </span>
        </button>

        <div className="space-y-1">
          <div className="flex justify-between gap-3">
            <label htmlFor="settings-speaking-rate" className="font-bold">
              Speaking Rate
            </label>
            <span className="font-bold">{settings.speaking_rate.toFixed(2)}×</span>
          </div>
          <input
            id="settings-speaking-rate"
            type="range"
            min="0.25"
            max="4"
            step="0.05"
            value={settings.speaking_rate}
            disabled={!provider?.supports_speaking_rate}
            onChange={(e) =>
              updateSettingsContinuousPatch({
                speaking_rate: Number(e.target.value),
              })
            }
            className="w-full accent-black disabled:opacity-60"
          />
          <p className="text-[10px] text-gray-700">
            {settings.tts_provider === "local"
              ? "Controls KittenTTS model speed directly."
              : "Guides Gemini TTS performance pace while preserving the exact utterance text."}
          </p>
        </div>

        {provider?.supports_pitch ? (
          <div className="space-y-1">
            <div className="flex justify-between gap-3">
              <label htmlFor="settings-pitch" className="font-bold">
                Pitch
              </label>
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
            <p className="text-[10px] text-gray-700">
              Google Gemini TTS interprets this as performance/register steering.
            </p>
          </div>
        ) : (
          <div className="p-2 border border-gray-400 rounded bg-white">
            <div className="font-bold">Pitch — unavailable for Local KittenTTS</div>
            <p className="text-[10px] text-gray-700">
              KittenTTS Mini does not expose truthful pitch control. Your saved
              Google pitch value ({settings.pitch.toFixed(1)}) is preserved and
              will be used again if you switch back to Google Gemini TTS.
            </p>
          </div>
        )}
      </section>

      <section
        className="border border-black rounded bg-[#fbf9f5] p-3 space-y-2"
        aria-labelledby="live-voice-heading"
      >
        <div>
          <h4 id="live-voice-heading" className="font-bold">
            Gemini Live Conversation Voice — cloud
          </h4>
          <p className="text-[10px] text-gray-700">
            Used only when you press Talk for a Gemini Live conversation. This is
            independent of the standalone speech provider and voice above.
          </p>
        </div>
        <label htmlFor="settings-live-voice" className="block font-bold">
          Live Conversation Voice
        </label>
        <select
          id="settings-live-voice"
          value={settings.live_voice}
          disabled={ttsCatalogStatus !== "ready" || !ttsCatalog}
          aria-busy={ttsCatalogStatus === "loading"}
          onChange={(e) =>
            void updateSettingsPatch({ live_voice: e.target.value })
          }
          className="w-full p-1.5 border border-black rounded bg-white font-bold disabled:opacity-60"
        >
          {ttsCatalogStatus === "loading" ? (
            <option value={settings.live_voice}>
              Current: {settings.live_voice} (loading catalog…)
            </option>
          ) : ttsCatalogStatus === "error" || !ttsCatalog ? (
            <option value={settings.live_voice}>
              Current: {settings.live_voice} (catalog unavailable)
            </option>
          ) : (
            <>
              {!liveVoiceAvailable && (
                <option value={settings.live_voice} disabled>
                  Unavailable: {settings.live_voice}
                </option>
              )}
              {ttsCatalog.gemini_live.voices.map((voice) => (
                <option key={voice.id} value={voice.id}>
                  {voiceLabel(voice)}
                </option>
              ))}
            </>
          )}
        </select>
      </section>
    </div>
  );
};
