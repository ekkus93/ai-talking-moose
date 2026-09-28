import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "./mooseStore";

describe("mooseStore settings persistence rollback", () => {
  beforeEach(() => {
    resetSettingsPersistenceForTests();
    useMooseStore.setState({
      settings: {
        ...frontendDefaultSettings(),
        wake_word_enabled: false,
        wake_word_phrase: "Hey, Moose",
      },
    });
  });

  afterEach(() => {
    resetSettingsPersistenceForTests();
    vi.restoreAllMocks();
  });

  it("does not resolve a failed settings update until rollback is reflected", async () => {
    const authoritative = {
      ...frontendDefaultSettings(),
      wake_word_enabled: false,
      wake_word_phrase: "Hey, Moose",
    };
    vi.spyOn(tauriBridge, "updateSettings").mockRejectedValue(
      new Error("injected persistence failure"),
    );
    vi.spyOn(tauriBridge, "getSettings").mockImplementation(async () => {
      await Promise.resolve();
      return { ...authoritative };
    });

    await useMooseStore
      .getState()
      .updateSettingsPatch({ wake_word_enabled: true });

    expect(useMooseStore.getState().settings).toMatchObject({
      wake_word_enabled: false,
      wake_word_phrase: "Hey, Moose",
    });
  });
});
