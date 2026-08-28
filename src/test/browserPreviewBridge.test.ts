import { describe, expect, it } from "vitest";
import { browserPreviewBridge } from "../lib/browserPreviewBridge";

describe("development-only browser preview adapter", () => {
  it("provides benign read-only presentation data without Tauri", async () => {
    expect(await browserPreviewBridge.getCharacterState()).toBe("idle");
    expect(await browserPreviewBridge.getMemories()).toEqual([]);
    expect(await browserPreviewBridge.getTranscripts()).toEqual([]);
  });

  it("keeps simulated effects isolated to this explicit adapter", async () => {
    expect(await browserPreviewBridge.hasGoogleApiKey()).toBe(true);
    expect(await browserPreviewBridge.testAiConnection()).toEqual({
      success: true,
      message: "Mock API connection successful!",
    });
    expect(await browserPreviewBridge.startConversation()).toBe(
      "mock-sess-123",
    );
    expect(await browserPreviewBridge.deleteMemory(1)).toBe(true);
    expect(await browserPreviewBridge.sendTextMessage("hello")).toBe(
      'Mock reply to: "hello"',
    );
  });
});
