// Pure motion maths for Grace (no three.js here, so it's easy to test).
// Motion stays subtle (DESIGN §5): a 2 px bob, a blink every few seconds,
// slowly floating cubes.

export type MascotState = "hello" | "thinking" | "idle" | "sleeping";

/** How far her head turns toward the pointer, in radians. */
export const MAX_YAW = 0.35;
export const MAX_PITCH = 0.2;

const clamp = (value: number, limit: number) => Math.max(-limit, Math.min(limit, value));

/** Head turn for a pointer offset, where -1..1 spans the window. */
export function clampLook(dx: number, dy: number): { yaw: number; pitch: number } {
  return { yaw: clamp(dx * MAX_YAW, MAX_YAW), pitch: clamp(dy * MAX_PITCH, MAX_PITCH) };
}

/** Milliseconds until the next blink: every 2.5 to 6 seconds. */
export function nextBlinkDelay(random: () => number = Math.random): number {
  return 2500 + random() * 3500;
}

/** Moves `current` toward `target` smoothly, independent of frame rate. */
export function approach(current: number, target: number, rate: number, dt: number): number {
  return target + (current - target) * Math.exp(-rate * dt);
}

export interface Pose {
  /** 1 = eyes open, ~0.1 = closed. */
  eyeOpen: number;
  headPitch: number;
  headRoll: number;
  /** Extra tilt of the top strand (positive perks it up). */
  ahoge: number;
  cubes: "float" | "orbit" | "rest";
  /** Bob height in model units (≈ 2 px at hero size) and its period in seconds. */
  bob: number;
  bobPeriod: number;
}

export function poseFor(state: MascotState): Pose {
  switch (state) {
    case "thinking":
      return {
        eyeOpen: 1,
        headPitch: -0.06,
        headRoll: 0.08,
        ahoge: 0.1,
        cubes: "orbit",
        bob: 0.012,
        bobPeriod: 3,
      };
    case "sleeping":
      return {
        eyeOpen: 0.1,
        headPitch: 0.14,
        headRoll: -0.05,
        ahoge: -0.18,
        cubes: "rest",
        bob: 0.008,
        bobPeriod: 4.5,
      };
    default:
      return {
        eyeOpen: 1,
        headPitch: 0,
        headRoll: 0,
        ahoge: 0,
        cubes: "float",
        bob: 0.012,
        bobPeriod: 3,
      };
  }
}

/** Arm angle (radians, outward) through a wave that lasts `progress` 0..1. */
export function waveAngle(progress: number): number {
  if (progress <= 0 || progress >= 1) return 0;
  const raised = 2.5;
  if (progress < 0.2) return raised * easeOut(progress / 0.2);
  if (progress > 0.8) return raised * (1 - easeOut((progress - 0.8) / 0.2));
  return raised + 0.35 * Math.sin(((progress - 0.2) / 0.6) * Math.PI * 6);
}

/** Height of a small hop over `progress` 0..1. */
export function hopHeight(progress: number): number {
  return progress <= 0 || progress >= 1 ? 0 : 0.08 * Math.sin(Math.PI * progress);
}

/** How far the cubes burst outward over `progress` 0..1, then settle back. */
export function burst(progress: number): number {
  if (progress <= 0 || progress >= 1) return 0;
  return Math.sin(Math.PI * progress) * (1 - progress * 0.5);
}

function easeOut(x: number): number {
  return 1 - (1 - x) ** 3;
}
