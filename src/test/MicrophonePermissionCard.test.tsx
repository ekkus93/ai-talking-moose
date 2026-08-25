import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MicrophonePermissionCard } from "../components/Settings/MicrophonePermissionCard";
import { tauriBridge } from "../lib/tauriBridge";
import type { MicrophonePermissionState } from "../types/moose";

describe("MicrophonePermissionCard", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("shows a neutral loading state before the live permission query resolves", async () => {
    let resolvePermission: (state: MicrophonePermissionState) => void = () =>
      undefined;
    vi.spyOn(tauriBridge, "getMicrophonePermission").mockReturnValue(
      new Promise((resolve) => {
        resolvePermission = resolve;
      }),
    );

    render(<MicrophonePermissionCard />);

    expect(screen.getByRole("status")).toHaveTextContent("Checking…");
    expect(
      screen.queryByText(/unavailable on this platform or runtime/i),
    ).not.toBeInTheDocument();

    await act(async () => {
      resolvePermission("denied");
    });

    expect(screen.getByRole("status")).toHaveTextContent("Denied");
  });

  it("re-reads permission when the window regains focus after an external grant", async () => {
    const getPermission = vi
      .spyOn(tauriBridge, "getMicrophonePermission")
      .mockResolvedValueOnce("denied")
      .mockResolvedValueOnce("granted");

    render(<MicrophonePermissionCard />);
    expect(await screen.findByText("Denied")).toBeInTheDocument();

    act(() => {
      window.dispatchEvent(new Event("focus"));
    });

    expect(await screen.findByText("Granted")).toBeInTheDocument();
    expect(getPermission).toHaveBeenCalledTimes(2);
    expect(
      screen.queryByText(/access was denied or restricted/i),
    ).not.toBeInTheDocument();
  });

  it("offers the native request action while permission is not requested", async () => {
    vi.spyOn(tauriBridge, "getMicrophonePermission").mockResolvedValue(
      "not_requested",
    );
    const requestPermission = vi
      .spyOn(tauriBridge, "requestMicrophoneAccess")
      .mockResolvedValue("granted");

    render(<MicrophonePermissionCard />);

    const requestButton = await screen.findByRole("button", {
      name: "Request Microphone Access",
    });
    fireEvent.click(requestButton);

    expect(await screen.findByText("Granted")).toBeInTheDocument();
    expect(requestPermission).toHaveBeenCalledTimes(1);
  });
});
