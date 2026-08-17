import { describe, expect, it } from "vitest";
import { countdown, displayTimestamp } from "../src/utils/time";

describe("reset countdown", () => {
  const now = new Date("2026-08-17T10:00:00.000Z");

  it("formats hours and minutes without a second-by-second display", () => {
    expect(countdown("2026-08-17T13:39:01.000Z", now)).toBe("3h 40m");
  });

  it("formats days for long windows", () => {
    expect(countdown("2026-08-19T12:00:00.000Z", now)).toBe("2d 2h");
  });

  it("requests a fresh state once a reset has elapsed", () => {
    expect(countdown("2026-08-17T10:00:00.000Z", now)).toBe("Refreshing");
  });

  it("does not invent a countdown from a missing or invalid reset", () => {
    expect(countdown(null, now)).toBeNull();
    expect(countdown("not-a-date", now)).toBeNull();
  });

  it("uses a compact local display for valid update timestamps", () => {
    expect(displayTimestamp("2026-08-17T10:05:00.000Z")).not.toBe("Unknown");
    expect(displayTimestamp(null)).toBe("Not checked");
  });
});
