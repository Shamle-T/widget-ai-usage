import type { AppViewState, ProviderView, UsageWindow } from "../types";

export function activeWindow(provider: ProviderView, weeklyOverride: boolean): UsageWindow | null {
  if (weeklyOverride) {
    return provider.windows.find((window) => window.kind === "weekly") ?? null;
  }

  return provider.windows.find((window) => window.id === provider.selectedWindowId) ?? null;
}

export function displayedPercent(provider: ProviderView, weeklyOverride: boolean): number | null {
  return activeWindow(provider, weeklyOverride)?.usedPercent ?? null;
}

export function visibleProviders(state: AppViewState): ProviderView[] {
  return state.providers.filter((provider) => provider.enabled);
}

export function cycleWindow(provider: ProviderView): string | null {
  if (provider.windows.length < 2) return provider.selectedWindowId;
  const selectedIndex = provider.windows.findIndex(
    (window) => window.id === provider.selectedWindowId,
  );
  return provider.windows[(selectedIndex + 1 + provider.windows.length) % provider.windows.length]
    .id;
}
