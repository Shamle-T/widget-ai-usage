export type Theme = "dark" | "light";
export type UsageWindowKind = "short_term" | "daily" | "weekly" | "monthly" | "provider_specific";
export type ProviderStatus =
  | "loading"
  | "live"
  | "cached"
  | "stale"
  | "unavailable"
  | "authentication_required"
  | "not_installed"
  | "unsupported"
  | "error";

export interface UsageWindow {
  id: string;
  displayName: string;
  kind: UsageWindowKind;
  usedPercent: number | null;
  resetsAt: string | null;
}

export interface ProviderView {
  id: string;
  displayName: string;
  enabled: boolean;
  detected: boolean;
  status: ProviderStatus;
  statusMessage: string | null;
  source: string;
  selectedWindowId: string | null;
  windows: UsageWindow[];
  updatedAt: string | null;
  lastSuccessfulAt: string | null;
}

export interface AppViewState {
  version: number;
  theme: Theme;
  weeklyOverride: boolean;
  refreshIntervalMinutes: 1 | 2 | 5 | 10;
  alwaysOnTop: boolean;
  lockPosition: boolean;
  startWithSystem: boolean;
  refreshing: boolean;
  providers: ProviderView[];
}
