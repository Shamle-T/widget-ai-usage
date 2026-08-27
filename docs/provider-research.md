# Provider Research

Research date: 2026-08-26. This document records the official interface evidence used by the current adapters. It is intentionally conservative: an interactive UI feature is not a supported machine integration.

## Codex

| Concern                | Finding                                                                                                                                                                                                   | Classification                     |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| Client detection       | The official Codex repository documents the `codex` CLI on Windows, macOS, and Linux. LimitLens checks only whether `codex` resolves on `PATH`.                                                           | Supported                          |
| Account authentication | The CLI supports ChatGPT plan sign-in and API-key operation. LimitLens does not read either credential.                                                                                                   | Supported client feature; not used |
| Usage data             | The official Codex app-server JSON-RPC protocol exposes `account/rateLimits/read`. Its result contains primary and secondary quota windows, `usedPercent`, and Unix `resetsAt` timestamps.                | Supported                          |
| Process lifecycle      | `codex app-server` is spawned directly for a single read, receives `initialize` and `initialized`, has an eight-second timeout, and is terminated after the response. On Windows, LimitLens resolves the native executable behind the official npm shim rather than invoking a command shell. No model thread or turn is created. | Supported                          |
| Windows/macOS/Linux    | Official binaries and install paths are documented for all target desktop platforms.                                                                                                                      | Supported                          |

Result: the adapter detects Codex and retrieves primary and secondary quota data through the documented app-server protocol. It does not start a terminal UI, invoke a model request, scrape a browser, or read Codex authentication state. The secondary window is normalized as weekly; if Codex omits it, LimitLens displays weekly usage as unavailable.

Sources: [Codex CLI repository](https://github.com/openai/codex), [Codex app-server protocol](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md), [OpenAI Codex pricing](https://learn.chatgpt.com/docs/pricing).

## Claude Code

| Concern                | Finding                                                                                                                                                                                             | Classification                        |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| Client detection       | Anthropic documents the `claude` client for Windows, macOS, and Linux. LimitLens checks only `PATH`.                                                                                                | Supported                             |
| Rate-limit fields      | The official custom status-line payload includes `rate_limits.five_hour` and `rate_limits.seven_day`, each with `used_percentage` and `resets_at`, for Claude.ai subscribers after an API response. | Supported in-process status-line data |
| Usage command          | Anthropic documents interactive `/usage` and desktop usage UI.                                                                                                                                      | Interactive only                      |
| Standalone machine API | No documented local API, JSON command, or credential-free endpoint exists for a separate desktop utility.                                                                                           | Unavailable                           |
| Privacy                | A status-line approach requires changing/participating in an active Claude session and can receive session data. LimitLens will not install scripts or capture session data.                        | Not used                              |

Result: the adapter detects Claude Code but reports usage retrieval as **Unsupported** rather than attempting status-line injection or output parsing.

Sources: [Claude Code repository](https://github.com/anthropics/claude-code), [status line data contract](https://code.claude.com/docs/en/statusline), [usage-limit errors](https://code.claude.com/docs/en/errors).

## Kimi Code

| Concern                   | Finding                                                                                                                                                                                                         | Classification                     |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| Client detection          | The official Kimi Code repository documents the `kimi` executable and `kimi --version` on Windows, macOS, and Linux. LimitLens checks only `PATH`.                                                              | Supported                          |
| Authentication            | Kimi Code supports Kimi OAuth or an API key. LimitLens does not read configuration or keys.                                                                                                                     | Supported client feature; not used |
| Usage display             | Official Kimi material documents interactive `/usage` for token/context/quota information.                                                                                                                      | Interactive only                   |
| Structured external query | No stable official non-interactive quota command, local protocol, or integration API was found. `stream-json` is response transcript output and requires a prompt, so it is unsuitable and would consume quota. | Unavailable                        |

Result: the adapter detects Kimi Code but reports usage retrieval as **Unsupported**. It never starts `kimi`, submits a prompt, reads `~/.kimi-code/config.toml`, or exposes API keys.

Sources: [Kimi Code repository](https://github.com/MoonshotAI/kimi-code), [Kimi command reference](https://github.com/MoonshotAI/kimi-code/blob/main/docs/en/reference/kimi-command.md), [Kimi Code overview](https://www.kimi.com/resources/kimi-code-introduction).

## Re-evaluation trigger

Before changing an adapter, re-check its official documentation and repository. A provider becomes supported only when it offers a documented, stable, non-generative, machine-readable usage interface that can be called without copying secrets or scraping a UI.
