import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { SettingsModal } from "../components/Settings/SettingsModal";
import { frontendDefaultSettings } from "../lib/backendContract";
import { tauriBridge } from "../lib/tauriBridge";
import { useMooseStore } from "../stores/mooseStore";

afterEach(() => vi.restoreAllMocks());

it("does not change selected Ollama provider after an unreachable readiness result", async () => {
  const check = vi
    .spyOn(tauriBridge, "testLocalTextServer")
    .mockResolvedValue("unreachable");
  useMooseStore.setState({
    isSettingsOpen: true,
    settings: {
      ...frontendDefaultSettings(),
      text_provider: "ollama",
      asr_mode: "whisper_small",
    },
    settingsPersistenceError: null,
  });
  render(<SettingsModal />);
  fireEvent.click(screen.getByText("AI & Models"));
  fireEvent.click(
    screen.getByRole("button", { name: "Check Server and Model" }),
  );
  await waitFor(() => expect(check).toHaveBeenCalledWith("ollama"));
  expect(await screen.findByRole("status")).toHaveTextContent(
    "Server is unreachable",
  );
  expect(useMooseStore.getState().settings?.text_provider).toBe("ollama");
});
