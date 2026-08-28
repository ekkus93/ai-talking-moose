import { describe, expect, it } from "vitest";
import { browserPreviewBridge } from "../lib/browserPreviewBridge";
import {
  isTauri,
  nativeTauriBridge,
  selectTauriBridge,
  tauriBridge,
} from "../lib/tauriBridge";

describe("Tauri bridge runtime selection", () => {
  it("uses the native IPC adapter in production-like frontend tests", () => {
    expect(isTauri()).toBe(true);
    expect(tauriBridge).toBe(nativeTauriBridge);
  });

  it("prefers native IPC even when a development preview is allowed", () => {
    expect(selectTauriBridge(true, true)).toBe(nativeTauriBridge);
  });

  it("fails closed when production has no Tauri backend", () => {
    expect(() => selectTauriBridge(false, false)).toThrow(
      /Tauri IPC is unavailable/,
    );
  });

  it("treats malformed Tauri internals as unavailable and fails closed", () => {
    const descriptor = Object.getOwnPropertyDescriptor(
      window,
      "__TAURI_INTERNALS__",
    );
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });

    try {
      expect(isTauri()).toBe(false);
      expect(() => selectTauriBridge(isTauri(), false)).toThrow(
        /Tauri IPC is unavailable/,
      );
    } finally {
      if (descriptor) {
        Object.defineProperty(window, "__TAURI_INTERNALS__", descriptor);
      }
    }
  });

  it("cannot enable preview through ordinary browser runtime state", () => {
    window.history.replaceState({}, "", "/?preview=true");
    window.localStorage.setItem("talking-moose-preview", "true");
    Object.assign(window, { __TALKING_MOOSE_PREVIEW__: true });

    expect(() => selectTauriBridge(false, false)).toThrow(
      /Tauri IPC is unavailable/,
    );

    window.localStorage.removeItem("talking-moose-preview");
    delete (window as unknown as { __TALKING_MOOSE_PREVIEW__?: boolean })
      .__TALKING_MOOSE_PREVIEW__;
    window.history.replaceState({}, "", "/");
  });

  it("exposes preview only through the explicit development selection", () => {
    expect(selectTauriBridge(false, true)).toBe(browserPreviewBridge);
  });
});
