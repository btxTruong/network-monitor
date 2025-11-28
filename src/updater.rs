//! Auto-update checker module
//! Checks for new versions once per day and notifies user.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CHECK_INTERVAL_SECS: u64 = 86400; // 24 hours
const GITHUB_API_URL: &str =
    "https://api.github.com/repos/btxTruong/network-monitor/releases/latest";

/// Current app version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Gets the config directory path
fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("network-monitor"))
}

/// Checks if we should check for updates (once per day)
fn should_check() -> bool {
    let Some(config) = config_dir() else {
        return false;
    };

    let last_check_file = config.join("last-check");
    if !last_check_file.exists() {
        return true;
    }

    let Ok(content) = fs::read_to_string(&last_check_file) else {
        return true;
    };

    let Ok(last_check) = content.trim().parse::<u64>() else {
        return true;
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();

    now - last_check >= CHECK_INTERVAL_SECS
}

/// Saves the current timestamp as last check time
fn save_last_check() {
    let Some(config) = config_dir() else {
        return;
    };

    let _ = fs::create_dir_all(&config);
    let last_check_file = config.join("last-check");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();

    let _ = fs::write(last_check_file, now.to_string());
}

/// Response from GitHub API
#[derive(Debug, serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Checks for updates and returns new version if available (respects daily limit)
pub async fn check_for_update() -> Option<String> {
    if !should_check() {
        return None;
    }
    save_last_check();
    check_for_update_internal().await
}

/// Checks for updates immediately (ignores daily limit)
pub async fn check_for_update_forced() -> Option<String> {
    check_for_update_internal().await
}

async fn check_for_update_internal() -> Option<String> {
    tracing::debug!("Checking for updates...");

    let client = reqwest::Client::new();
    let response = client
        .get(GITHUB_API_URL)
        .header("User-Agent", "network-monitor")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?;

    let release: GitHubRelease = response.json().await.ok()?;

    // Remove 'v' prefix if present for comparison
    let latest = release.tag_name.trim_start_matches('v');
    let current = VERSION;

    if latest != current && is_newer_version(latest, current) {
        tracing::info!("New version available: v{} (current: v{})", latest, current);
        Some(release.tag_name)
    } else {
        tracing::debug!("Already on latest version: v{}", current);
        None
    }
}

/// Saves available update version to persist across restarts
pub fn save_available_update(version: &str) {
    let Some(config) = config_dir() else { return };
    let _ = fs::create_dir_all(&config);
    let _ = fs::write(config.join("update-available"), version);
}

/// Loads persisted update version (if still newer than current)
pub fn load_available_update() -> Option<String> {
    let config = config_dir()?;
    let version = fs::read_to_string(config.join("update-available")).ok()?;
    let version = version.trim().to_string();
    let latest = version.trim_start_matches('v');
    if is_newer_version(latest, VERSION) {
        Some(version)
    } else {
        // Clear stale update file
        let _ = fs::remove_file(config.join("update-available"));
        None
    }
}

/// Clears the persisted update file (after successful update)
pub fn clear_available_update() {
    if let Some(config) = config_dir() {
        let _ = fs::remove_file(config.join("update-available"));
    }
}

/// Simple version comparison (assumes semver x.y.z)
fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let latest_parts = parse(latest);
    let current_parts = parse(current);

    for i in 0..3 {
        let l = latest_parts.get(i).copied().unwrap_or(0);
        let c = current_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("0.2.0", "0.1.0"));
        assert!(is_newer_version("1.0.0", "0.9.9"));
        assert!(is_newer_version("0.1.1", "0.1.0"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.1.0", "0.2.0"));
    }
}
