use serde::Serialize;

/// All NSUserDefaults key variations we attempt for disabling the 60fps cap.
/// These are all public API calls via NSUserDefaults — no private APIs used.
const PREFERENCE_KEYS: &[&str] = &[
    "PreferPageRenderingUpdatesNear60FPSEnabled",
    "WebKitPreferPageRenderingUpdatesNear60FPSEnabled",
    "WebKit.PreferPageRenderingUpdatesNear60FPSEnabled",
    "com.apple.WebKit.PreferPageRenderingUpdatesNear60FPSEnabled",
    "WebKitPreferences.preferPageRenderingUpdatesNear60FPSEnabled",
    "com.apple.WebKit.WebPreferences.PreferPageRenderingUpdatesNear60FPSEnabled",
    "WebKit2PreferPageRenderingUpdatesNear60FPSEnabled",
    "com.apple.WebKit2.PreferPageRenderingUpdatesNear60FPSEnabled",
];

#[derive(Debug, Clone, Serialize)]
pub struct KeyStatus {
    pub key: String,
    pub value: bool,
}

// ──────────────────────────────────────────────────────────
// macOS implementation using objc2
// ──────────────────────────────────────────────────────────
#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use objc2_foundation::{NSString, NSUserDefaults};

    /// Set a single boolean NSUserDefaults key.
    pub fn set_bool_default(key: &str, value: bool) {
        unsafe {
            let defaults = NSUserDefaults::standardUserDefaults();
            let ns_key = NSString::from_str(key);
            defaults.setBool_forKey(value, &ns_key);
        }
    }

    /// Read a single boolean NSUserDefaults key.
    pub fn get_bool_default(key: &str) -> bool {
        unsafe {
            let defaults = NSUserDefaults::standardUserDefaults();
            let ns_key = NSString::from_str(key);
            defaults.boolForKey(&ns_key)
        }
    }

    /// Set ALL known key variations to `value` (false = attempt to uncap).
    pub fn set_all_keys(value: bool) -> Vec<KeyStatus> {
        PREFERENCE_KEYS
            .iter()
            .map(|&key| {
                set_bool_default(key, value);
                let readback = get_bool_default(key);
                KeyStatus {
                    key: key.to_string(),
                    value: readback,
                }
            })
            .collect()
    }

    /// Read the current value of all known keys.
    pub fn read_all_keys() -> Vec<KeyStatus> {
        PREFERENCE_KEYS
            .iter()
            .map(|&key| KeyStatus {
                key: key.to_string(),
                value: get_bool_default(key),
            })
            .collect()
    }
}

// ──────────────────────────────────────────────────────────
// Non-macOS stub (compiles but does nothing)
// ──────────────────────────────────────────────────────────
#[cfg(not(target_os = "macos"))]
mod platform {
    use super::*;

    pub fn set_bool_default(_key: &str, _value: bool) {}

    pub fn get_bool_default(_key: &str) -> bool {
        false
    }

    pub fn set_all_keys(value: bool) -> Vec<KeyStatus> {
        PREFERENCE_KEYS
            .iter()
            .map(|&key| KeyStatus {
                key: key.to_string(),
                value,
            })
            .collect()
    }

    pub fn read_all_keys() -> Vec<KeyStatus> {
        PREFERENCE_KEYS
            .iter()
            .map(|&key| KeyStatus {
                key: key.to_string(),
                value: false,
            })
            .collect()
    }
}

// ──────────────────────────────────────────────────────────
// Tauri commands
// ──────────────────────────────────────────────────────────

/// Get the list of all key variations and their current values.
#[tauri::command]
fn get_all_keys() -> Vec<KeyStatus> {
    platform::read_all_keys()
}

/// Toggle a single key by name. Returns new status of ALL keys.
#[tauri::command]
fn toggle_key(key: String, value: bool) -> Vec<KeyStatus> {
    platform::set_bool_default(&key, value);
    platform::read_all_keys()
}

/// Set ALL keys to false (attempt to uncap) and return their readback values.
#[tauri::command]
fn uncap_all() -> Vec<KeyStatus> {
    platform::set_all_keys(false)
}

/// Set ALL keys to true (re-enable 60fps cap) and return their readback values.
#[tauri::command]
fn cap_all() -> Vec<KeyStatus> {
    platform::set_all_keys(true)
}

// ──────────────────────────────────────────────────────────
// App entry
// ──────────────────────────────────────────────────────────

pub fn run() {
    // IMPORTANT: Set all keys BEFORE creating any webview.
    // NSUserDefaults writes are process-wide and take effect immediately,
    // so WKWebView should pick these up when it initialises.
    let initial = platform::set_all_keys(false);
    eprintln!("[fps-diag] Set {} NSUserDefaults keys to false BEFORE webview creation:", initial.len());
    for ks in &initial {
        eprintln!("  {} = {} (readback)", ks.key, ks.value);
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_all_keys,
            toggle_key,
            uncap_all,
            cap_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
