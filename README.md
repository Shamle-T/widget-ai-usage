# LimitLens

LimitLens is a compact local-first taskbar utility for glancing at AI coding-service usage limits without opening multiple dashboards.

> Screenshot placeholder: add a verified production screenshot before the first public release.

## Current status

LimitLens runs without an application or settings window. On Windows, enabled and detected providers appear in a native widget embedded in the main horizontal taskbar. The notification-area icon remains available as a fallback and settings entry point. On macOS and Linux, provider rings use the notification area. Codex usage is collected through its documented local app-server rate-limit protocol. Claude Code and Kimi Code remain unavailable because no equivalent safe machine-readable query has been verified.

| Provider     | Detection               | Short-term  | Weekly                  | Reset                   | Platforms             |
| ------------ | ----------------------- | ----------- | ----------------------- | ----------------------- | --------------------- |
| OpenAI Codex | `PATH` executable check | Supported   | Supported when supplied | Supported when supplied | Windows, macOS, Linux |
| Claude Code  | `PATH` executable check | Unavailable | Unavailable             | Unavailable             | Windows, macOS, Linux |
| Kimi Code    | `PATH` executable check | Unavailable | Unavailable             | Unavailable             | Windows, macOS, Linux |

See [provider research](docs/provider-research.md) for the evidence and policy behind this matrix.

## Product scope

- A compact Windows taskbar widget that grows horizontally with enabled, detected providers.
- Native notification-area rings remain available for settings and recovery.
- Numeric progress rings generated in Rust with no WebView or rendering loop.
- Independent per-provider window selection: click one tile to cycle its real windows; drag a tile horizontally to reposition the Windows widget.
- Native right-click Settings menu for providers, refresh interval, ring theme, autostart, About, and Exit.
- Persisted provider settings, selected windows, provider order, and compact last-known usage cache.
- Centralized 1/2/5/10-minute refresh scheduling, manual refresh, stale-state modeling, single-instance behavior, and system startup setting.
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

Use `npm.cmd run dev` to start the development application on Windows PowerShell systems that restrict `npm.ps1` execution. LimitLens has no main window. On Windows, interact with the embedded taskbar widget or its notification-area icon.

Windows may initially place the notification icon in the overflow panel. This does not affect the embedded widget. Windows owns and persists notification-area placement.

## Provider requirements

LimitLens only detects clients installed on `PATH`. It invokes Codex's documented read-only app-server method only to read its rate limits, and does not run any model turn or copy credentials. Undetected providers are listed in Settings but do not receive taskbar rings.

When a provider documents a safe quota protocol, LimitLens will implement it behind a focused adapter with fixtures and contract tests. It will not use browser scraping, terminal emulation, local conversation-log parsing, or generated model requests as substitutes.

## Privacy and security

LimitLens is local-first. Configuration and an optional last-known usage cache stay in the platform application-data directory. Current adapters store no credentials. Codex uses its already authenticated official client to request quota metadata only. Review [security.md](docs/security.md) before contributing an integration.

## Resource usage

The widget is designed for negligible idle work: one scheduler, no rendering loop, no filesystem polling, and no persistent provider processes. Codex is a short-lived child process only during a refresh. See [performance.md](docs/performance.md) for the current measurement and protocol.

## Troubleshooting

- **Provider says Not detected:** install the official client, make its executable visible on `PATH`, enable it under **Settings > Providers**, and choose **Refresh Now**.
- **Provider says Unsupported:** the client is present, but LimitLens has no safe documented usage query for it. This is intentional, not a request to sign in again.
- **Taskbar widget is absent:** right-click the fallback notification icon, enable a detected provider under **Providers**, then choose **Refresh Now**. Providers that cannot be identified are intentionally not rendered.
- **Move the Windows widget:** drag the visible provider tile horizontally. LimitLens clamps and persists the position within the main taskbar.
- **Notification icon is in the overflow panel:** drag it into the visible notification area if desired. Applications cannot force this Windows preference.
- **Custom or vertical taskbar:** V1 embeds into the primary horizontal `Shell_TrayWnd`. Secondary, vertical, and heavily modified taskbars are not yet claimed as supported and retain the notification-area fallback.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md), and [LATER_FEATURES.mf](LATER_FEATURES.mf). A software license has not been chosen, so this repository is not yet represented as an open-source release.
