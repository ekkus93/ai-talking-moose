import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import type { AppSettings } from "../types/moose";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "./mooseStore";

const deferred = <T>() => {
  const controls: { resolve?: (value: T) => void } = {};
  const promise = new Promise<T>((resolve) => {
    controls.resolve = resolve;
  });
  return {
    promise,
    resolve: (value: T) => {
      if (!controls.resolve) {
        throw new Error("deferred resolver was not initialized");
      }
      controls.resolve(value);
    },
  };
};

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
    const authoritative: AppSettings = {
      ...frontendDefaultSettings(),
      wake_word_enabled: false,
      wake_word_phrase: "Hey, Moose",
    };
    const authoritativeSettings = deferred<AppSettings>();
    let updateResolved = false;

    vi.spyOn(tauriBridge, "updateSettings").mockRejectedValue(
      new Error("injected persistence failure"),
    );
    vi.spyOn(tauriBridge, "getSettings").mockReturnValue(
      authoritativeSettings.promise,
    );

    const update = useMooseStore
      .getState()
      .updateSettingsPatch({ wake_word_enabled: true })
      .then(() => {
        updateResolved = true;
      });

    await Promise.resolve();
    await Promise.resolve();

    expect(updateResolved).toBe(false);
    expect(useMooseStore.getState().settings).toMatchObject({
      wake_word_enabled: true,
      wake_word_phrase: "Hey, Moose",
    });

    authoritativeSettings.resolve(authoritative);
    await update;

    expect(updateResolved).toBe(true);
    expect(useMooseStore.getState().settings).toMatchObject({
      wake_word_enabled: false,
      wake_word_phrase: "Hey, Moose",
    });
  });
});
