import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";

const renderOllama = () => {
  useMooseStore.setState({
    isSettingsOpen: true,
    settings: {
      ...frontendDefaultSettings(),
      text_provider: "ollama",
      asr_mode: "whisper_small",
    },
    settingsPersistenceError: null,
    hasApiKey: false,
  });
  render(<SettingsModal />);
  fireEvent.click(screen.getByText("AI & Models"));
};

describe("external server readiness snapshot isolation", () => {
  afterEach(() => vi.restoreAllMocks());

  it("does not show readiness from an obsolete model selection", async () => {
    let resolveCheck!: (value: "model_ready") => void;
    vi.spyOn(tauriBridge, "testLocalTextServer").mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveCheck = resolve;
        }),
    );
    renderOllama();
    fireEvent.click(
      screen.getByRole("button", { name: "Check Server and Model" }),
    );
    expect(screen.getByRole("status")).toHaveTextContent(
      /Checking loopback server/i,
    );

    await act(async () => {
      useMooseStore.setState((state) => ({
        settings: state.settings
          ? { ...state.settings, ollama_text_model: "replacement-model" }
          : null,
      }));
      resolveCheck("model_ready");
    });
    expect(
      screen.queryByText("Connected: selected model is available."),
    ).not.toBeInTheDocument();
  });

  it("only publishes the most recent overlapping check", async () => {
    const complete: Array<(value: "model_ready" | "unreachable") => void> = [];
    vi.spyOn(tauriBridge, "testLocalTextServer").mockImplementation(
      () =>
        new Promise((resolve) => {
          complete.push(resolve);
        }),
    );
    renderOllama();
    const check = screen.getByRole("button", {
      name: "Check Server and Model",
    });
    fireEvent.click(check);
    fireEvent.click(check);
    expect(complete).toHaveLength(2);

    await act(async () => complete[0]("model_ready"));
    expect(screen.getByRole("status")).toHaveTextContent(
      /Checking loopback server/i,
    );

    await act(async () => complete[1]("unreachable"));
    expect(screen.getByRole("status")).toHaveTextContent(
      /Server is unreachable/i,
    );
  });
});
