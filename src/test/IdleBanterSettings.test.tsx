import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { BehaviorTab } from "../components/Settings/BehaviorTab";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "../stores/mooseStore";

describe("Idle Banter behavior settings", () => {
  beforeEach(() => {
    resetSettingsPersistenceForTests();
    vi.spyOn(tauriBridge, "updateSettings").mockResolvedValue(undefined);
    useMooseStore.setState({ settings: frontendDefaultSettings() });
  });

  afterEach(() => {
    resetSettingsPersistenceForTests();
    vi.restoreAllMocks();
  });

  it("renders authoritative defaults and persists timing controls", async () => {
    render(<BehaviorTab />);
    expect(screen.getByRole("checkbox", { name: "Idle Banter" })).toBeChecked();
    expect(
      screen.getByLabelText("Idle Banter initial delay in minutes"),
    ).toHaveValue(60);
    expect(
      screen.getByLabelText("Idle Banter repeat interval in minutes"),
    ).toHaveValue(30);

    fireEvent.click(screen.getByRole("checkbox", { name: "Idle Banter" }));
    await waitFor(() =>
      expect(useMooseStore.getState().settings?.idle_banter_enabled).toBe(
        false,
      ),
    );
    fireEvent.click(screen.getByRole("checkbox", { name: "Idle Banter" }));

    fireEvent.change(
      screen.getByLabelText("Idle Banter initial delay in minutes"),
      { target: { value: "90" } },
    );
    fireEvent.change(
      screen.getByLabelText("Idle Banter repeat interval in minutes"),
      { target: { value: "45" } },
    );
    await waitFor(() => {
      expect(
        useMooseStore.getState().settings?.idle_banter_initial_delay_minutes,
      ).toBe(90);
      expect(
        useMooseStore.getState().settings?.idle_banter_repeat_interval_minutes,
      ).toBe(45);
    });
  });

  it("supports add edit delete and restore-default seed actions", async () => {
    render(<BehaviorTab />);
    const initial = frontendDefaultSettings().idle_banter_seed_topics;

    fireEvent.change(screen.getByLabelText("New Idle Banter seed topic"), {
      target: { value: "printer grudges" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    fireEvent.click(screen.getByRole("button", { name: "Save Topics" }));
    await waitFor(() =>
      expect(
        useMooseStore.getState().settings?.idle_banter_seed_topics,
      ).toContain("printer grudges"),
    );

    const first = screen.getByLabelText("Idle Banter seed topic 1");
    fireEvent.change(first, {
      target: { value: "being dramatically ignored" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save Topics" }));
    await waitFor(() =>
      expect(
        useMooseStore.getState().settings?.idle_banter_seed_topics[0],
      ).toBe("being dramatically ignored"),
    );

    fireEvent.click(screen.getByLabelText("Delete Idle Banter seed topic 1"));
    fireEvent.click(screen.getByRole("button", { name: "Save Topics" }));
    await waitFor(() =>
      expect(
        useMooseStore.getState().settings?.idle_banter_seed_topics,
      ).not.toContain("being dramatically ignored"),
    );

    fireEvent.click(screen.getByRole("button", { name: "Restore Defaults" }));
    await waitFor(() =>
      expect(
        useMooseStore.getState().settings?.idle_banter_seed_topics,
      ).toEqual(initial),
    );
  });

  it("rejects duplicate topics accessibly and prevents deleting the final topic", () => {
    const defaults = frontendDefaultSettings();
    useMooseStore.setState({
      settings: { ...defaults, idle_banter_seed_topics: ["boredom"] },
    });
    render(<BehaviorTab />);

    expect(
      screen.getByLabelText("Delete Idle Banter seed topic 1"),
    ).toBeDisabled();
    fireEvent.change(screen.getByLabelText("New Idle Banter seed topic"), {
      target: { value: "  BOREDOM " },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(screen.getByRole("alert")).toHaveTextContent(/duplicated/i);
  });

  it("rejects oversized seed topics before persistence", () => {
    const defaults = frontendDefaultSettings();
    useMooseStore.setState({
      settings: { ...defaults, idle_banter_seed_topics: ["boredom"] },
    });
    render(<BehaviorTab />);

    fireEvent.change(screen.getByLabelText("Idle Banter seed topic 1"), {
      target: { value: "x".repeat(121) },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save Topics" }));
    expect(screen.getByRole("alert")).toHaveTextContent(/120 characters/i);
    expect(useMooseStore.getState().settings?.idle_banter_seed_topics).toEqual([
      "boredom",
    ]);
  });

  it("explains the global ambient master gate and disables dependent controls", () => {
    const defaults = frontendDefaultSettings();
    useMooseStore.setState({
      settings: {
        ...defaults,
        unsolicited_comments: false,
        idle_banter_enabled: false,
      },
    });
    render(<BehaviorTab />);
    expect(screen.getByRole("status")).toHaveTextContent(/paused/i);
    expect(
      screen.getByLabelText("Idle Banter initial delay in minutes"),
    ).toBeDisabled();
    expect(screen.getByLabelText("New Idle Banter seed topic")).toBeDisabled();
  });
});
