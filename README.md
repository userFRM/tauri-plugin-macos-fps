# tauri-plugin-macos-fps

[![Crates.io](https://img.shields.io/crates/v/tauri-plugin-macos-fps.svg)](https://crates.io/crates/tauri-plugin-macos-fps)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

**Unlock >60fps on macOS for Tauri v2 apps.** One line of code. 120Hz ProMotion, 144Hz+ external displays — your Tauri app renders at the display's native refresh rate.

> **macOS 26+ (Tahoe):** Apple removed the 60fps cap entirely in macOS 26. WKWebView now renders at native refresh rate by default. This plugin is a **no-op on macOS 26+** and is most useful for users on **macOS 13–15** (Ventura through Sequoia) where the cap is still enforced.

> **Not App Store safe.** This plugin uses WebKit's private `_features` API. Apps distributed through the Mac App Store **will be rejected**. This is not a limitation of the plugin — Apple provides **no public API** to control WKWebView's frame rate. We investigated every alternative. [Read why.](#why-this-cant-be-app-store-safe)

---

## The problem

On **macOS 13–15**, WKWebView **caps `requestAnimationFrame` at 60fps** — regardless of your display's actual refresh rate. Your MacBook Pro has a 120Hz ProMotion display? Your Tauri app is stuck at 60fps. Your external monitor runs at 144Hz or 240Hz? Still 60fps.

This is tracked in:
- [tauri-apps/tauri#13978](https://github.com/tauri-apps/tauri/issues/13978) — closed as "not planned"
- [WebKit Bug #173434](https://bugs.webkit.org/show_bug.cgi?id=173434) — open since 2017
- [WebKit Bug #294338](https://bugs.webkit.org/show_bug.cgi?id=294338) — feature request, still open

The Tauri maintainers said *"WKWebView simply does not expose settings for that."*

They were right about the public API. But there is a private one.

## The fix

WebKit has an internal preference called `PreferPageRenderingUpdatesNear60FPSEnabled` (defaults to `true`). This plugin sets it to `false` via the private `_features` API on `WKPreferences` — the same mechanism Safari uses internally.

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_macos_fps::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**That's it.** Every webview now renders at your display's native refresh rate.

## Install

Add to your Tauri app's `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri-plugin-macos-fps = "0.1"
```

Minimum supported Rust version: **1.85**.

## Try it yourself

A test app is included to verify the fix on your Mac:

```sh
git clone https://github.com/userFRM/tauri-plugin-macos-fps.git
cd tauri-plugin-macos-fps/examples/fps-test

# Install Tauri CLI if you don't have it
cargo install tauri-cli --version "^2"

# Run the test app
cargo tauri dev
```

The app shows a real-time FPS counter with a large display, min/max/avg stats, a live graph, and a toggle button to switch between locked (60fps) and unlocked (native) in real time.

### Expected results

| macOS version | Without plugin | With plugin |
|---|---|---|
| **macOS 13–15** (Ventura–Sequoia) | ~60 FPS | 120 FPS on ProMotion, 144+ on external |
| **macOS 26+** (Tahoe) | Already native refresh rate | No change (no-op) |

## macOS version compatibility

| macOS | 60fps cap exists? | Plugin effect |
|---|---|---|
| **13 Ventura** | Yes | Unlocks native refresh rate |
| **14 Sonoma** | Yes | Unlocks native refresh rate |
| **15 Sequoia** | Yes | Unlocks native refresh rate |
| **26 Tahoe** | **No** — Apple removed the cap | No-op (already unlocked) |

On macOS 26+, the `PreferPageRenderingUpdatesNear60FPSEnabled` preference still exists in WebKit's feature list and can be toggled, but WKWebView ignores it. The plugin detects this gracefully and logs success, though the toggle has no observable effect.

Tested on macOS 26.3.1 (Build 25D2128) with a 120Hz ProMotion display and a 165Hz external monitor — both ran at native refresh rate with or without the plugin.

## Configuration

Optional. Set in `tauri.conf.json` to disable the plugin without removing it:

```json
{
  "plugins": {
    "macos-fps": {
      "enabled": false
    }
  }
}
```

Default: `enabled: true` — all webviews are automatically unlocked.

## Per-webview control

For fine-grained control, use the extension trait:

```rust
use tauri_plugin_macos_fps::MacFpsExt;

// Unlock native refresh rate on a specific webview:
webview.unlock_fps()?;

// Re-lock to 60fps:
webview.lock_fps()?;
```

## How it works

```
WKWebView
  → .configuration  → WKWebViewConfiguration
    → .preferences   → WKPreferences
      → [WKPreferences _features]  → NSArray<_WKFeature *>
        → find key == "PreferPageRenderingUpdatesNear60FPSEnabled"
          → [preferences _setEnabled:NO forFeature:feature]
```

The plugin hooks into Tauri's `on_webview_ready` lifecycle event. Every webview is automatically unlocked as it's created. The private `_features` API returns all WebKit feature flags; we find our target key and toggle it off.

## Why this can't be App Store safe

We exhaustively investigated every possible public API path. None of them work. Here's why:

### NSUserDefaults — won't work

The obvious idea: set `PreferPageRenderingUpdatesNear60FPSEnabled = false` via `NSUserDefaults` (a public API) before the webview loads. We traced through the WebKit source code ([`WebPreferencesCocoa.mm`](https://github.com/nicolo-ribaudo/tc39-proposal-signals-webkit/blob/main/Source/WebKit/UIProcess/Cocoa/WebPreferencesCocoa.mm)) and found the blocker:

```cpp
void WebPreferences::platformInitializeStore()
{
    // ...persistent preferences...
    if (!m_identifier)
        return;  // ← WKWebView with default config hits this and SKIPS reading
    FOR_EACH_PERSISTENT_WEBKIT_PREFERENCE(INITIALIZE_PREFERENCE_FROM_NSUSERDEFAULTS)
}
```

WKWebView created with a default `WKWebViewConfiguration` (which is what Tauri/wry uses) has an **empty identifier**. This causes `platformGetBoolUserValueForKey()` to return early — **NSUserDefaults is never consulted** for this preference. No matter what key format you use (`WebKitPrefer...`, `WebKit2Prefer...`, `com.apple.WebKit.Prefer...`), it won't be read.

We built a [diagnostic app](examples/fps-diag/) that tests 8 different NSUserDefaults key variations to confirm this.

### Info.plist keys — won't work

`CADisableMinimumFrameDurationOnPhone` unlocks 120Hz for native Core Animation on iOS, but has **zero effect** on WKWebView's internal rendering on any platform. No documented Info.plist key affects WKWebView frame rate.

### WKWebViewConfiguration / WKPreferences public API — doesn't exist

We checked every public property on `WKWebViewConfiguration` and `WKPreferences` through macOS 26. None affect rendering frame rate. Apple added no new frame-rate APIs in macOS 14, 15, or 26.

### CADisplayLink / Core Animation — can't reach the compositor

WKWebView's rendering pipeline is entirely internal to the WebKit process. Setting `preferredFrameRateRange` on the webview's backing layer doesn't affect WebKit's internal compositor. You can't drive it from outside.

### KVC (Key-Value Coding) backdoors — Apple is killing them

Recent WebKit commits now **raise exceptions** when apps use `setValue:forKey:` on undocumented `WKWebViewConfiguration` properties when linked against newer SDKs. Apple is actively closing this door.

### The bottom line

The **only** way to toggle `PreferPageRenderingUpdatesNear60FPSEnabled` at runtime is through the private `_features` / `_setEnabled:forFeature:` API. There is no public alternative. Apple's own Safari browser uses this same private API internally.

We filed this research against [WebKit Bug #294338](https://bugs.webkit.org/show_bug.cgi?id=294338). A future WebKit proposal for CSS [`animation-frame-rate`](https://github.com/WebKit/explainers/tree/main/animation-frame-rate) may eventually provide a public path, but it hasn't shipped in any stable Safari release.

## Who should use this plugin

This plugin is for apps distributed **outside** the Mac App Store:

- **Direct distribution** (`.dmg`, Homebrew, GitHub releases, `cargo install`)
- **Internal / enterprise** apps
- **Development and testing**
- **Open source** projects where users build from source

If you need Mac App Store distribution, you cannot use this plugin. There is currently no alternative — the only option is to wait for Apple to provide a public API or for macOS 26+ adoption to make the cap irrelevant.

## Platform behavior

| Platform | Effect |
|---|---|
| **macOS 13–15** (WKWebView) | Unlocks native refresh rate (120Hz ProMotion, 144Hz+, etc.) |
| **macOS 26+** (WKWebView) | No-op — Apple removed the cap, already runs at native refresh rate |
| **Windows** (WebView2) | No-op — WebView2 already renders at native refresh rate |
| **Linux** (WebKitGTK) | No-op — typically 60fps, no known toggle |
| **iOS** (WKWebView) | No-op — untested, may work in future versions |

The plugin compiles and runs on all platforms. On non-macOS, `init()` registers a plugin that does nothing.

## Graceful degradation

The plugin never crashes. If the private API changes in a future macOS version:

| Scenario | Behavior |
|---|---|
| `_features` returns nil | Warning logged, app continues at 60fps |
| Target preference key not found | Warning logged, app continues at 60fps |
| `WKPreferences` class missing | Warning logged, app continues at 60fps |
| WKWebView pointer is null | Warning logged, app continues at 60fps |

All failure paths log via the standard `log` crate at `WARN` level and fall back silently to the default 60fps behavior.

## Verify in any Tauri app

Paste this in your browser console or embed it in your frontend to measure actual FPS:

```javascript
let frames = 0, lastTime = performance.now();
(function measure() {
    frames++;
    const now = performance.now();
    if (now - lastTime >= 1000) {
        console.log(`${Math.round(frames * 1000 / (now - lastTime))} FPS`);
        frames = 0;
        lastTime = now;
    }
    requestAnimationFrame(measure);
})();
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under either of

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

## Links

- [tauri-apps/tauri#13978](https://github.com/tauri-apps/tauri/issues/13978) — Original Tauri issue
- [WebKit Bug #173434](https://bugs.webkit.org/show_bug.cgi?id=173434) — WebKit tracking bug (open since 2017)
- [WebKit Bug #294338](https://bugs.webkit.org/show_bug.cgi?id=294338) — Feature request for WKWebView 120Hz support
- [WebKit `animation-frame-rate` proposal](https://github.com/WebKit/explainers/tree/main/animation-frame-rate) — Future public API (not shipped)
- [WICG Proposal #165](https://github.com/WICG/proposals/issues/165) — Permission-based high frame rate API proposal
- [WebKit Source — PreferPageRenderingUpdatesNear60FPSEnabled](https://github.com/nicolo-ribaudo/tc39-proposal-signals-webkit/blob/main/Source/WTF/Scripts/Preferences/WebPreferences.yaml) — Preference definition in WebKit source
