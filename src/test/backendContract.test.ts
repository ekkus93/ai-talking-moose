import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";
import {
  frontendDefaultSettings,
  frontendGoogleModels,
  frontendGoogleTtsVoices,
  frontendTtsCatalog,
} from "../lib/backendContract";
import { isTauri, tauriBridge } from "../lib/tauriBridge";

describe("generated backend contract", () => {
  it("runs production-like tests through the Tauri invoke boundary", async () => {
    expect(isTauri()).toBe(true);

    await tauriBridge.getSettings();

    expect(vi.mocked(invoke)).toHaveBeenCalledWith("get_settings");
  });

  it("drives frontend settings from the generated Rust contract", async () => {
    const expected = frontendDefaultSettings();
    expect(await tauriBridge.getSettings()).toEqual(expected);
    expect(expected).not.toHaveProperty("microphone_permission_granted");
    expect(expected).not.toHaveProperty("provider");
    expect(expected).not.toHaveProperty("text_model");
    expect(expected.text_provider).toBe("local");
  });

  it("keeps default Google model ids inside the generated Rust catalog", () => {
    const settings = frontendDefaultSettings();
    const models = frontendGoogleModels();
    expect(models.map((model) => model.id)).toContain(settings.live_model);
    expect(models.map((model) => model.id)).toContain(
      settings.google_text_model,
    );
    expect(settings.local_text_model).toBe("smollm2-360m-instruct-q4-k-m");
    expect(frontendGoogleTtsVoices()).toHaveLength(30);
  });

  it("exposes truthful TTS provider capabilities without frontend guesses", async () => {
    const catalog = frontendTtsCatalog();
    expect(await tauriBridge.getTtsCatalog()).toEqual(catalog);
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("get_tts_catalog");

    const google = catalog.providers.find(
      (provider) => provider.id === "google",
    );
    const local = catalog.providers.find((provider) => provider.id === "local");
    expect(google).toMatchObject({
      is_local: false,
      sample_rate_hz: 24000,
      supports_speaking_rate: true,
      supports_pitch: true,
      install_required: false,
    });
    expect(google?.voices).toHaveLength(30);
    expect(local).toMatchObject({
      is_local: true,
      sample_rate_hz: 24000,
      supports_speaking_rate: true,
      supports_pitch: false,
      install_required: true,
    });
    expect(local?.models.map((model) => model.id)).toEqual([
      "KittenML/kitten-tts-mini-0.8",
    ]);
    expect(local?.voices).toHaveLength(8);
    expect(catalog.gemini_live.voices).toHaveLength(30);
  });
});
