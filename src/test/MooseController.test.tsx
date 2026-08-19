import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { MooseController } from "../components/Moose/MooseController";
import { useMooseStore } from "../stores/mooseStore";

describe("MooseController dismissal visibility", () => {
  beforeEach(() => {
    useMooseStore.setState({
      characterState: "idle",
      mouthShape: "closed",
      isBlinking: false,
      isConversationActive: false,
    });
  });

  it.each(["hidden", "dismissed"] as const)(
    "does not render the Moose while %s",
    (characterState) => {
      useMooseStore.setState({ characterState });

      render(<MooseController />);

      expect(screen.queryByTestId("moose-sprite")).not.toBeInTheDocument();
    },
  );

  it("renders again after the authoritative state returns to idle", () => {
    useMooseStore.setState({ characterState: "idle" });

    render(<MooseController />);

    expect(screen.getByTestId("moose-sprite")).toBeInTheDocument();
  });
});
