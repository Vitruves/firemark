use std::time::Duration;

const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_RELEASES_API: &str = "https://api.github.com/repos/Vitruves/firemark/releases/latest";
const INSTALL_CMD: &str =
    "curl -fsSL https://raw.githubusercontent.com/Vitruves/firemark/main/install.sh | sh";

/// Check GitHub Releases for a newer version and print a short notice to
/// stderr if one exists. Silently returns on any failure (offline, timeout,
/// parse error) so it never blocks or annoys the user.
pub fn check_for_update() {
    let result = std::panic::catch_unwind(try_check);
    if let Ok(Some(latest)) = result {
        if is_newer(&latest, CURRENT_VERSION) {
            eprintln!(
                "\n  {CRATE_NAME} v{latest} is available (you have v{CURRENT_VERSION}).\n  \
                 Update:  {INSTALL_CMD}\n"
            );
        }
    }
}

fn try_check() -> Option<String> {
    let resp = ureq::get(GITHUB_RELEASES_API)
        .set("User-Agent", &format!("{CRATE_NAME}/{CURRENT_VERSION}"))
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(2))
        .call()
        .ok()?;

    let body: serde_json::Value = resp.into_json().ok()?;
    let tag = body.get("tag_name")?.as_str()?;
    // Release tags are of the form `v0.1.4`; strip the leading `v`.
    Some(tag.trim_start_matches('v').to_string())
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
