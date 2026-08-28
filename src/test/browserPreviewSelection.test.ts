import { afterEach, describe, expect, it, vi } from "vitest";

describe("Vite development preview selection", () => {
  const originalInternals = Object.getOwnPropertyDescriptor(
    window,
    "__TAURI_INTERNALS__",
  );

  afterEach(() => {
    if (originalInternals) {
      Object.defineProperty(window, "__TAURI_INTERNALS__", originalInternals);
    }
    vi.resetModules();
  });

  it("selects the explicit preview adapter when Vite dev has no Tauri backend", async () => {
    delete (
      window as unknown as {
        __TAURI_INTERNALS__?: unknown;
      }
    ).__TAURI_INTERNALS__;
    vi.resetModules();

    const [{ tauriBridge }, { browserPreviewBridge }] = await Promise.all([
      import("../lib/tauriBridge"),
      import("../lib/browserPreviewBridge"),
    ]);

    expect(tauriBridge).toBe(browserPreviewBridge);
    expect(await tauriBridge.getSettings()).toBeDefined();
  });
});
