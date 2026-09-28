import { waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
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
    useMooseStore.setState({ settings: persistedWakeDisabledSettings() });
  });

  afterEach(() => {
    resetSettingsPersistenceForTests();
    vi.restoreAllMocks();
  });

  it("restores authoritative persisted Wake settings after persistence failure", async () => {
    const persisted = persistedWakeDisabledSettings();
    const updateSettings = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockRejectedValueOnce(
        new Error("simulated Wake settings persistence failure"),
      );
    const getSettings = vi
      .spyOn(tauriBridge, "getSettings")
      .mockResolvedValueOnce(persisted);

    await useMooseStore.getState().updateSettingsPatch({
      wake_word_enabled: true,
      wake_word_phrase: "Hey, Moose",
    });

    expect(updateSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        wake_word_enabled: true,
        wake_word_phrase: "Hey, Moose",
      }),
    );
    await waitFor(() => {
      expect(useMooseStore.getState().settings).toMatchObject({
        wake_word_enabled: false,
        wake_word_phrase: "Hey, Moose",
      });
    });
    expect(getSettings).toHaveBeenCalledTimes(1);
  });
});
