import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";

function renderOpenaiSettings() {
  useMooseStore.setState({
    isSettingsOpen: true,
    settings: {
      ...frontendDefaultSettings(),
      text_provider: "openai",
      asr_mode: "whisper_small",
    },
    hasApiKey: true,
    settingsPersistenceError: null,
  });
  render(<SettingsModal />);
  fireEvent.click(screen.getByText("AI & Models"));
}

describe("OpenAI credential isolation in Settings", () => {
  afterEach(() => vi.restoreAllMocks());

  it("saves only through the secret bridge, clears the input, and preserves Google status", async () => {
    vi.spyOn(tauriBridge, "hasOpenaiApiKey").mockResolvedValue(false);
    const saveOpenai = vi
      .spyOn(tauriBridge, "setOpenaiApiKey")
      .mockResolvedValue(undefined);
    const saveGoogle = vi.spyOn(tauriBridge, "setGoogleApiKey");
    renderOpenaiSettings();

    await screen.findByText("No OpenAI key is configured.");
    fireEvent.change(screen.getByLabelText("OpenAI API key (BYOK)"), {
      target: { value: "  test-openai-secret  " },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save OpenAI Key" }));

    await waitFor(() =>
      expect(saveOpenai).toHaveBeenCalledWith("test-openai-secret"),
    );
    expect(
      await screen.findByRole("button", { name: "Remove Saved OpenAI Key" }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("OpenAI API key (BYOK)")).toHaveValue("");
    expect(saveGoogle).not.toHaveBeenCalled();
    expect(useMooseStore.getState().hasApiKey).toBe(true);
    expect(useMooseStore.getState().settings?.text_provider).toBe("openai");
    expect(JSON.stringify(useMooseStore.getState().settings)).not.toContain(
      "test-openai-secret",
    );
  });

  it("removes only the OpenAI credential without clearing Google's key status", async () => {
    vi.spyOn(tauriBridge, "hasOpenaiApiKey").mockResolvedValue(true);
    const clearOpenai = vi
      .spyOn(tauriBridge, "clearOpenaiApiKey")
      .mockResolvedValue(undefined);
    const clearGoogle = vi.spyOn(tauriBridge, "clearGoogleApiKey");
    renderOpenaiSettings();

    fireEvent.click(
      await screen.findByRole("button", { name: "Remove Saved OpenAI Key" }),
    );
    await waitFor(() => expect(clearOpenai).toHaveBeenCalledOnce());
    expect(
      await screen.findByText("No OpenAI key is configured."),
    ).toBeInTheDocument();
    expect(clearGoogle).not.toHaveBeenCalled();
    expect(useMooseStore.getState().hasApiKey).toBe(true);
    expect(useMooseStore.getState().settings?.text_provider).toBe("openai");
  });

  it("ignores an old key-presence result after a successful key save", async () => {
    let resolvePresence: (value: boolean) => void = () => {
      throw new Error("missing pending resolver");
    };
    vi.spyOn(tauriBridge, "hasOpenaiApiKey").mockReturnValue(
      new Promise<boolean>((resolve) => {
        resolvePresence = resolve;
      }),
    );
    vi.spyOn(tauriBridge, "setOpenaiApiKey").mockResolvedValue(undefined);
    renderOpenaiSettings();

    fireEvent.change(screen.getByLabelText("OpenAI API key (BYOK)"), {
      target: { value: "fresh-secret" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save OpenAI Key" }));
    expect(
      await screen.findByRole("button", { name: "Remove Saved OpenAI Key" }),
    ).toBeInTheDocument();

    resolvePresence(false);
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Remove Saved OpenAI Key" }),
      ).toBeInTheDocument(),
    );
    expect(useMooseStore.getState().settings?.text_provider).toBe("openai");
  });

  it("sanitizes credential-store errors rather than displaying raw exceptions", async () => {
    vi.spyOn(tauriBridge, "hasOpenaiApiKey").mockResolvedValue(false);
    vi.spyOn(tauriBridge, "setOpenaiApiKey").mockRejectedValue(
      new Error("private-provider-key-must-not-appear"),
    );
    renderOpenaiSettings();

    await screen.findByText("No OpenAI key is configured.");
    fireEvent.change(screen.getByLabelText("OpenAI API key (BYOK)"), {
      target: { value: "test-openai-secret" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save OpenAI Key" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Could not securely store the OpenAI key.");
    expect(alert).not.toHaveTextContent("private-provider-key-must-not-appear");
    expect(
      screen.queryByRole("button", { name: "Remove Saved OpenAI Key" }),
    ).not.toBeInTheDocument();
  });
});
