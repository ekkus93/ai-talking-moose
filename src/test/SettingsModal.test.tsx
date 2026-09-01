import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { useMooseStore } from "../stores/mooseStore";
import {
  frontendDefaultSettings,
  frontendGoogleModels,
} from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";

const originalBridgeMethods = {
  getGoogleModels: tauriBridge.getGoogleModels,
  updateSettings: tauriBridge.updateSettings,
  setGoogleApiKey: tauriBridge.setGoogleApiKey,
  clearGoogleApiKey: tauriBridge.clearGoogleApiKey,
  getLocalLlmModels: tauriBridge.getLocalLlmModels,
  installLocalLlmModel: tauriBridge.installLocalLlmModel,
  cancelLocalLlmInstall: tauriBridge.cancelLocalLlmInstall,
  deleteLocalLlmModel: tauriBridge.deleteLocalLlmModel,
  testLocalLlmModel: tauriBridge.testLocalLlmModel,
  onLocalLlmModelProgress: tauriBridge.onLocalLlmModelProgress,
};

describe("SettingsModal Component", () => {
  beforeEach(() => {
    useMooseStore.setState({
      isSettingsOpen: true,
      settings: frontendDefaultSettings(),
      hasApiKey: false,
      inputDevices: [],
      outputDevices: [],
      googleTtsVoices: [],
      memories: [],
    });
  });

  afterEach(() => {
    Object.assign(tauriBridge, originalBridgeMethods);
    vi.restoreAllMocks();
    vi.clearAllMocks();
  });

  it("does not mount the P6 acceptance harness in production Settings", () => {
    render(<SettingsModal />);
    expect(screen.queryByText(/P6 Voice Acceptance/i)).not.toBeInTheDocument();
  });

  it("renders tabs and navigates to personality", () => {
    render(<SettingsModal />);
    expect(screen.getByTestId("settings-modal")).toBeInTheDocument();

    const personalityTab = screen.getByText("Personality");
    fireEvent.click(personalityTab);

    expect(screen.getByText("Moose Character Persona")).toBeInTheDocument();
    expect(screen.getByText("Dry Wit & Deadpan")).toBeInTheDocument();
  });

  it("updates every behavior and personality control in runtime settings", async () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("Behavior"));

    fireEvent.click(
      screen.getByRole("checkbox", {
        name: /Enable unsolicited ambient remarks/i,
      }),
    );
    await waitFor(() =>
      expect(useMooseStore.getState().settings?.unsolicited_comments).toBe(
        false,
      ),
    );

    fireEvent.change(screen.getAllByRole("slider")[0], {
      target: { value: "0.75" },
    });
    await waitFor(() =>
      expect(useMooseStore.getState().settings?.talkativeness).toBe(0.75),
    );

    fireEvent.change(screen.getAllByRole("slider")[1], {
      target: { value: "9" },
    });
    await waitFor(() =>
      expect(useMooseStore.getState().settings?.max_comments_per_hour).toBe(9),
    );

    fireEvent.click(screen.getByRole("checkbox", { name: /Quiet Hours/i }));
    await waitFor(() =>
      expect(useMooseStore.getState().settings?.quiet_hours_enabled).toBe(
        false,
      ),
    );

    fireEvent.click(screen.getByText("Personality"));
    const personalityUpdates = [
      ["dry", 0.1],
      ["sarcastic", 0.2],
      ["friendly", 0.3],
      ["absurd", 0.4],
      ["helpful", 0.5],
      ["verbosity", 0.6],
    ] as const;

    for (const [index, [key, value]] of personalityUpdates.entries()) {
      fireEvent.change(screen.getAllByRole("slider")[index], {
        target: { value: String(value) },
      });
      await waitFor(() =>
        expect(useMooseStore.getState().settings?.[key]).toBe(value),
      );
    }
  });

  it("offers explicit local and cloud ASR modes with privacy copy", async () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("Speech Recognition"));

    const tiny = await screen.findByRole("radio", {
      name: /Moonshine Tiny Streaming/,
    });
    expect(tiny).toBeChecked();
    expect(
      screen.getByText(/Only finalized transcript text is sent to Gemini/i),
    ).toBeInTheDocument();
    expect(screen.getAllByText("Not installed")).toHaveLength(2);

    fireEvent.click(
      screen.getByRole("radio", { name: /Gemini Live Cloud Audio/ }),
    );
    await waitFor(() =>
      expect(useMooseStore.getState().settings?.asr_mode).toBe(
        "gemini_live_audio",
      ),
    );
    expect(screen.getByText(/Cloud mode selected:/i)).toBeInTheDocument();
    expect(
      screen.getByText(
        /never switch to cloud microphone upload automatically/i,
      ),
    ).toBeInTheDocument();
  });

  it("preserves model selections and blocks writes while the Google catalog is loading", async () => {
    let resolveModels: (
      models: ReturnType<typeof frontendGoogleModels>,
    ) => void = () => undefined;
    const pendingModels = new Promise<ReturnType<typeof frontendGoogleModels>>(
      (resolve) => {
        resolveModels = resolve;
      },
    );
    vi.spyOn(tauriBridge, "getGoogleModels").mockReturnValue(pendingModels);
    const updateSpy = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockResolvedValue(undefined);

    render(<SettingsModal />);
    fireEvent.click(screen.getByText("AI & Models"));

    const liveSelect = screen.getByLabelText("Realtime Live Voice Model");
    const textSelect = screen.getByLabelText(
      "Gemini Text & Ambient Remark Model",
    );
    const settings = frontendDefaultSettings();

    expect(liveSelect).toBeDisabled();
    expect(textSelect).toBeDisabled();
    expect(liveSelect).toHaveValue(settings.live_model);
    expect(textSelect).toHaveValue(settings.google_text_model);
    expect(
      screen.getByRole("option", {
        name: new RegExp(
          `Current: ${settings.live_model}.*loading catalog`,
          "i",
        ),
      }),
    ).toBeInTheDocument();

    // Even a synthetic change during the disabled/loading window cannot persist.
    fireEvent.change(liveSelect, {
      target: { value: "unexpected-live-model" },
    });
    expect(updateSpy).not.toHaveBeenCalled();

    resolveModels(frontendGoogleModels());
    await waitFor(() => expect(liveSelect).not.toBeDisabled());
    expect(updateSpy).not.toHaveBeenCalled();
  });

  it("shows a persisted Google model as unavailable after catalog resolution without rewriting it", async () => {
    const settings = frontendDefaultSettings();
    vi.spyOn(tauriBridge, "getGoogleModels").mockResolvedValue([
      {
        id: "replacement-live",
        display_name: "Replacement Live",
        capabilities: ["live_audio"],
      },
      {
        id: "replacement-text",
        display_name: "Replacement Text",
        capabilities: ["text_generation"],
      },
    ]);
    const updateSpy = vi
      .spyOn(tauriBridge, "updateSettings")
      .mockResolvedValue(undefined);

    render(<SettingsModal />);
    fireEvent.click(screen.getByText("AI & Models"));

    const liveSelect = screen.getByLabelText("Realtime Live Voice Model");
    const textSelect = screen.getByLabelText(
      "Gemini Text & Ambient Remark Model",
    );
    await waitFor(() => expect(liveSelect).not.toBeDisabled());

    expect(liveSelect).toHaveValue(settings.live_model);
    expect(textSelect).toHaveValue(settings.google_text_model);
    expect(
      screen.getByRole("option", {
        name: `Unavailable: ${settings.live_model}`,
      }),
    ).toBeDisabled();
    expect(
      screen.getByRole("option", {
        name: `Unavailable: ${settings.google_text_model}`,
      }),
    ).toBeDisabled();
    expect(updateSpy).not.toHaveBeenCalled();
  });

  it("selects Local text without auto-downloading and manages the selected model explicitly", async () => {
    const installSpy = vi.spyOn(tauriBridge, "installLocalLlmModel");
    const testSpy = vi.spyOn(tauriBridge, "testLocalLlmModel");
    const deleteSpy = vi.spyOn(tauriBridge, "deleteLocalLlmModel");
    vi.spyOn(window, "confirm").mockReturnValue(true);

    render(<SettingsModal />);
    fireEvent.click(screen.getByText("AI & Models"));

    expect(
      screen.getByRole("radio", { name: /Google Gemini — cloud/i }),
    ).toBeChecked();
    fireEvent.click(
      screen.getByRole("radio", { name: /Local — on this computer/i }),
    );

    await waitFor(() =>
      expect(useMooseStore.getState().settings?.text_provider).toBe("local"),
    );
    expect(installSpy).not.toHaveBeenCalled();
    expect(
      screen.getByText(/Selecting a model never downloads it/i),
    ).toBeInTheDocument();

    const localSelect = await screen.findByLabelText("Local Text Model");
    expect(localSelect).toHaveValue("smollm2-360m-instruct-q4-k-m");
    expect(screen.getByText(/Smallest supported model/i)).toBeInTheDocument();

    fireEvent.change(localSelect, {
      target: { value: "qwen3-0-6b-instruct-q4-k-m" },
    });
    await waitFor(() =>
      expect(useMooseStore.getState().settings?.local_text_model).toBe(
        "qwen3-0-6b-instruct-q4-k-m",
      ),
    );
    expect(installSpy).not.toHaveBeenCalled();

    fireEvent.click(
      screen.getByRole("button", { name: /Download & Verify/i }),
    );
    expect(
      await screen.findByRole("button", { name: /Test Local Model/i }),
    ).toBeInTheDocument();
    expect(installSpy).toHaveBeenCalledWith("qwen3-0-6b-instruct-q4-k-m");

    fireEvent.click(
      screen.getByRole("button", { name: /Test Local Model/i }),
    );
    expect(await screen.findByText("Local model test succeeded")).toBeInTheDocument();
    expect(testSpy).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: /Delete Model/i }));
    await waitFor(() =>
      expect(deleteSpy).toHaveBeenCalledWith("qwen3-0-6b-instruct-q4-k-m"),
    );
    expect(useMooseStore.getState().settings?.local_text_model).toBe(
      "qwen3-0-6b-instruct-q4-k-m",
    );
    expect(
      await screen.findByRole("button", { name: /Download & Verify/i }),
    ).toBeInTheDocument();
  });

  it("makes the local/cloud text, voice, TTS and credential boundaries explicit", async () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("AI & Models"));
    fireEvent.click(
      screen.getByRole("radio", { name: /Local — on this computer/i }),
    );

    expect(
      await screen.findByText(/Not required for Local text generation/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Voice Conversation — Google Gemini Live/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Choosing Local text above does not change the Gemini Live voice provider/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Google TTS also receives the final reply text/i),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(/Google Gemini API Key/i)).toBeInTheDocument();
  });

  it("uses the backend-derived voice catalog in the primary voice selector", async () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("Voice & Audio"));

    const voiceSelect = screen.getByLabelText("Moose Voice Preset");
    await waitFor(() =>
      expect(voiceSelect.querySelectorAll("option")).toHaveLength(30),
    );
    expect(
      screen.getByRole("option", { name: /Fenrir \(Excitable\).*Default/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("option", { name: /Sulafat \(Warm\)/i }),
    ).toBeInTheDocument();
  });

  it("shows only current capability-filtered Gemini model options", async () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("AI & Models"));

    expect(
      await screen.findByRole("option", {
        name: /Gemini 3.1 Flash Live Preview.*gemini-3.1-flash-live-preview/i,
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("option", {
        name: /Gemini 3.7 Flash.*gemini-3.7-flash/i,
      }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("option", {
        name: /gemini-2.5-flash-native-audio/i,
      }),
    ).not.toBeInTheDocument();
  });

  it("exposes audio diagnostics and local microphone test controls", async () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("Diagnostics"));

    expect(screen.getByText("Audio Diagnostics")).toBeInTheDocument();
    expect(screen.getByText("Test Microphone")).toBeInTheDocument();
    expect(screen.getByText("Test Output")).toBeInTheDocument();
    expect(await screen.findByText("Granted")).toBeInTheDocument();
  });

  it("exposes local-time quiet hours with explicit overnight UX", async () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("Behavior"));

    expect(screen.getByLabelText("Quiet hours start")).toHaveValue("22");
    expect(screen.getByLabelText("Quiet hours end")).toHaveValue("8");
    expect(
      screen.getByText(/Overnight: quiet from 10:00 PM/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /Lower values make Moose require more important events/i,
      ),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Quiet hours start"), {
      target: { value: "7" },
    });
    await waitFor(() =>
      expect(useMooseStore.getState().settings?.quiet_hours_start).toBe(7),
    );
    expect(
      screen.getByText(/Same-day: quiet from 7:00 AM/i),
    ).toBeInTheDocument();
  });

  it("summarizes privacy and resets opt-in retention controls", async () => {
    useMooseStore.setState((state) => ({
      settings: state.settings
        ? {
            ...state.settings,
            active_app_observation: true,
            memory_enabled: true,
            save_transcripts: true,
          }
        : null,
    }));

    render(<SettingsModal />);
    fireEvent.click(screen.getByText("Privacy"));

    expect(screen.getByText("Active privacy summary")).toBeInTheDocument();
    expect(screen.getByText(/Window titles:/i)).toHaveTextContent("Off");
    fireEvent.click(
      screen.getByRole("button", { name: /Reset privacy defaults/i }),
    );

    await waitFor(() => {
      const settings = useMooseStore.getState().settings;
      expect(settings?.active_app_observation).toBe(false);
      expect(settings?.memory_enabled).toBe(false);
      expect(settings?.save_transcripts).toBe(false);
    });
  });

  it("documents focused-window shortcuts without global capture", () => {
    render(<SettingsModal />);
    expect(screen.getByText("Ctrl/Cmd + Enter")).toBeInTheDocument();
    expect(
      screen.getByText(/No global keyboard capture is registered/i),
    ).toBeInTheDocument();
  });

  // Tabs unmount when the user switches away, so transient Google credential state has to be
  // owned by the modal shell. Held inside a tab it would be silently discarded.
  it("preserves a half-typed API key across a tab switch", () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("AI & Models"));

    const keyField = screen.getByLabelText(/Google Gemini API Key/i);
    fireEvent.change(keyField, { target: { value: "AIzaSyPartialDraft" } });

    fireEvent.click(screen.getByText("Privacy"));
    fireEvent.click(screen.getByText("AI & Models"));

    expect(screen.getByLabelText(/Google Gemini API Key/i)).toHaveValue(
      "AIzaSyPartialDraft",
    );
  });

  it("updates key-presence state immediately when Settings saves a key", async () => {
    vi.spyOn(tauriBridge, "setGoogleApiKey").mockResolvedValue(undefined);

    render(<SettingsModal />);
    fireEvent.click(screen.getByText("AI & Models"));
    fireEvent.change(screen.getByLabelText(/Google Gemini API Key/i), {
      target: { value: "AIzaSySettingsKey" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Save Key/i }));

    await waitFor(() => expect(useMooseStore.getState().hasApiKey).toBe(true));
  });

  it("removes a saved key and updates key-presence state immediately", async () => {
    vi.spyOn(tauriBridge, "clearGoogleApiKey").mockResolvedValue(undefined);
    useMooseStore.setState({ hasApiKey: true });

    render(<SettingsModal />);
    fireEvent.click(screen.getByText("AI & Models"));
    fireEvent.click(screen.getByRole("button", { name: /Remove Saved Key/i }));

    await waitFor(() => expect(useMooseStore.getState().hasApiKey).toBe(false));
  });

  it("keeps a Google connection-test result visible across a tab switch", async () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("AI & Models"));
    fireEvent.click(
      screen.getByRole("button", { name: /Test Gemini Connection/i }),
    );

    const banner = await screen.findByText(/Test connection succeeded/i);

    fireEvent.click(screen.getByText("Privacy"));
    fireEvent.click(screen.getByText("AI & Models"));

    expect(screen.getByText(banner.textContent as string)).toBeInTheDocument();
  });
});
