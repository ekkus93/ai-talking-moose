import backendContract from "../generated/backendContract.json";
import {
  AppSettings,
  GoogleModelDescriptor,
  GoogleTtsVoiceDescriptor,
  TtsCatalog,
  TtsProvider,
} from "../types/moose";

// JSON imports widen serialized enum literals to `string`. The Rust-derived
// representative shape is checked against `AppSettings` by
// `check:frontend-contract-shapes`, so this assertion bridges that import-only
// widening rather than hiding an unchecked hand-maintained contract.
export const frontendDefaultSettings = (): AppSettings =>
  ({
    ...backendContract.settings,
  }) as AppSettings;

export const frontendGoogleModels = (): GoogleModelDescriptor[] =>
  backendContract.google_models.map((model) => ({
    ...model,
    capabilities: [...model.capabilities],
  })) as GoogleModelDescriptor[];

export const frontendGoogleTtsVoices = (): GoogleTtsVoiceDescriptor[] =>
  backendContract.google_tts_voices.map((voice) => ({ ...voice }));

const parseTtsProvider = (value: string): TtsProvider => {
  if (value === "google" || value === "local") return value;
  throw new Error(
    `Unknown TTS provider in generated backend contract: ${value}`,
  );
};

export const frontendTtsCatalog = (): TtsCatalog => ({
  providers: backendContract.tts_catalog.providers.map((provider) => ({
    id: parseTtsProvider(provider.id),
    display_name: provider.display_name,
    is_local: provider.is_local,
    models: provider.models.map((model) => ({ ...model })),
    voices: provider.voices.map((voice) => ({ ...voice })),
    sample_rate_hz: provider.sample_rate_hz,
    supports_speaking_rate: provider.supports_speaking_rate,
    supports_pitch: provider.supports_pitch,
    install_required: provider.install_required,
    license_summary: provider.license_summary,
  })),
  gemini_live: {
    ...backendContract.tts_catalog.gemini_live,
    voices: backendContract.tts_catalog.gemini_live.voices.map((voice) => ({
      ...voice,
    })),
  },
});
