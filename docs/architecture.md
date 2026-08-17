# LimitLens Architecture

LimitLens is a Tauri 2 desktop utility with a small Rust core and a framework-free TypeScript WebView. The frontend only renders normalized provider state and invokes narrow backend commands. It has no filesystem, shell, network, or credential API exposed to JavaScript.

## State boundaries

- `settings.json` stores versioned user preferences: theme, enabled providers, order, selected windows, widget position, and behavior settings.
- `usage-cache.json` stores only last successful normalized provider usage and timestamps. Both files are atomically replaced to reduce corruption risk.
- `AppState` owns settings, cache, runtime provider state, the provider registry, and one refresh-overlap guard.
- `ProviderRegistry` contains only Codex, Claude Code, and Kimi Code adapters. Adding a provider means implementing `ProviderAdapter`, registering it, and adding adapter tests.

## Refresh model

One scheduler starts at launch, performs an asynchronous refresh, then sleeps for the selected 1, 2, 5, or 10 minute interval. It does not create per-provider timers. A single atomic guard rejects overlapping manual, scheduled, or future resume refreshes. Enabled providers are queried concurrently with bounded local work.

Reset timers are rendered from trusted `resets_at` values. The widget updates only at minute-level presentation time; reaching a reset does not change usage to zero. A later provider refresh must confirm any new value.

## Provider data policy

The adapter contract distinguishes detection from usage retrieval. Detection only checks a known executable name on `PATH`; it does not launch a CLI or inspect user configuration. Adapters may return normalized `UsageWindow` records only from documented safe interfaces. Any other situation becomes a controlled unavailable/unsupported state. No production adapter returns fixture data.
