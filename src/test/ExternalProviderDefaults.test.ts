import { expect, it } from "vitest";
import { frontendDefaultSettings } from "../lib/backendContract";
it("keeps fresh profiles on Local text and TTS", () => {
  const defaults = frontendDefaultSettings();
  expect(defaults.text_provider).toBe("local");
  expect(defaults.tts_provider).toBe("local");
});

it("keeps external provider model configuration separate from the selected Local provider", () => {
  const defaults = frontendDefaultSettings();
  expect(defaults.openai_text_model).toBe("gpt-4.1-mini");
  expect(defaults.ollama_text_model).toBe("llama3.2");
  expect(defaults.ollama_server_address).toBe("http://127.0.0.1:11434");
  expect(defaults.llama_server_text_model).toBe("default");
  expect(defaults.llama_server_address).toBe("http://127.0.0.1:8080");
  expect(defaults.system_tts_voice).toBe("system_default");
  expect(defaults.text_provider).toBe("local");
  expect(defaults.tts_provider).toBe("local");
});

it("keeps fresh profiles free of transcript persistence", () => {
  expect(frontendDefaultSettings().save_transcripts).toBe(false);
});
