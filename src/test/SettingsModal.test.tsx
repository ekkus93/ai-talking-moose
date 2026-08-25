import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, beforeEach } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { useMooseStore } from "../stores/mooseStore";
import { frontendDefaultSettings } from "../lib/backendContract";

describe("SettingsModal Component", () => {
  beforeEach(() => {
    useMooseStore.setState({
      isSettingsOpen: true,
      settings: frontendDefaultSettings(),
    });
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

  it("shows only current capability-filtered Gemini model options", async () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("Gemini AI"));

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
    expect(await screen.findByText("Unavailable")).toBeInTheDocument();
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

  // Tabs unmount when the user switches away, so transient tab state has to be
  // owned by the modal shell. Held inside a tab it would be silently discarded.
  it("preserves a half-typed API key across a tab switch", () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("Gemini AI"));

    const keyField = screen.getByLabelText(/Google Gemini API Key/i);
    fireEvent.change(keyField, { target: { value: "AIzaSyPartialDraft" } });

    fireEvent.click(screen.getByText("Privacy"));
    fireEvent.click(screen.getByText("Gemini AI"));

    expect(screen.getByLabelText(/Google Gemini API Key/i)).toHaveValue(
      "AIzaSyPartialDraft",
    );
  });

  it("keeps a connection-test result visible across a tab switch", async () => {
    render(<SettingsModal />);
    fireEvent.click(screen.getByText("Gemini AI"));
    fireEvent.click(
      screen.getByRole("button", { name: /Test Gemini Connection/i }),
    );

    const banner = await screen.findByText(/Mock API connection successful/i);

    fireEvent.click(screen.getByText("Privacy"));
    fireEvent.click(screen.getByText("Gemini AI"));

    expect(screen.getByText(banner.textContent as string)).toBeInTheDocument();
  });
});
