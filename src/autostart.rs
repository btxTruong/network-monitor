//! XDG Autostart module for automatic startup on login
//! Creates/manages .desktop file in ~/.config/autostart/

use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[allow(dead_code)]
const APP_NAME: &str = "network-monitor";
const DESKTOP_FILENAME: &str = "network-monitor.desktop";

/// Errors during autostart setup
#[derive(Debug, Error)]
pub enum AutostartError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not determine home directory")]
    NoHomeDir,
    #[error("Could not determine executable path")]
    NoExePath,
}

/// Gets the path to the autostart directory
fn autostart_dir() -> Result<PathBuf, AutostartError> {
    let config_dir = dirs::config_dir().ok_or(AutostartError::NoHomeDir)?;
    Ok(config_dir.join("autostart"))
}

/// Gets the full path to the .desktop file
fn desktop_file_path() -> Result<PathBuf, AutostartError> {
    Ok(autostart_dir()?.join(DESKTOP_FILENAME))
}

/// Creates the .desktop file content
fn create_desktop_content(exec_path: &str) -> String {
    format!(
        r#"[Desktop Entry]
Type=Application
Name=Network Monitor
Comment=Display network location country flag in system tray
Exec={exec_path}
Icon=network-workgroup
Terminal=false
Categories=Network;System;Monitor;
X-GNOME-Autostart-enabled=true
StartupNotify=false
"#,
        exec_path = exec_path
    )
}

/// Sets up autostart by creating the .desktop file
pub fn setup_autostart() -> Result<(), AutostartError> {
    let autostart_path = autostart_dir()?;
    let desktop_path = desktop_file_path()?;

    // Get current executable path
    let exe_path = std::env::current_exe()
        .map_err(|_| AutostartError::NoExePath)?
        .to_string_lossy()
        .to_string();

    // Create autostart directory if it doesn't exist
    fs::create_dir_all(&autostart_path)?;

    // Write .desktop file
    let content = create_desktop_content(&exe_path);
    fs::write(&desktop_path, content)?;

    tracing::info!("Autostart enabled: {:?}", desktop_path);
    Ok(())
}

/// Removes the autostart .desktop file
pub fn remove_autostart() -> Result<(), AutostartError> {
    let desktop_path = desktop_file_path()?;

    if desktop_path.exists() {
        fs::remove_file(&desktop_path)?;
        tracing::info!("Autostart disabled");
    }

    Ok(())
}

/// Checks if autostart is currently enabled
pub fn is_autostart_enabled() -> bool {
    desktop_file_path()
        .map(|p| p.exists())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_content_format() {
        let content = create_desktop_content("/usr/bin/network-monitor");
        assert!(content.contains("[Desktop Entry]"));
        assert!(content.contains("Type=Application"));
        assert!(content.contains("Exec=/usr/bin/network-monitor"));
    }
}
