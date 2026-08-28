import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

describe("frontend-only Vite preview rendering", () => {
  const originalInternals = Object.getOwnPropertyDescriptor(
    window,
    "__TAURI_INTERNALS__",
  );

  afterEach(() => {
    cleanup();
    if (originalInternals) {
      Object.defineProperty(window, "__TAURI_INTERNALS__", originalInternals);
    }
    vi.resetModules();
  });

  it("renders the actual Moose window without a Tauri backend", async () => {
    delete (
      window as unknown as {
        __TAURI_INTERNALS__?: unknown;
      }
    ).__TAURI_INTERNALS__;
    vi.resetModules();

    const { default: App } = await import("../app/App");
    render(<App />);

    expect(await screen.findByTestId("moose-window")).toBeInTheDocument();
    expect(screen.getByText("The Talking Moose")).toBeInTheDocument();
  });
});
