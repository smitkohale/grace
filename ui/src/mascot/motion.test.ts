import { describe, expect, it } from "vitest";
import {
  approach,
  burst,
  clampLook,
  hopHeight,
  MAX_PITCH,
  MAX_YAW,
  nextBlinkDelay,
  poseFor,
  waveAngle,
} from "./motion";

describe("Grace's motion", () => {
  it("never turns her head further than the limits", () => {
    expect(clampLook(0, 0)).toEqual({ yaw: 0, pitch: 0 });
    expect(clampLook(10, -10)).toEqual({ yaw: MAX_YAW, pitch: -MAX_PITCH });
    expect(clampLook(-0.5, 0.5).yaw).toBeCloseTo(-MAX_YAW / 2);
  });

  it("blinks every 2.5 to 6 seconds", () => {
    expect(nextBlinkDelay(() => 0)).toBe(2500);
    expect(nextBlinkDelay(() => 1)).toBe(6000);
  });

  it("eases toward a target at any frame rate", () => {
    expect(approach(0, 1, 10, 0)).toBe(0);
    expect(approach(0, 1, 10, 10)).toBeCloseTo(1);
    const twoSmallSteps = approach(approach(0, 1, 5, 0.05), 1, 5, 0.05);
    expect(twoSmallSteps).toBeCloseTo(approach(0, 1, 5, 0.1));
  });

  it("closes her eyes when sleeping and orbits the cubes when thinking", () => {
    expect(poseFor("sleeping").eyeOpen).toBeLessThan(0.2);
    expect(poseFor("thinking").cubes).toBe("orbit");
    expect(poseFor("idle").eyeOpen).toBe(1);
  });

  it("keeps the bob subtle", () => {
    for (const state of ["hello", "thinking", "idle", "sleeping"] as const) {
      expect(poseFor(state).bob).toBeLessThanOrEqual(0.015);
    }
  });

  it("waves, hops and bursts only while running", () => {
    expect(waveAngle(0)).toBe(0);
    expect(waveAngle(1)).toBe(0);
    expect(waveAngle(0.5)).toBeGreaterThan(2);
    expect(hopHeight(0.5)).toBeCloseTo(0.08);
    expect(hopHeight(1.2)).toBe(0);
    expect(burst(0.5)).toBeGreaterThan(0);
    expect(burst(1)).toBe(0);
  });
});
