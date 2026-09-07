import React from "react";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AiTab } from "../components/Settings/AiTab";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "../stores/mooseStore";

const staleLocalPanel = vi.hoisted(() => ({
  onSelectModel: null as null | ((modelId: string) => Promise<void>),
}));

vi.mock("../components/Settings/LocalLlmSettingsPanel", () => ({
  LocalLlmSettingsPanel: ({
    onSelectModel,
  }: {
    onSelectModel: (modelId: string) => Promise<void>;
  }) => {
    if (!staleLocalPanel.onSelectModel) {
      staleLocalPanel.onSelectModel = onSelectModel;
    }
    return React.createElement(
      "button",
      {
        type: "button",
        onClick: () =>
          void staleLocalPanel.onSelectModel?.("qwen3-0-6b-instruct-q4-k-m"),
      },
      "Choose model from stale Local panel",
    );
  },
}));

describe("Settings patch intent", () => {
  beforeEach(() => {
    resetSettingsPersistenceForTests();
    staleLocalPanel.onSelectModel = null;
    useMooseStore.setState({
      settings: {
        ...frontendDefaultSettings(),
        text_provider: "local",
        local_text_model: "smollm2-360m-instruct-q4-k-m",
        save_transcripts: false,
      },
      hasApiKey: false,
    });
  });

  afterEach(() => {
    resetSettingsPersistenceForTests();
    vi.restoreAllMocks();
  });

  it("preserves an unrelated newer setting when a stale Local model control fires", async () => {
    const persist = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockResolvedValue(undefined);

    render(
      <AiTab
        googleModels={[]}
        googleModelsStatus="ready"
        apiKeyInput=""
        setApiKeyInput={() => undefined}
        testResult={null}
        setTestResult={() => undefined}
        isTesting={false}
        setIsTesting={() => undefined}
      />,
    );

    // Simulate another component/persistence path advancing an unrelated field
    // after this Local panel captured its original callback. The mocked child
    // deliberately retains that first callback across the parent re-render.
    act(() => {
      useMooseStore.setState((state) => ({
        settings: state.settings
          ? { ...state.settings, save_transcripts: true }
          : null,
      }));
    });

    fireEvent.click(
      screen.getByRole("button", {
        name: "Choose model from stale Local panel",
      }),
    );

    await waitFor(() => expect(persist).toHaveBeenCalledTimes(1));
    expect(useMooseStore.getState().settings).toMatchObject({
      local_text_model: "qwen3-0-6b-instruct-q4-k-m",
      save_transcripts: true,
    });
    expect(persist.mock.calls[0][0]).toMatchObject({
      local_text_model: "qwen3-0-6b-instruct-q4-k-m",
      save_transcripts: true,
    });
  });
});
