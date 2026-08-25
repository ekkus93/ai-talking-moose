import { describe, expect, it } from "vitest";
import {
  frontendDefaultSettings,
  frontendGoogleModels,
  frontendGoogleTtsVoices,
} from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";

describe("generated backend contract", () => {
  it("drives the browser settings fallback without persisted runtime permission state", async () => {
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
