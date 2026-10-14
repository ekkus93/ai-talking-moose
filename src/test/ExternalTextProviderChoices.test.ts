import { expect, it } from "vitest";
import { frontendDefaultSettings } from "../lib/backendContract";

it("starts with Local text and Local speech", () => {
  const settings = frontendDefaultSettings();
  expect(settings.text_provider).toBe("local");
  expect(settings.tts_provider).toBe("local");
});
