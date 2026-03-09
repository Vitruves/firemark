use std::time::Duration;

use log::info;

const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CRATES_IO_API: &str = "https://crates.io/api/v1/crates/firemark";

/// Check crates.io for a newer version. Silently returns on any failure
/// (no internet, timeout, parse error) so it never blocks the user.
pub fn check_for_update() {
    // Spawn in a best-effort fashion — 2 second timeout max.
    let result = std::panic::catch_unwind(|| try_check());
    if let Ok(Some(latest)) = result {
        if is_newer(&latest, CURRENT_VERSION) {
            info!(
                "Update available: {CRATE_NAME} v{latest} (you have v{CURRENT_VERSION}). \
                 Run `cargo install {CRATE_NAME}` to update."
            );
        }
    }
}

fn try_check() -> Option<String> {
    let resp = ureq::get(CRATES_IO_API)
        .set("User-Agent", &format!("{CRATE_NAME}/{CURRENT_VERSION}"))
        .timeout(Duration::from_secs(2))
        .call()
        .ok()?;

    let body: serde_json::Value = resp.into_json().ok()?;
    let latest = body
        .get("crate")?
        .get("max_stable_version")?
        .as_str()?
        .to_string();

    Some(latest)
}

/// Simple semver comparison: returns true if `latest` > `current`.
fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> (u32, u32, u32) {
        let mut parts = s.split('.').filter_map(|p| p.parse::<u32>().ok());
        (
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        )
    };
    parse(latest) > parse(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("0.2.0", "0.1.1"));
        assert!(is_newer("0.1.2", "0.1.1"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.1", "0.1.1"));
        assert!(!is_newer("0.1.0", "0.1.1"));
    }
}
