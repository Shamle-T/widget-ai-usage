# Security Review

Review date: 2026-08-26. This review covers the current native taskbar and notification-area foundation, not a claim of final release approval.

## Reviewed controls

- **Provider execution:** Codex is invoked only as the fixed direct command `codex app-server`, with no shell command string, interpolation, user-controlled arguments, or arbitrary command IPC. On Windows, the adapter resolves either `codex.exe` on `PATH` or the native `codex.exe` in the official npm package behind `codex.cmd`; it never executes the batch shim or `cmd.exe`. The adapter sends only the documented initialization messages and `account/rateLimits/read`, limits the response to 16 lines of 64 KiB each, waits at most eight seconds, and terminates the child process after the request. Claude and Kimi use only a `PATH` lookup.
- **Credentials:** LimitLens does not read provider configuration, browser cookies, environment variables, prompts, response logs, source files, or shell history. It stores no secrets.
- **IPC and rendering:** LimitLens creates no WebView and exposes no frontend IPC commands. Numeric rings are generated from normalized state in Rust and passed directly to native APIs. On Windows, the taskbar child accepts only fixed click, drag, and menu actions; it cannot execute arbitrary commands.
- **Taskbar containment:** the Windows composition child is parented to the primary `Shell_TrayWnd`. Although its composition bounds match the taskbar, a rounded `SetWindowRgn` limits drawing and hit testing to the exact provider strip. Dragging is clamped to taskbar client bounds. This was reviewed specifically to avoid blocking Start, pinned applications, empty taskbar space, or the system tray.
- **Data at rest:** settings and the minimal normalized usage cache are separate versioned JSON files in the application data directory. Writes use uniquely created temporary files and same-volume atomic replacement; Windows uses `MoveFileExW` with replacement and write-through flags. Cache payloads exclude credentials, prompts, responses, source paths, and raw provider stderr.
- **Tauri privileges:** the empty WebView capability targets no windows. No shell, HTTP, or filesystem capability is exposed.
- **External links and updater:** no external URL opening, auto-updater, analytics, telemetry, crash upload, or remote LimitLens service exists in this revision.

## Remaining release work

- `npm audit` completed on 2026-08-26 with zero reported vulnerabilities.
- `cargo audit` completed on 2026-08-26. It reported 17 allowed advisory warnings in Tauri's cross-platform transitive graph: unmaintained GTK3 bindings, unmaintained `unic-*` crates, `proc-macro-error`, and `RUSTSEC-2024-0429` for `glib`. No warning was suppressed. Track Tauri/Wry upgrades and re-run the audit before release; do not ship Linux artifacts until the maintainers assess the current advisory status.
- Run platform-native security checks for every release candidate.
- Reassess CSP policy against the final bundled asset paths.
- Exercise startup registration and tray behavior on Windows, macOS, and Linux. Test the embedded surface on supported Windows 10/11 taskbars and common display scaling values.
- Review future provider adapters for process lifetime, output-size limits, timeouts, cancellation, path handling, and sanitized failures before enabling them.
