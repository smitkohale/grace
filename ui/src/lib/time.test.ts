import { describe, expect, it } from "vitest";
import { relativeTime } from "./time";

const now = Date.parse("2026-11-27T12:00:00.000Z");
const ago = (minutes: number) => new Date(now - minutes * 60_000).toISOString();

describe("relativeTime", () => {
  it("uses plain words", () => {
    expect(relativeTime(ago(0.2), now)).toBe("just now");
    expect(relativeTime(ago(1), now)).toBe("1 minute ago");
    expect(relativeTime(ago(5), now)).toBe("5 minutes ago");
    expect(relativeTime(ago(90), now)).toBe("1 hour ago");
    expect(relativeTime(ago(180), now)).toBe("3 hours ago");
    expect(relativeTime(ago(30 * 60), now)).toBe("yesterday");
    expect(relativeTime(ago(4 * 24 * 60), now)).toBe("4 days ago");
    expect(relativeTime(ago(75 * 24 * 60), now)).toBe("2 months ago");
  });

  it("is empty for missing or broken timestamps", () => {
    expect(relativeTime(null, now)).toBe("");
    expect(relativeTime("not a date", now)).toBe("");
  });
});
