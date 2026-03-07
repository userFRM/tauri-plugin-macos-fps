# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-07

### Added

- Initial release
- Automatic >60fps unlock via `on_webview_ready` lifecycle hook
- `MacFpsExt` trait for per-webview `unlock_fps()` / `lock_fps()` control
- `Config` support via `tauri.conf.json` (`plugins.macos-fps.enabled`)
- Graceful degradation — never crashes, logs warnings on failure
- No-op on non-macOS platforms
- Example test app (`examples/fps-test/`) for verification

### Tested

- Verified on macOS 26.3.1 (Tahoe, Build 25D2128): plugin API works correctly, but Apple removed the 60fps cap in macOS 26 — WKWebView renders at native refresh rate by default
- Plugin is most useful on macOS 13–15 (Ventura through Sequoia) where the cap is still enforced
- Tested with 120Hz ProMotion display and 165Hz external monitor
