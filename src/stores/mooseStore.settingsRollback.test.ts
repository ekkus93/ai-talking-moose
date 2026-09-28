import { invoke } from "@tauri-apps/api/core";
import { waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { frontendDefaultSettings } from "../lib/backendContract";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "./mooseStore";

const persistedWakeDisabledSettings = () => ({
  ...frontendDefaultSettings(),
  wake_word_enabled: false,
  wake_word_phrase: "Hey, Moose",
});

describe("mooseStore settings persistence rollback", () => {
  beforeEach(() => {
    resetSettingsPersistenceForTests();
    vi.clearAllMocks();
  });

  it("restores authoritative persisted Wake settings after persistence failure", async () => {
    const persisted = persistedWakeDisabledSettings();
    const defaultInvoke = vi.mocked(invoke).getMockImplementation();
    if (!defaultInvoke) {
      throw new Error(
        "Tauri invoke test fixture is missing its default implementation",
      );
    }
    useMooseStore.setState({ settings: persisted });
    vi.mocked(invoke).mockImplementation(async (cmd, args, options) => {
      if (cmd === "update_settings") {
        throw new Error("simulated Wake settings persistence failure");
      }
      if (cmd === "get_settings") return persisted;
      return defaultInvoke(cmd, args, options);
    });

    await useMooseStore.getState().updateSettingsPatch({
      wake_word_enabled: true,
      wake_word_phrase: "Hey, Moose",
    });

    expect(invoke).toHaveBeenCalledWith(
      "update_settings",
      expect.objectContaining({
        newSettings: expect.objectContaining({
          wake_word_enabled: true,
          wake_word_phrase: "Hey, Moose",
        }),
      }),
    );
    await waitFor(() => {
      expect(useMooseStore.getState().settings).toMatchObject({
        wake_word_enabled: false,
        wake_word_phrase: "Hey, Moose",
      });
    });
    expect(invoke).toHaveBeenCalledWith("get_settings");
  });
});
