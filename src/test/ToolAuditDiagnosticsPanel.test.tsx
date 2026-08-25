import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ToolAuditDiagnosticsPanel } from "../components/Settings/ToolAuditDiagnosticsPanel";
import { tauriBridge } from "../lib/tauriBridge";

describe("ToolAuditDiagnosticsPanel", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("shows privacy-safe tool audit metadata including execution duration", async () => {
    vi.spyOn(tauriBridge, "getToolAudit").mockResolvedValue([
      {
        tool_name: "remember_fact",
        timestamp: "2026-08-25T09:12:34.000Z",
        duration_ms: 17,
        permission: "memory_mutation",
        permission_outcome: "allowed",
        result_category: "success",
      },
    ]);

    render(<ToolAuditDiagnosticsPanel />);

    expect(await screen.findByText("remember_fact")).toBeInTheDocument();
    expect(screen.getByText("17 ms")).toBeInTheDocument();
    expect(screen.getByText("allowed")).toBeInTheDocument();
    expect(screen.getByText("success")).toBeInTheDocument();
    expect(
      screen.getByText(/Raw tool arguments and results are never included/i),
    ).toBeInTheDocument();
  });
});
