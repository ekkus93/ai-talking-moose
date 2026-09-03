import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LocalLlmSettingsPanel } from "../components/Settings/LocalLlmSettingsPanel";
import { tauriBridge } from "../lib/tauriBridge";
import type {
  LocalModelDescriptor,
  LocalModelInstallProgress,
} from "../types/moose";

const MODEL_ID = "smollm2-360m-instruct-q4-k-m";
const SECOND_MODEL_ID = "qwen3-0-6b-instruct-q4-k-m";

const descriptor = (
  id: string,
  installState: LocalModelDescriptor["install_state"] = "not_installed",
  error: LocalModelDescriptor["error"] = null,
): LocalModelDescriptor => {
  const expectedBytes = (id === MODEL_ID ? 100 : 200) * 1024 * 1024;
  return {
    id,
    display_name: id === MODEL_ID ? "SmolLM2 360M" : "Qwen3 0.6B",
    family: id === MODEL_ID ? "SmolLM2" : "Qwen3",
    parameter_scale: id === MODEL_ID ? "360M" : "0.6B",
    quantization: "Q4_K_M",
    revision: "0123456789012345678901234567890123456789",
    expected_bytes: expectedBytes,
    installed_bytes: installState === "installed" ? expectedBytes : null,
    license: "Apache-2.0",
    context_limit: 8192,
    recommended_max_output: 192,
    install_state: installState,
    active: id === MODEL_ID,
    error,
  };
};

const originalBridgeMethods = {
  getLocalLlmModels: tauriBridge.getLocalLlmModels,
  installLocalLlmModel: tauriBridge.installLocalLlmModel,
  cancelLocalLlmInstall: tauriBridge.cancelLocalLlmInstall,
  deleteLocalLlmModel: tauriBridge.deleteLocalLlmModel,
  testLocalLlmModel: tauriBridge.testLocalLlmModel,
  onLocalLlmModelProgress: tauriBridge.onLocalLlmModelProgress,
};

describe("LocalLlmSettingsPanel residual lifecycle coverage", () => {
  let progressListener:
    ((progress: LocalModelInstallProgress) => void) | undefined;

  beforeEach(() => {
    vi.spyOn(tauriBridge, "getLocalLlmModels").mockResolvedValue([
      descriptor(MODEL_ID),
      descriptor(SECOND_MODEL_ID),
    ]);
    vi.spyOn(tauriBridge, "onLocalLlmModelProgress").mockImplementation(
      async (listener) => {
        progressListener = listener;
        return () => undefined;
      },
    );
  });

  afterEach(() => {
    Object.assign(tauriBridge, originalBridgeMethods);
    vi.restoreAllMocks();
    progressListener = undefined;
  });

  it("renders live download progress and sends an explicit cancel request", async () => {
    let resolveInstall: ((value: LocalModelDescriptor) => void) | undefined;
    vi.spyOn(tauriBridge, "installLocalLlmModel").mockImplementation(
      () =>
        new Promise<LocalModelDescriptor>((resolve) => {
          resolveInstall = resolve;
        }),
    );
    const cancelSpy = vi
      .spyOn(tauriBridge, "cancelLocalLlmInstall")
      .mockResolvedValue(true);

    render(
      <LocalLlmSettingsPanel
        selectedModelId={MODEL_ID}
        onSelectModel={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    fireEvent.click(
      await screen.findByRole("button", { name: /Download & Verify/i }),
    );
    expect(
      await screen.findByRole("button", { name: /Cancel Download/i }),
    ).toBeInTheDocument();

    await act(async () => {
      progressListener?.({
        model_id: MODEL_ID,
        install_state: "downloading",
        downloaded_bytes: 50 * 1024 * 1024,
        total_bytes: 100 * 1024 * 1024,
      });
    });
    expect(
      screen.getByLabelText("Local model download progress"),
    ).toBeInTheDocument();
    expect(screen.getByText("50 MiB / 100 MiB")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Cancel Download/i }));
    await waitFor(() => expect(cancelSpy).toHaveBeenCalledWith(MODEL_ID));
    expect(
      await screen.findByText(/Cancellation requested for SmolLM2 360M/i),
    ).toBeInTheDocument();

    await act(async () => {
      resolveInstall?.(descriptor(MODEL_ID, "installed"));
    });
  });

  it("renders verification progress distinctly from download progress", async () => {
    render(
      <LocalLlmSettingsPanel
        selectedModelId={MODEL_ID}
        onSelectModel={vi.fn().mockResolvedValue(undefined)}
      />,
    );
    await screen.findByLabelText("Local Text Model");

    await act(async () => {
      progressListener?.({
        model_id: MODEL_ID,
        install_state: "verifying",
        downloaded_bytes: 100 * 1024 * 1024,
        total_bytes: 100 * 1024 * 1024,
      });
    });

    expect(
      screen.getByText(/Verifying SHA-256 and byte count/i),
    ).toBeInTheDocument();
    expect(screen.getByText("Verifying")).toBeInTheDocument();
  });

  it("surfaces install failure and refreshes authoritative failed status", async () => {
    const failed = descriptor(MODEL_ID, "failed", {
      kind: "sha256_mismatch",
      message: "The downloaded local model failed SHA-256 verification.",
      retryable: true,
    });
    vi.mocked(tauriBridge.getLocalLlmModels)
      .mockResolvedValueOnce([
        descriptor(MODEL_ID),
        descriptor(SECOND_MODEL_ID),
      ])
      .mockResolvedValueOnce([failed, descriptor(SECOND_MODEL_ID)]);
    vi.spyOn(tauriBridge, "installLocalLlmModel").mockRejectedValue(
      new Error("install rejected"),
    );

    render(
      <LocalLlmSettingsPanel
        selectedModelId={MODEL_ID}
        onSelectModel={vi.fn().mockResolvedValue(undefined)}
      />,
    );
    fireEvent.click(
      await screen.findByRole("button", { name: /Download & Verify/i }),
    );

    expect(await screen.findByText(/install rejected/i)).toBeInTheDocument();
    expect(
      await screen.findByText(
        "The downloaded local model failed SHA-256 verification.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("Install failed")).toBeInTheDocument();
  });

  it("rolls back optimistic download state when install and status refresh both fail", async () => {
    vi.mocked(tauriBridge.getLocalLlmModels)
      .mockResolvedValueOnce([
        descriptor(MODEL_ID),
        descriptor(SECOND_MODEL_ID),
      ])
      .mockRejectedValueOnce(new Error("status refresh rejected"));
    vi.spyOn(tauriBridge, "installLocalLlmModel").mockRejectedValue(
      new Error("install rejected"),
    );

    render(
      <LocalLlmSettingsPanel
        selectedModelId={MODEL_ID}
        onSelectModel={vi.fn().mockResolvedValue(undefined)}
      />,
    );
    fireEvent.click(
      await screen.findByRole("button", { name: /Download & Verify/i }),
    );

    expect(
      await screen.findByText(
        /Could not read local model status:.*status refresh rejected/i,
      ),
    ).toBeInTheDocument();
    await waitFor(() =>
      expect(
        screen.queryByRole("button", { name: /Cancel Download/i }),
      ).not.toBeInTheDocument(),
    );
    expect(screen.getByText("Not installed")).toBeInTheDocument();
  });

  it("reports backend selection rejection without selecting a replacement", async () => {
    const onSelectModel = vi
      .fn<(modelId: string) => Promise<void>>()
      .mockRejectedValue(new Error("selection persistence rejected"));

    render(
      <LocalLlmSettingsPanel
        selectedModelId={MODEL_ID}
        onSelectModel={onSelectModel}
      />,
    );
    const selector = await screen.findByLabelText("Local Text Model");
    expect(selector).toHaveValue(MODEL_ID);

    fireEvent.change(selector, { target: { value: SECOND_MODEL_ID } });

    await waitFor(() =>
      expect(onSelectModel).toHaveBeenCalledWith(SECOND_MODEL_ID),
    );
    expect(
      await screen.findByText(/selection persistence rejected/i),
    ).toBeInTheDocument();
    expect(selector).toHaveValue(MODEL_ID);
  });
});
