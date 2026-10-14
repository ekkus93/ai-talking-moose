import { expect, it } from "vitest";
import { frontendDefaultSettings } from "../lib/backendContract";
it("keeps fresh profiles on Local text and TTS", () => {
  const defaults = frontendDefaultSettings();
  expect(defaults.text_provider).toBe("local");
  expect(defaults.tts_provider).toBe("local");
});
