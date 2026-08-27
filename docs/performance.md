# Performance Measurements

## Windows tray-only release measurement

| Metric                  | Result                                                        |
| ----------------------- | ------------------------------------------------------------- |
| Measurement date        | 2026-08-26                                                    |
| LimitLens version       | `0.1.0`                                                       |
| Operating system        | Microsoft Windows build `10.0.26200`, x86_64                  |
| Build type              | Windows x86_64 release, unbundled                             |
| Enabled providers       | None; persisted settings disabled all three providers         |
| Refresh interval        | 5 minutes                                                     |
| Idle observation        | 5 minutes after a 3-second launch settling period             |
| Idle CPU                | `0.000` additional CPU seconds over the observation           |
| Working set             | `21.7 MiB` (`22,708,224` bytes) at the end of observation     |
| Private memory          | `3.3 MiB` (`3,469,312` bytes) at the end of observation       |
| Release executable size | `8.9 MiB` (`9,344,000` bytes)                                 |
| Background processes    | One LimitLens process; no child process and no WebView process |
| Application windows     | No visible or main application window                         |

The host denied access to hardware details through the available system API, so this is not a complete reproducible hardware benchmark. The sample measures the no-provider idle path and does not measure a Codex refresh. Windows notification-area placement and first-icon latency were visually outside this command-line measurement.

## Intended measurement protocol

Measure a signed-off release candidate on Windows, macOS, and Linux with the following recorded for each result:

- LimitLens version and build profile.
- Operating system, hardware, display scaling, and enabled providers.
- Selected refresh interval.
- Cold startup time to first notification icon.
- Idle CPU and resident memory over five minutes.
- CPU and peak memory during one refresh.
- Number and lifetime of child processes.
- Packaged artifact size.

The current implementation has one low-frequency scheduler, no WebView, no rendering loop, no file polling, and no persistent child provider process. Codex starts a short-lived process during a refresh to issue a quota-only request through its official app-server. Those design properties are not substitutes for measurements with enabled providers or validation on macOS and Linux.
