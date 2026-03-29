use std::sync::Arc;
use crate::native::cdp::client::CdpClient;

const STEALTH_JS: &str = include_str!("stealth.js");

/// Parse Chrome major version and full version string from a User-Agent string.
/// Returns `(major, full)` where major is e.g. `"124"` and full is e.g. `"124.0.0.0"`.
/// Falls back to `("124", "124.0.0.0")` if parsing fails.
fn parse_chrome_version(ua: &str) -> (&str, &str) {
    // Find "Chrome/X.Y.Z.W" in the UA string
    if let Some(start) = ua.find("Chrome/") {
        let version_str = &ua[start + 7..]; // skip "Chrome/"
        // Find the end of the version (space or end of string)
        let end = version_str.find(|c: char| c == ' ' || c == ')').unwrap_or(version_str.len());
        let full = &version_str[..end];
        // Major version is everything before the first '.'
        let major_end = full.find('.').unwrap_or(full.len());
        let major = &full[..major_end];
        if !major.is_empty() && !full.is_empty() {
            return (major, full);
        }
    }
    ("124", "124.0.0.0")
}

/// Parse Chrome version from a User-Agent string and build userAgentMetadata.
/// Returns a serde_json::Value suitable for Emulation.setUserAgentOverride.
pub fn build_stealth_ua_metadata(user_agent: &str) -> serde_json::Value {
    let (major, full) = parse_chrome_version(user_agent);

    serde_json::json!({
        "brands": [
            {"brand": "Chromium", "version": major},
            {"brand": "Google Chrome", "version": major},
            {"brand": "Not-A.Brand", "version": "99"}
        ],
        "fullVersionList": [
            {"brand": "Chromium", "version": full},
            {"brand": "Google Chrome", "version": full}
        ],
        "platform": if cfg!(target_os = "macos") { "macOS" } else if cfg!(target_os = "windows") { "Windows" } else { "Linux" },
        "platformVersion": if cfg!(target_os = "macos") { "14.0.0" } else if cfg!(target_os = "windows") { "15.0.0" } else { "6.1.0" },
        "architecture": if cfg!(target_arch = "aarch64") { "arm" } else { "x86" },
        "bitness": if cfg!(target_arch = "x86_64") || cfg!(target_arch = "aarch64") { "64" } else { "32" },
        "model": "",
        "mobile": false,
        "wow64": false
    })
}

/// Injects stealth evasion scripts via Page.addScriptToEvaluateOnNewDocument.
/// This is a single CDP call that registers the script to run before any page JS.
/// Fail-open: errors are returned to the caller for logging, not panicked.
pub async fn inject_stealth_scripts(
    client: &Arc<CdpClient>,
    session_id: &str,
) -> Result<(), String> {
    let params = serde_json::json!({
        "source": STEALTH_JS,
    });

    client
        .send_command(
            "Page.addScriptToEvaluateOnNewDocument",
            Some(params),
            Some(session_id),
        )
        .await
        .map_err(|e| format!("Stealth injection failed: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chrome_version_typical() {
        let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
        let (major, full) = parse_chrome_version(ua);
        assert_eq!(major, "124");
        assert_eq!(full, "124.0.0.0");
    }

    #[test]
    fn test_parse_chrome_version_different_version() {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.6099.71 Safari/537.36";
        let (major, full) = parse_chrome_version(ua);
        assert_eq!(major, "120");
        assert_eq!(full, "120.0.6099.71");
    }

    #[test]
    fn test_parse_chrome_version_fallback() {
        let ua = "Mozilla/5.0 (compatible; MSIE 10.0; Windows NT 6.1)";
        let (major, full) = parse_chrome_version(ua);
        assert_eq!(major, "124");
        assert_eq!(full, "124.0.0.0");
    }

    #[test]
    fn test_build_stealth_ua_metadata_structure() {
        let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
        let metadata = build_stealth_ua_metadata(ua);

        // brands array has 3 entries
        let brands = metadata["brands"].as_array().expect("brands should be array");
        assert_eq!(brands.len(), 3);

        // fullVersionList has 2 entries
        let fvl = metadata["fullVersionList"].as_array().expect("fullVersionList should be array");
        assert_eq!(fvl.len(), 2);

        // version fields match parsed UA
        assert_eq!(brands[0]["version"], "124");
        assert_eq!(fvl[0]["version"], "124.0.0.0");

        // mobile is false
        assert_eq!(metadata["mobile"], false);
        assert_eq!(metadata["wow64"], false);
        assert_eq!(metadata["model"], "");
    }

    #[test]
    fn test_stealth_js_is_not_empty() {
        assert!(!STEALTH_JS.is_empty(), "stealth.js should not be empty");
    }

    #[test]
    fn test_stealth_js_contains_idempotency_guard() {
        assert!(
            STEALTH_JS.contains("__stealth_applied"),
            "stealth.js must contain idempotency guard"
        );
    }

    #[test]
    fn test_stealth_js_contains_key_evasions() {
        assert!(STEALTH_JS.contains("navigator.webdriver"), "Must patch navigator.webdriver");
        assert!(STEALTH_JS.contains("window.chrome"), "Must inject window.chrome");
        assert!(STEALTH_JS.contains("HeadlessChrome"), "Must patch HeadlessChrome UA");
        assert!(STEALTH_JS.contains("navigator.plugins"), "Must spoof navigator.plugins");
        assert!(STEALTH_JS.contains("navigator.permissions"), "Must patch navigator.permissions");
        assert!(STEALTH_JS.contains("UNMASKED_VENDOR_WEBGL"), "Must spoof WebGL vendor");
        assert!(STEALTH_JS.contains("hardwareConcurrency"), "Must normalize hardwareConcurrency");
    }

    #[test]
    fn test_stealth_js_is_iife() {
        assert!(
            STEALTH_JS.trim_start().starts_with("(function()"),
            "stealth.js must be wrapped in an IIFE"
        );
    }

    #[test]
    fn test_stealth_js_contains_window_geometry() {
        assert!(STEALTH_JS.contains("outerHeight"), "Must patch window.outerHeight");
        assert!(STEALTH_JS.contains("outerWidth"), "Must patch window.outerWidth");
        assert!(STEALTH_JS.contains("screenX"), "Must patch window.screenX");
    }

    #[test]
    fn test_stealth_js_contains_chrome_runtime_extensions() {
        assert!(STEALTH_JS.contains("getManifest"), "Must add chrome.runtime.getManifest");
        assert!(STEALTH_JS.contains("getURL"), "Must add chrome.runtime.getURL");
        assert!(STEALTH_JS.contains("getPlatformInfo"), "Must add chrome.runtime.getPlatformInfo");
    }

    #[test]
    fn test_stealth_js_contains_native_tostring() {
        assert!(STEALTH_JS.contains("[native code]"), "Must spoof Function.toString to return native code");
    }

    #[test]
    fn test_stealth_js_contains_high_entropy_values() {
        assert!(STEALTH_JS.contains("getHighEntropyValues"), "Must patch getHighEntropyValues");
    }
}
