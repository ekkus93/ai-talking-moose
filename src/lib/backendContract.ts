import backendContract from "../generated/backendContract.json";
import {
  AppSettings,
  GoogleModelDescriptor,
  GoogleTtsVoiceDescriptor,
} from "../types/moose";

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
