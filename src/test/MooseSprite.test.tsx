import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { MooseSprite } from "../components/Moose/MooseSprite";

describe("MooseSprite Component", () => {
  it("renders the pixel art SVG for idle state", () => {
    render(<MooseSprite state="idle" mouth="closed" isBlinking={false} />);
    const sprite = screen.getByTestId("moose-sprite");
    expect(sprite).toBeInTheDocument();
    expect(sprite.innerHTML).toContain("<svg");
  });

  it("renders different SVG for talking state with wide mouth", () => {
    const { container } = render(
      <MooseSprite state="talking" mouth="wide" isBlinking={false} />,
    );
    expect(container.innerHTML).toContain("#d32f2f"); // Wide mouth color
  });

  it("uses a keyboard-operable control and crisp 32x32 render grid", () => {
    render(<MooseSprite state="idle" mouth="closed" isBlinking={false} />);
    const sprite = screen.getByRole("button", { name: "Talk to Moose" });
    const svg = sprite.querySelector("svg");

    expect(sprite).toHaveClass("pixelated-sprite");
    expect(svg).toHaveAttribute("viewBox", "0 0 32 32");
    expect(svg).toHaveAttribute("shape-rendering", "crispEdges");
  });
});
