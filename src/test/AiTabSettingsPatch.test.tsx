import { act, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AiTab } from "../components/Settings/AiTab";
import {
  frontendDefaultSettings,
  frontendGoogleModels,
} from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import {
  resetSettingsPersistenceForTests,
  useMooseStore,
} from "../stores/mooseStore";

const localPanelHarness = vi.hoisted(() => ({
  onSelectModel: undefined as undefined | ((modelId: string) => Promise<void>),
}));

vi.mock("../components/Settings/LocalLlmSettingsPanel", () => ({
  LocalLlmSettingsPanel: ({
    onSelectModel,
  }: {
    selectedModelId: string;
    onSelectModel: (modelId: string) => Promise<void>;
  }) => {
    localPanelHarness.onSelectModel = onSelectModel;
    return <div data-testid="local-llm-settings-panel" />;
  },
}));

describe("AiTab settings patch intent", () => {
  beforeEach(() => {
    resetSettingsPersistenceForTests();
    localPanelHarness.onSelectModel = undefined;
    useMooseStore.setState({
      settings: frontendDefaultSettings(),
      hasApiKey: false,
    });
  });

  afterEach(() => {
    resetSettingsPersistenceForTests();
    vi.restoreAllMocks();
  });

  it("keeps unrelated newer settings when an old Local-model callback runs", async () => {
    const persist = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockResolvedValue(undefined);

    render(
      <AiTab
        googleModels={frontendGoogleModels()}
        googleModelsStatus="ready"
        apiKeyInput=""
        setApiKeyInput={vi.fn()}
        testResult={null}
        setTestResult={vi.fn()}
        isTesting={false}
        setIsTesting={vi.fn()}
      />,
    );

    const staleSelectModel = localPanelHarness.onSelectModel;
    expect(staleSelectModel).toBeDefined();

    await act(async () => {
      await useMooseStore
        .getState()
        .updateSettingsPatch({ memory_enabled: true });
    });
    expect(useMooseStore.getState().settings?.memory_enabled).toBe(true);

    await act(async () => {
      await staleSelectModel!("qwen3-0-6b-instruct-q4-k-m");
    });

    expect(useMooseStore.getState().settings).toMatchObject({
      memory_enabled: true,
      local_text_model: "qwen3-0-6b-instruct-q4-k-m",
    });
    expect(persist).toHaveBeenCalledTimes(2);
    expect(persist.mock.calls[1][0]).toMatchObject({
      memory_enabled: true,
      local_text_model: "qwen3-0-6b-instruct-q4-k-m",
    });
  });
});
