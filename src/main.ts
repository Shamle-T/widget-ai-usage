import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AppViewState, ProviderView, Theme } from "./types";
import { activeWindow, displayedPercent, visibleProviders } from "./state/widget-state";
import { countdown, displayTimestamp } from "./utils/time";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/600.css";
import "@fontsource/jetbrains-mono/700.css";
import "./styles.css";

const app = document.querySelector<HTMLElement>("#app");
if (!app) throw new Error("LimitLens root element is missing.");
const root = app;
const settingsMode = new URLSearchParams(window.location.search).has("settings");
const aboutMode = new URLSearchParams(window.location.search).has("about");

let state: AppViewState | null = null;
let contextMenuOpen = false;
let lastVisibilityRefresh = Date.now();

function escapeHtml(value: string): string {
  const template = document.createElement("template");
  template.textContent = value;
  return template.innerHTML;
}

function statusLabel(status: ProviderView["status"]): string {
  return status.replaceAll("_", " ").replace(/\b\w/g, (character) => character.toUpperCase());
}

function renderRing(provider: ProviderView, appState: AppViewState): string {
  const window = activeWindow(provider, appState.weeklyOverride);
  const percent = displayedPercent(provider, appState.weeklyOverride);
  const offset =
    percent === null ? 263.9 : 263.9 - (Math.max(0, Math.min(100, percent)) / 100) * 263.9;
  const percentLabel = percent === null ? "—" : `${Math.round(percent)}%`;
  const reset = countdown(window?.resetsAt ?? null);
  const windowLabel = window?.displayName ?? (appState.weeklyOverride ? "Weekly" : "Unavailable");
  const tooltip = [
    provider.displayName.toUpperCase(),
    windowLabel,
    percent === null ? "Usage: Unavailable" : `Used: ${Math.round(percent)}%`,
    percent === null ? "" : `Remaining: ${Math.max(0, 100 - Math.round(percent))}%`,
    reset ? `Resets in: ${reset}` : "",
    `Last checked: ${displayTimestamp(provider.updatedAt)}`,
    `Status: ${statusLabel(provider.status)}`,
    `Source: ${provider.source}`,
    provider.statusMessage ?? "",
  ]
    .filter(Boolean)
    .join("<br />");

  return `
    <button class="provider-card" data-provider-id="${escapeHtml(provider.id)}" type="button"
      aria-label="${escapeHtml(provider.displayName)}: ${escapeHtml(windowLabel)}, ${percentLabel} consumed. Click to change window."
      data-tooltip="${escapeHtml(tooltip)}">
      <span class="ring" aria-hidden="true">
        <svg viewBox="0 0 96 96" focusable="false">
          <circle class="ring-track" cx="48" cy="48" r="42" />
          <circle class="ring-value ${percent === null ? "ring-value--unavailable" : ""}" cx="48" cy="48" r="42"
            stroke-dasharray="263.9" stroke-dashoffset="${offset}" />
        </svg>
        <span class="ring-percent">${percentLabel}</span>
      </span>
      <span class="provider-name">${escapeHtml(provider.displayName)}</span>
      <span class="provider-window">${escapeHtml(windowLabel)}</span>
      <span class="provider-reset">${escapeHtml(reset ? `resets ${reset}` : provider.status === "not_installed" ? "not detected" : "Unavailable")}</span>
    </button>`;
}

function renderContextMenu(appState: AppViewState): string {
  const providerItems = appState.providers
    .map(
      (provider) => `
        <button type="button" data-action="toggle-provider" data-provider-id="${provider.id}" role="menuitemcheckbox" aria-checked="${provider.enabled}">
          <span>${provider.enabled ? "✓" : ""}</span>${escapeHtml(provider.displayName)}
        </button>`,
    )
    .join("");
  const intervals = ([1, 2, 5, 10] as const)
    .map(
      (minutes) => `
        <button type="button" data-action="set-interval" data-minutes="${minutes}" role="menuitemradio" aria-checked="${appState.refreshIntervalMinutes === minutes}">
          <span>${appState.refreshIntervalMinutes === minutes ? "✓" : ""}</span>${minutes} minute${minutes === 1 ? "" : "s"}
        </button>`,
    )
    .join("");
  return `
    <div class="context-menu" role="menu" aria-label="LimitLens menu">
      <button type="button" data-action="refresh" role="menuitem">Refresh Now</button>
      <div class="menu-group"><span>Providers</span>${providerItems}<button type="button" data-action="settings" role="menuitem">Manage Providers...</button></div>
      <div class="menu-group"><span>Refresh Interval</span>${intervals}</div>
      <div class="menu-group"><span>Theme</span>
        <button type="button" data-action="theme" data-theme="dark" role="menuitemradio" aria-checked="${appState.theme === "dark"}"><span>${appState.theme === "dark" ? "✓" : ""}</span>Dark</button>
        <button type="button" data-action="theme" data-theme="light" role="menuitemradio" aria-checked="${appState.theme === "light"}"><span>${appState.theme === "light" ? "✓" : ""}</span>Light</button>
      </div>
      <button type="button" data-action="always-on-top" role="menuitemcheckbox" aria-checked="${appState.alwaysOnTop}"><span>${appState.alwaysOnTop ? "✓" : ""}</span>Always on Top</button>
      <button type="button" data-action="lock-position" role="menuitemcheckbox" aria-checked="${appState.lockPosition}"><span>${appState.lockPosition ? "✓" : ""}</span>Lock Position</button>
      <button type="button" data-action="autostart" role="menuitemcheckbox" aria-checked="${appState.startWithSystem}"><span>${appState.startWithSystem ? "✓" : ""}</span>Start with System</button>
      <div class="menu-divider"></div>
      <button type="button" data-action="settings" role="menuitem">Settings</button>
      <button type="button" data-action="about" role="menuitem">About</button>
      <div class="menu-divider"></div>
      <button type="button" data-action="hide" role="menuitem">Hide LimitLens</button>
      <button type="button" data-action="quit" role="menuitem">Quit</button>
    </div>`;
}

function render(next: AppViewState): void {
  state = next;
  document.documentElement.dataset.theme = next.theme;
  root.innerHTML = `
    <section class="widget" aria-label="LimitLens usage widget">
      <header class="widget-header" data-drag-region>
        <span class="wordmark">LIMIT<span>LENS</span></span>
        ${next.refreshing ? '<span class="refresh-state">refreshing</span>' : ""}
      </header>
      <div class="telemetry-row">${visibleProviders(next)
        .map((provider) => renderRing(provider, next))
        .join("")}
        <label class="week-toggle" title="Show weekly usage for every provider">
          <span>WEEK</span>
          <input type="checkbox" ${next.weeklyOverride ? "checked" : ""} aria-label="Show weekly usage for every provider" />
          <span class="toggle-track" aria-hidden="true"><span></span></span>
        </label>
      </div>
      ${contextMenuOpen ? renderContextMenu(next) : ""}
    </section>`;
}

function renderSettings(next: AppViewState, aboutOnly: boolean): void {
  document.documentElement.dataset.theme = next.theme;
  const providerRows = next.providers
    .map(
      (provider) => `
    <article class="settings-provider">
      <div><strong>${escapeHtml(provider.displayName)}</strong><span>${escapeHtml(statusLabel(provider.status))}</span></div>
      <p>${escapeHtml(provider.source)} · ${provider.windows.map((window) => window.displayName).join(", ") || "Quota data unavailable"}</p>
      <label class="settings-check"><input type="checkbox" data-setting-provider="${provider.id}" ${provider.enabled ? "checked" : ""} /> Enabled</label>
      <button type="button" class="secondary-button" data-action="test-connection" data-provider-id="${provider.id}">Test Connection</button>
    </article>`,
    )
    .join("");
  root.innerHTML = aboutOnly
    ? `
    <section class="settings-page"><header><span class="wordmark">LIMIT<span>LENS</span></span><h1>About</h1></header>
    <p>A local-first telemetry widget. It stores preferences and last-known usage locally, never sends prompts, source code, tokens, cookies, or telemetry.</p>
    <dl><dt>Version</dt><dd>0.1.0</dd><dt>Platform</dt><dd>${navigator.platform}</dd><dt>License</dt><dd>Not selected yet</dd></dl>
    <p class="settings-note">GitHub links and release signing are intentionally deferred until the public project setup is complete.</p></section>`
    : `
    <section class="settings-page"><header><span class="wordmark">LIMIT<span>LENS</span></span><h1>Settings</h1></header>
      <section><h2>Providers</h2>${providerRows}</section>
      <section><h2>Appearance</h2><div class="setting-options"><button data-action="theme" data-theme="dark" aria-pressed="${next.theme === "dark"}">Dark</button><button data-action="theme" data-theme="light" aria-pressed="${next.theme === "light"}">Light</button></div></section>
      <section><h2>Behaviour</h2><label>Refresh interval <select data-setting="interval">${[1, 2, 5, 10].map((minutes) => `<option value="${minutes}" ${next.refreshIntervalMinutes === minutes ? "selected" : ""}>${minutes} minute${minutes === 1 ? "" : "s"}</option>`).join("")}</select></label>
      <label class="settings-check"><input type="checkbox" data-setting="always-on-top" ${next.alwaysOnTop ? "checked" : ""} /> Always on top</label><label class="settings-check"><input type="checkbox" data-setting="lock-position" ${next.lockPosition ? "checked" : ""} /> Lock position</label><label class="settings-check"><input type="checkbox" data-setting="autostart" ${next.startWithSystem ? "checked" : ""} /> Start with system</label></section>
      <section><h2>About</h2><p>LimitLens 0.1.0 · Local-first · No telemetry · No provider credentials stored</p></section></section>`;
}

async function update(): Promise<void> {
  const next = await invoke<AppViewState>("get_view_state");
  if (settingsMode || aboutMode) renderSettings(next, aboutMode);
  else render(next);
}

async function handleAction(action: string, element: HTMLElement): Promise<void> {
  switch (action) {
    case "refresh":
      await invoke("refresh_now");
      break;
    case "toggle-provider":
      await invoke("set_provider_enabled", {
        providerId: element.dataset.providerId,
        enabled: element.getAttribute("aria-checked") !== "true",
      });
      break;
    case "set-interval":
      await invoke("set_refresh_interval", { minutes: Number(element.dataset.minutes) });
      break;
    case "theme":
      await invoke("set_theme", { theme: element.dataset.theme as Theme });
      break;
    case "always-on-top":
      await invoke("set_always_on_top", {
        enabled: element.getAttribute("aria-checked") !== "true",
      });
      break;
    case "lock-position":
      await invoke("set_lock_position", {
        enabled: element.getAttribute("aria-checked") !== "true",
      });
      break;
    case "autostart":
      await invoke("set_start_with_system", {
        enabled: element.getAttribute("aria-checked") !== "true",
      });
      break;
    case "test-connection":
      await invoke("test_connection", { providerId: element.dataset.providerId }).catch(
        () => undefined,
      );
      break;
    case "settings":
      await invoke("open_settings");
      break;
    case "about":
      await invoke("open_about");
      break;
    case "hide":
      await invoke("hide_widget");
      break;
    case "quit":
      await invoke("quit_application");
      break;
    default:
      return;
  }
  contextMenuOpen = false;
  await update();
}

root.addEventListener("click", async (event) => {
  const target = event.target as HTMLElement;
  const action = target.closest<HTMLElement>("[data-action]");
  if (action?.dataset.action) {
    await handleAction(action.dataset.action, action);
    return;
  }
  const toggle = target.closest<HTMLInputElement>(".week-toggle input");
  if (toggle) {
    await invoke("set_weekly_override", { enabled: toggle.checked });
    await update();
    return;
  }
  const card = target.closest<HTMLElement>(".provider-card");
  if (card && state && !state.weeklyOverride) {
    await invoke("cycle_provider_window", { providerId: card.dataset.providerId });
    await update();
  }
});

root.addEventListener("change", async (event) => {
  const target = event.target as HTMLInputElement | HTMLSelectElement;
  if (target instanceof HTMLInputElement && target.dataset.settingProvider) {
    await invoke("set_provider_enabled", {
      providerId: target.dataset.settingProvider,
      enabled: target.checked,
    });
    await update();
  }
  if (target instanceof HTMLSelectElement && target.dataset.setting === "interval") {
    await invoke("set_refresh_interval", { minutes: Number(target.value) });
    await update();
  }
  if (target instanceof HTMLInputElement && target.dataset.setting === "always-on-top") {
    await invoke("set_always_on_top", { enabled: target.checked });
    await update();
  }
  if (target instanceof HTMLInputElement && target.dataset.setting === "lock-position") {
    await invoke("set_lock_position", { enabled: target.checked });
    await update();
  }
  if (target instanceof HTMLInputElement && target.dataset.setting === "autostart") {
    await invoke("set_start_with_system", { enabled: target.checked });
    await update();
  }
});

root.addEventListener("contextmenu", (event) => {
  event.preventDefault();
  contextMenuOpen = true;
  if (state) render(state);
});

root.addEventListener("pointerdown", async (event) => {
  const target = event.target as HTMLElement;
  if (event.button === 0 && target.closest("[data-drag-region]") && !state?.lockPosition)
    await getCurrentWindow().startDragging();
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && contextMenuOpen && state) {
    contextMenuOpen = false;
    render(state);
  }
});

async function bootstrap(): Promise<void> {
  await listen<AppViewState>("usage-state-updated", (event) =>
    settingsMode || aboutMode ? renderSettings(event.payload, aboutMode) : render(event.payload),
  );
  const initial = await invoke<AppViewState>("get_view_state");
  if (settingsMode || aboutMode) renderSettings(initial, aboutMode);
  else render(initial);
}

void bootstrap();

window.setInterval(() => {
  if (state && !settingsMode && !aboutMode) render(state);
}, 60_000);

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState !== "visible" || Date.now() - lastVisibilityRefresh < 60_000) return;
  lastVisibilityRefresh = Date.now();
  window.setTimeout(() => {
    void invoke("refresh_now");
  }, 4_000);
});
