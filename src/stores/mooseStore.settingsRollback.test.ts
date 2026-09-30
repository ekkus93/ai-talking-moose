import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { resetSettingsPersistenceForTests, useMooseStore } from "./mooseStore";

describe("mooseStore settings persistence rollback", () => {
  beforeEach(() => {
    resetSettingsPersistenceForTests();
    useMooseStore.setState({ settings: frontendDefaultSettings() });
  });

  afterEach(() => {
    resetSettingsPersistenceForTests();
    vi.restoreAllMocks();
  });

  it("restores authoritative persisted settings after a rejected discrete write", async () => {
    const initial = frontendDefaultSettings();
    const persisted = {
      ...initial,
      volume: 0.64,
      unsolicited_comments: false,
    };
    useMooseStore.setState({ settings: initial });

    const write = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockRejectedValueOnce(new Error("private persistence detail"));
    const reload = vi
      .spyOn(tauriBridge, "getSettings")
      .mockResolvedValueOnce(persisted);

    const result = await useMooseStore
      .getState()
      .updateSettingsPatch({ volume: 0.25 });

    expect(result).toEqual({
      status: "rolled_back",
      message:
        "Settings could not be saved. Your last persisted settings were restored.",
    });
    expect(write).toHaveBeenCalledTimes(1);
    expect(write).toHaveBeenCalledWith(
      expect.objectContaining({
        volume: 0.25,
      }),
    );
    expect(reload).toHaveBeenCalledTimes(1);
    expect(useMooseStore.getState().settings).toEqual(persisted);
    expect(useMooseStore.getState().settings?.volume).not.toBe(0.25);
  });
});
