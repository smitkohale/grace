import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { GraceMascot } from "./GraceMascot";

const { createGraceScene } = vi.hoisted(() => ({ createGraceScene: vi.fn() }));
vi.mock("./scene", () => ({ createGraceScene }));

describe("GraceMascot without WebGL", () => {
  it("shows a still of the same model, with no canvas, and keeps her label", () => {
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
    const { container } = render(<GraceMascot state="sleeping" size="card" />);
    expect(container.querySelector("img")).not.toBeNull();
    expect(container.querySelector("canvas")).toBeNull();
    expect(screen.getByRole("img")).toHaveAccessibleName();
    expect(createGraceScene).not.toHaveBeenCalled();
  });
});
