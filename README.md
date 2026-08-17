# LimitLens

LimitLens is a compact local-first desktop widget for glancing at AI coding-service usage limits without opening multiple dashboards.

> Screenshot placeholder: add a verified production screenshot before the first public release.

## Current status

The Tauri widget foundation is implemented for Windows, macOS, and Linux architecture. Codex, Claude Code, and Kimi Code are registered providers with safe executable detection. Their live quota collection is intentionally **unavailable** in this revision because current official interfaces do not offer a stable, standalone, non-generative machine-readable usage query. LimitLens does not scrape terminal UIs, browser sessions, or credentials to fill that gap. Linux release artifacts remain subject to the documented GTK/WebKit Cargo Audit advisories in [security.md](docs/security.md).

| Provider     | Detection               | Short-term  | Weekly      | Reset       | Platforms             |
| ------------ | ----------------------- | ----------- | ----------- | ----------- | --------------------- |
| OpenAI Codex | `PATH` executable check | Unavailable | Unavailable | Unavailable | Windows, macOS, Linux |
| Claude Code  | `PATH` executable check | Unavailable | Unavailable | Unavailable | Windows, macOS, Linux |
| Kimi Code    | `PATH` executable check | Unavailable | Unavailable | Unavailable | Windows, macOS, Linux |

See [provider research](docs/provider-research.md) for the evidence and policy behind this matrix.

## Product scope

- Frameless horizontal telemetry widget with SVG usage rings.
- Per-provider window selection and a global WEEK override, ready for real normalized provider windows.
- Dark and light warm-neutral themes with locally bundled JetBrains Mono.
- Persisted settings, selected windows, provider order, widget position, and compact last-known usage cache.
- Centralized 1/2/5/10-minute refresh scheduling, manual refresh, stale-state modeling, best-effort resume/visibility refresh, close-to-tray behavior, single-instance activation, always-on-top, position lock, and system startup setting.
- No dashboard, notifications, history graphs, telemetry, analytics, LimitLens account, cloud backend, prompt capture, response capture, source-code capture, cookie scraping, or stored provider credentials.

## Installation

No public installers are published yet. Do not treat this repository as a signed release. Release signing, updater infrastructure, and software licensing are deliberately deferred.

## Build from source

Prerequisites:

- Node.js 22 or newer.
- Rust toolchain 1.88.0 or newer.
- Platform prerequisites required by [Tauri 2](https://v2.tauri.app/start/prerequisites/).

```powershell
npm.cmd install
npm.cmd run check
npm.cmd test
npm.cmd run tauri:build
```

Use `npm.cmd run dev` to start the development application on Windows PowerShell systems that restrict `npm.ps1` execution.

## Provider requirements

LimitLens only detects clients installed on `PATH`. It neither runs provider agents nor copies tokens from provider clients. A detected provider with unavailable usage remains visible and reports why in its tooltip and settings state.

When a provider documents a safe quota protocol, LimitLens will implement it behind a focused adapter with fixtures and contract tests. It will not use browser scraping, terminal emulation, local conversation-log parsing, or generated model requests as substitutes.

## Privacy and security

LimitLens is local-first. Configuration and an optional last-known usage cache stay in the platform application-data directory. Current adapters store no credentials and make no provider network request. Review [security.md](docs/security.md) before contributing an integration.

## Resource usage

The widget is designed for negligible idle work: one scheduler, no rendering loop, no filesystem polling, and no persistent provider processes. Measured release metrics have not yet been recorded because this development workspace lacks Rust/Cargo; see [performance.md](docs/performance.md).

## Troubleshooting

- **Provider says Not installed:** install the official client and make its executable visible on `PATH`, then use Settings → Test Connection.
- **Provider says Unsupported:** the client is present, but LimitLens has no safe documented usage query for it. This is intentional, not a request to sign in again.
- **Widget is hidden:** use the system-tray menu, then choose **Show LimitLens**.
- **Position is wrong after display changes:** show the widget from the tray. The app validates persisted coordinates and falls back to Tauri’s centered placement for obviously invalid values.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md), and [LATER_FEATURES.mf](LATER_FEATURES.mf). A software license has not been chosen, so this repository is not yet represented as an open-source release.
