import { expect, it } from "vitest";
import { frontendDefaultSettings } from "../lib/backendContract";

it("starts with Local text and Local speech", () => {
  const settings = frontendDefaultSettings();
  expect(settings.text_provider).toBe("local");
  expect(settings.tts_provider).toBe("local");
});

it("preserves independent loopback defaults without selecting either server", () => {
  const settings = frontendDefaultSettings();
  expect(settings.ollama_server_address).toBe("http://127.0.0.1:11434");
  expect(settings.llama_server_address).toBe("http://127.0.0.1:8080");
  expect(settings.ollama_text_model).not.toBe(settings.llama_server_text_model);
  expect(settings.text_provider).toBe("local");
});
