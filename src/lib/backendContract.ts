import backendContract from "../generated/backendContract.json";
import {
  AppSettings,
  GoogleModelDescriptor,
  GoogleTtsVoiceDescriptor,
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
