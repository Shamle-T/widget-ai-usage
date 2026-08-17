# Performance Measurements

## Initial Windows release measurement

| Metric                  | Result                                                          |
| ----------------------- | --------------------------------------------------------------- |
| LimitLens version       | `0.1.0`                                                         |
| Build type              | Windows x86_64 release, unbundled                               |
| Enabled providers       | Codex, Claude, Kimi (Codex app-server runs only during refresh) |
| Refresh interval        | 5 minutes                                                       |
| Idle observation        | 5 minutes after a 10-second launch settling period              |
| Idle CPU                | `0.000` CPU seconds over 5 minutes                              |
| Working set             | `37.9 MB` at the end of observation                             |
| Release executable size | `10.3 MB` (`10,292,736` bytes)                                  |

The host denied access to its detailed Windows version through the available system API, so this result should be treated as a Windows-host measurement rather than a reproducible OS-build benchmark. It does not yet measure a real provider refresh. Cold time-to-first-render was also not measured precisely; the process was observed after a deliberate 10-second settling period.

## Intended measurement protocol

Measure a signed-off release candidate on Windows, macOS, and Linux with the following recorded for each result:

- LimitLens version and build profile.
- Operating system, hardware, display scaling, and enabled providers.
- Selected refresh interval.
- Cold startup time to first rendered widget.
- Idle CPU and resident memory over five minutes.
- CPU and peak memory during one refresh.
- Number and lifetime of child processes.
- Packaged artifact size.

The current implementation has one low-frequency scheduler, no rendering loop, no file polling, and no persistent child provider process. Codex starts a short-lived process during a refresh to issue a quota-only request through its official app-server. Those design properties are not substitutes for the additional measurements above.
