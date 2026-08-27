# Changelog

All notable changes to LimitLens will be documented in this file.

## [Unreleased]

### Added

- Initial Tauri 2 widget architecture with normalized provider-state model.
- Safe executable detection adapters for Codex, Claude Code, and Kimi Code.
- Native per-provider taskbar usage rings with independent left-click window cycling.
- Native taskbar Settings menu, preferences, cache, polling, and documentation foundations.
- Windows taskbar-embedded provider strip with persisted horizontal dragging and Explorer recovery.

### Changed

- Replaced the floating WebView widget and settings window with native taskbar and notification-area surfaces.
- Removed the global weekly override; every provider retains its own selected usage window.

### Security

- No provider usage scraping, token copying, browser-session access, or model-generated quota checks.
