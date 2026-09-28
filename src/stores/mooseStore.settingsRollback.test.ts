import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { frontendDefaultSettings } from "../lib/backendContract";
import type { AppSettings } from "../types/moose";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "./mooseStore";

type TestTauriInternals = {
  __TAURI_INTERNALS__: {
    invoke: (
      command: string,
      commandArgs?: Record<string, unknown>,
    ) => Promise<unknown>;
  };
};

const tauriInternals = () =>
  (window as unknown as TestTauriInternals).__TAURI_INTERNALS__;

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
  let originalInvoke: TestTauriInternals["__TAURI_INTERNALS__"]["invoke"];

  beforeEach(() => {
    resetSettingsPersistenceForTests();
    originalInvoke = tauriInternals().invoke;
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
    tauriInternals().invoke = originalInvoke;
  });

  it("does not resolve a failed settings update until rollback is reflected", async () => {
    const authoritative: AppSettings = {
      ...frontendDefaultSettings(),
      wake_word_enabled: false,
      wake_word_phrase: "Hey, Moose",
    };
    const authoritativeSettings = deferred<AppSettings>();
    let updateResolved = false;

    tauriInternals().invoke = async (command, commandArgs) => {
      if (command === "update_settings") {
        throw new Error("injected persistence failure");
      }
      if (command === "get_settings") {
        return authoritativeSettings.promise;
      }
      return originalInvoke(command, commandArgs);
    };

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
