import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "../stores/mooseStore";
import type { AppSettings } from "../types/moose";

beforeEach(() => {
  resetSettingsPersistenceForTests();
  useMooseStore.setState({
    settings: frontendDefaultSettings(),
    settingsPersistenceError: null,
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  resetSettingsPersistenceForTests();
});

it("serializes provider and TTS settings writes", async () => {
  const persisted: AppSettings[] = [];
  let releaseFirst: () => void = () => {
    throw new Error("first settings request was not started");
  };
  const firstPending = new Promise<void>((resolve) => {
    releaseFirst = () => resolve();
  });
  let activeWrites = 0;
  let maxConcurrentWrites = 0;
  const updateSettings = vi.spyOn(tauriBridge, "updateSettings");
  updateSettings.mockImplementation(async (candidate) => {
    activeWrites += 1;
    maxConcurrentWrites = Math.max(maxConcurrentWrites, activeWrites);
    persisted.push({ ...candidate });
    if (persisted.length === 1) {
      await firstPending;
    }
    activeWrites -= 1;
  });

  const provider = useMooseStore
    .getState()
    .updateSettingsPatch({ text_provider: "ollama" });
  const model = useMooseStore
    .getState()
    .updateSettingsPatch({ ollama_text_model: "llama3.2:instruct" });
  const voice = useMooseStore
    .getState()
    .updateSettingsPatch({ tts_provider: "google" });

  expect(useMooseStore.getState().settings?.text_provider).toBe("ollama");
  expect(useMooseStore.getState().settings?.tts_provider).toBe("google");
  expect(persisted).toHaveLength(1);
  expect(persisted[0].text_provider).toBe("ollama");
  expect(persisted[0].tts_provider).toBe("local");

  releaseFirst();
  const outcomes = await Promise.all([provider, model, voice]);
  expect(outcomes.map((outcome) => outcome.status)).toEqual([
    "persisted",
    "persisted",
    "persisted",
  ]);
  expect(maxConcurrentWrites).toBe(1);
  expect(persisted).toHaveLength(3);
  expect(persisted[2].text_provider).toBe("ollama");
  expect(persisted[2].ollama_text_model).toBe("llama3.2:instruct");
  expect(persisted[2].tts_provider).toBe("google");
  expect(persisted[2].openai_text_model).toBe(
    frontendDefaultSettings().openai_text_model,
  );
  expect(persisted[2].llama_server_address).toBe(
    frontendDefaultSettings().llama_server_address,
  );
  expect(useMooseStore.getState().settings).toMatchObject({
    text_provider: "ollama",
    ollama_text_model: "llama3.2:instruct",
    tts_provider: "google",
  });
});
