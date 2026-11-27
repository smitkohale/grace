import { render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { GraceMascot } from "./GraceMascot";

const { fakeScene, createGraceScene } = vi.hoisted(() => {
  const fakeScene = { setState: vi.fn(), setPaused: vi.fn(), press: vi.fn(), dispose: vi.fn() };
  return {
    fakeScene,
    createGraceScene: vi.fn(async (_canvas: HTMLCanvasElement, _options: unknown) => fakeScene),
  };
});
vi.mock("./scene", () => ({ createGraceScene }));

const canvasOf = (call: number): HTMLCanvasElement => {
  const canvas = createGraceScene.mock.calls[call]?.[0];
  if (!canvas) throw new Error(`the scene wasn't created ${call + 1} times`);
  return canvas;
};

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

// The WebGL check runs once per page, so the no-WebGL case lives in
// GraceMascot.flat.test.tsx, which gets a page of its own.
describe("GraceMascot with WebGL", () => {
  it("draws on a fresh canvas and hands the GPU memory back when she leaves", async () => {
    const loseContext = vi.fn();
    const getContext = vi
      .spyOn(HTMLCanvasElement.prototype, "getContext")
      .mockReturnValue({ getExtension: () => ({ loseContext }) } as unknown as RenderingContext);
    vi.stubGlobal(
      "IntersectionObserver",
      class {
        observe() {}
        disconnect() {}
      },
    );

    const first = render(<GraceMascot state="hello" size="hero" />);
    await waitFor(() => expect(first.container.querySelector("img")).toBeNull());
    expect(createGraceScene).toHaveBeenCalledTimes(1);
    const canvas = canvasOf(0);
    expect(canvas.isConnected).toBe(true);
    // The check's own context is released straight away.
    expect(loseContext).toHaveBeenCalledTimes(1);

    first.unmount();
    expect(fakeScene.dispose).toHaveBeenCalledTimes(1);
    expect(canvas.isConnected).toBe(false);

    // Coming back draws on a new canvas, without checking WebGL again.
    const second = render(<GraceMascot state="idle" size="card" />);
    await waitFor(() => expect(createGraceScene).toHaveBeenCalledTimes(2));
    expect(canvasOf(1)).not.toBe(canvas);
    expect(getContext).toHaveBeenCalledTimes(1);
    second.unmount();
  });
});
