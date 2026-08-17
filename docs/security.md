# Security Review

Review date: 2026-08-17. This review covers the current foundation, not a claim of final release approval.

## Reviewed controls

- **Provider execution:** Codex is invoked only as the fixed direct command `codex app-server` (the platform npm shim is `codex.cmd` on Windows), with no shell command string, interpolation, user-controlled arguments, or arbitrary command IPC. The adapter sends only the documented initialization messages and `account/rateLimits/read`, limits the response to 16 lines of 64 KiB each, waits at most eight seconds, and terminates the child process after the request. Claude and Kimi use only a `PATH` lookup.
- **Credentials:** LimitLens does not read provider configuration, browser cookies, environment variables, prompts, response logs, source files, or shell history. It stores no secrets.
- **IPC:** the WebView has narrow typed commands for settings, refresh, view state, hiding/quitting, and settings windows. There is no frontend filesystem, shell, HTTP, or arbitrary window-command bridge.
- **Data at rest:** settings and the minimal normalized usage cache are separate versioned JSON files in the application data directory. Writes use temporary files and replacement. Cache payloads exclude credentials, prompts, responses, source paths, and raw provider stderr.
- **Rendering:** provider strings are escaped before HTML rendering. Percentages are normalized in Rust before reaching UI state.
- **Tauri privileges:** the default capability grants only required core window actions and autostart plugin actions. No broad shell or filesystem capability is granted.
- **External links and updater:** no external URL opening, auto-updater, analytics, telemetry, crash upload, or remote LimitLens service exists in this revision.

## Remaining release work

- `npm audit` completed on 2026-08-17 with zero reported vulnerabilities.
- `cargo audit` completed on 2026-08-17. It reported 17 non-blocking advisory warnings in Tauri's Linux GTK/WebKit transitive graph: unmaintained GTK3 bindings, unmaintained `unic-*` crates, `proc-macro-error`, and `RUSTSEC-2024-0429` for `glib`. No warning was suppressed. Track Tauri/Wry upgrades and re-run the audit before release; do not ship Linux artifacts until the maintainers assess the current advisory status.
- Run platform-native security checks for every release candidate.
- Reassess CSP policy against the final bundled asset paths.
- Exercise startup registration and tray behavior on Windows, macOS, and Linux.
- Review future provider adapters for process lifetime, output-size limits, timeouts, cancellation, path handling, and sanitized failures before enabling them.
