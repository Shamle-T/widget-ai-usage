# Contributing to LimitLens

## Before contributing

This project intentionally has no chosen software license yet. Do not assume contributions are accepted under MIT, Apache-2.0, or any other license until the maintainer makes that decision.

## Development

```powershell
npm.cmd install
npm.cmd run format:write
npm.cmd run lint
npm.cmd test
npm.cmd run check
```

Install the Rust toolchain and Tauri platform prerequisites before running `npm.cmd run tauri:build`.

## Provider adapters

Provider changes must:

- cite current official documentation or an official repository;
- avoid browser scraping, terminal-screen parsing, prompt generation, and credential copying;
- use direct executable-and-argument process spawning if a process becomes necessary;
- include parser fixtures, malformed-data tests, and stale/error-state tests;
- document platform differences in `docs/provider-research.md`;
- report unavailable rather than estimate a quota.

## Pull requests

Keep pull requests focused. Run formatting, TypeScript checks, frontend tests, Rust formatting, Rust tests, and release build checks that your environment supports. State any checks that could not run.
