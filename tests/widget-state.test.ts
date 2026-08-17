import { describe, expect, it } from "vitest";
import {
  activeWindow,
  cycleWindow,
  displayedPercent,
  visibleProviders,
} from "../src/state/widget-state";
import type { ProviderView } from "../src/types";

const provider: ProviderView = {
  id: "codex",
  displayName: "Codex",
  enabled: true,
  detected: true,
  status: "live",
  statusMessage: null,
  source: "Fixture",
  selectedWindowId: "five-hour",
  updatedAt: null,
  lastSuccessfulAt: null,
  windows: [
    { id: "five-hour", displayName: "5 hour", kind: "short_term", usedPercent: 24, resetsAt: null },
    { id: "weekly", displayName: "Weekly", kind: "weekly", usedPercent: 61, resetsAt: null },
  ],
};

describe("widget state", () => {
  it("keeps an individual provider selection when weekly view is inactive", () => {
    expect(activeWindow(provider, false)?.id).toBe("five-hour");
    expect(displayedPercent(provider, false)).toBe(24);
  });

  it("uses the weekly window without mutating individual selection", () => {
    expect(activeWindow(provider, true)?.id).toBe("weekly");
    expect(provider.selectedWindowId).toBe("five-hour");
  });

  it("cycles only among available windows", () => expect(cycleWindow(provider)).toBe("weekly"));

  it("returns unavailable when global weekly data is absent", () => {
    const withoutWeekly = { ...provider, windows: [provider.windows[0]] };
    expect(displayedPercent(withoutWeekly, true)).toBeNull();
  });

  it("does not cycle a provider that exposes one window", () => {
    expect(cycleWindow({ ...provider, windows: [provider.windows[0]] })).toBe("five-hour");
  });

  it("hides disabled providers without leaving an empty slot", () => {
    const state = {
      version: 1,
      theme: "dark" as const,
      weeklyOverride: false,
      refreshIntervalMinutes: 5 as const,
      alwaysOnTop: false,
      lockPosition: false,
      startWithSystem: false,
      refreshing: false,
      providers: [provider, { ...provider, id: "kimi", enabled: false }],
    };
    expect(visibleProviders(state)).toHaveLength(1);
  });

  it("hides enabled providers until their client is detected", () => {
    const state = {
      version: 1,
      theme: "dark" as const,
      weeklyOverride: false,
      refreshIntervalMinutes: 5 as const,
      alwaysOnTop: false,
      lockPosition: false,
      startWithSystem: false,
      refreshing: false,
      providers: [provider, { ...provider, id: "kimi", detected: false }],
    };
    expect(visibleProviders(state).map((item) => item.id)).toEqual(["codex"]);
  });
});
