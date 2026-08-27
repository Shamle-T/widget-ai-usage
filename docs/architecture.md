# LimitLens Architecture

LimitLens is a windowless Tauri 2 utility with a small Rust core and no application WebView. On Windows it adds a native GDI child surface to the primary horizontal taskbar and retains native notification-area icons as settings/recovery controls. Other platforms use the notification area.

## State boundaries

- `settings.json` stores versioned user preferences: ring theme, enabled providers, order, selected windows, refresh interval, autostart, and the Windows taskbar offset.
- `usage-cache.json` stores only last successful normalized provider usage and timestamps. Both files are atomically replaced to reduce corruption risk.
- `AppState` owns settings, cache, runtime provider state, the provider registry, and one refresh-overlap guard.
- `ProviderRegistry` contains only Codex, Claude Code, and Kimi Code adapters. Adding a provider means implementing `ProviderAdapter`, registering it, and adding adapter tests.
- The tray registry uses stable `provider-<id>` identifiers. Refreshes replace icon pixels, tooltip text, and native menus without recreating unchanged icons.
- The Windows taskbar surface is created as `WS_CHILD` of `Shell_TrayWnd`. Windows 11 requires a taskbar-sized layered composition surface; `SetWindowRgn` restricts its rendered and interactive area to the exact rounded provider strip. The region grows by 108 physical pixels per visible provider and prevents the remaining taskbar area from intercepting input.
- A low-frequency recovery check reattaches or recreates the child after Explorer/taskbar replacement. It does not repaint when state and geometry are unchanged.

## Refresh model

One scheduler starts at launch, performs an asynchronous refresh, then sleeps for the selected 1, 2, 5, or 10 minute interval. It does not create per-provider timers. A single atomic guard rejects overlapping manual, scheduled, or future resume refreshes. Enabled providers are queried concurrently with bounded local work.

Reset times in native tooltips are derived from trusted `resets_at` values. Reaching a reset does not change usage to zero; a provider refresh must confirm a new value.

Left-clicking a provider icon cycles only that provider's real usage windows. There is no global weekly override. Right-clicking any ring opens the same native settings menu.

## Provider data policy

The adapter contract distinguishes detection from usage retrieval. Detection only checks a known executable name on `PATH`; it does not launch a CLI or inspect user configuration. Adapters may return normalized `UsageWindow` records only from documented safe interfaces. Any other situation becomes a controlled unavailable/unsupported state. No production adapter returns fixture data.
