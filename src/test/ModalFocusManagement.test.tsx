import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { OnboardingModal } from "../components/Onboarding/OnboardingModal";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { TranscriptDrawer } from "../components/Transcript/TranscriptDrawer";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";

const createFocusedInvoker = (label: string) => {
  const button = document.createElement("button");
  button.textContent = label;
  document.body.append(button);
  button.focus();
  return button;
};

describe("modal focus management", () => {
  let invoker: HTMLButtonElement | null = null;

  beforeEach(() => {
    useMooseStore.setState({
      isSettingsOpen: false,
      isOnboardingOpen: false,
      isTranscriptOpen: false,
      settings: frontendDefaultSettings(),
      transcripts: [],
      partialUserTranscript: null,
      partialMooseTranscript: null,
    });
  });

  afterEach(() => {
    invoker?.remove();
    invoker = null;
    vi.restoreAllMocks();
  });

  it("traps Settings focus and restores its invoker", async () => {
    invoker = createFocusedInvoker("settings invoker");
    useMooseStore.setState({ isSettingsOpen: true });

    render(<SettingsModal />);
    const dialog = screen.getByRole("dialog", {
      name: "TALKING MOOSE CONTROL PANEL",
    });
    const close = within(dialog).getByRole("button", {
      name: "Close settings",
    });
    const done = within(dialog).getByRole("button", { name: "Done" });

    await waitFor(() => expect(close).toHaveFocus());

    done.focus();
    fireEvent.keyDown(document, { key: "Tab" });
    expect(close).toHaveFocus();

    close.focus();
    fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
    expect(done).toHaveFocus();

    fireEvent.click(close);
    await waitFor(() => {
      expect(screen.queryByTestId("settings-modal")).not.toBeInTheDocument();
      expect(invoker).toHaveFocus();
    });
  });

  it("traps Onboarding focus and restores its invoker", async () => {
    vi.spyOn(tauriBridge, "getAsrModels").mockResolvedValue([]);
    vi.spyOn(tauriBridge, "acknowledgeOnboarding").mockResolvedValue({
      current_version: 1,
      acknowledged_version: 1,
      needs_acknowledgement: false,
    });
    invoker = createFocusedInvoker("onboarding invoker");
    useMooseStore.setState({ isOnboardingOpen: true });

    render(<OnboardingModal />);
    const dialog = screen.getByRole("dialog", { name: "TALKING MOOSE AI" });
    const skip = within(dialog).getByRole("button", {
      name: "Skip onboarding",
    });
    const next = within(dialog).getByRole("button", { name: "Next: Voice" });

    await waitFor(() => expect(skip).toHaveFocus());

    next.focus();
    fireEvent.keyDown(document, { key: "Tab" });
    expect(skip).toHaveFocus();

    skip.focus();
    fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
    expect(next).toHaveFocus();

    fireEvent.click(skip);
    await waitFor(() => {
      expect(screen.queryByTestId("onboarding-modal")).not.toBeInTheDocument();
      expect(invoker).toHaveFocus();
    });
  });

  it("traps Transcript focus and restores its invoker", async () => {
    invoker = createFocusedInvoker("transcript invoker");
    useMooseStore.setState({ isTranscriptOpen: true });

    render(<TranscriptDrawer />);
    const dialog = screen.getByRole("dialog", { name: "MOOSE DEBUG TERMINAL" });
    const clear = within(dialog).getByRole("button", {
      name: "Forget all stored Moose data",
    });
    const close = within(dialog).getByRole("button", {
      name: "Close transcript terminal",
    });
    const input = within(dialog).getByRole("textbox", {
      name: "Message Moose",
    });

    await waitFor(() => expect(input).toHaveFocus());

    input.focus();
    fireEvent.keyDown(document, { key: "Tab" });
    expect(clear).toHaveFocus();

    clear.focus();
    fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
    expect(input).toHaveFocus();

    fireEvent.click(close);
    await waitFor(() => {
      expect(screen.queryByTestId("transcript-drawer")).not.toBeInTheDocument();
      expect(invoker).toHaveFocus();
    });
  });
});
