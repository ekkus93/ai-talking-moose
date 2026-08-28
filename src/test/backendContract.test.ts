import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";
import {
  frontendDefaultSettings,
  frontendGoogleModels,
  frontendGoogleTtsVoices,
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
  });

  it("keeps default model ids inside the generated Rust catalog", () => {
    const settings = frontendDefaultSettings();
    const models = frontendGoogleModels();
    expect(models.map((model) => model.id)).toContain(settings.live_model);
    expect(models.map((model) => model.id)).toContain(settings.text_model);
    expect(frontendGoogleTtsVoices()).toHaveLength(30);
  });
});
