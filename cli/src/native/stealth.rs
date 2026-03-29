use std::sync::Arc;
use crate::native::cdp::client::CdpClient;

const STEALTH_JS: &str = include_str!("stealth.js");

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
}
