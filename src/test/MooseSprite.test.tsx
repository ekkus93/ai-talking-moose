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
    const { container } = render(<MooseSprite state="talking" mouth="wide" isBlinking={false} />);
    expect(container.innerHTML).toContain("#d32f2f"); // Wide mouth color
  });

  it("renders sleeping state with Zzz", () => {
    const { container } = render(
      <MooseSprite state="sleeping" mouth="closed" isBlinking={false} />
    );
    expect(container.innerHTML).toContain("Zzz");
  });
});
