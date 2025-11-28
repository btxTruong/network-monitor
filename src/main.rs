//! Network Monitor - System tray app displaying country flag based on network location
//!
//! Features:
//! - Displays country flag in system tray based on geo-IP location
//! - Shows IP, country, city, ISP on click
//! - Auto-refreshes every 1 minute
//! - Refreshes on network connectivity changes
//! - Optional autostart on login

mod autostart;
mod geo;
mod icons;
mod network;
mod tray;
mod updater;

use crate::autostart::{is_autostart_enabled, remove_autostart, setup_autostart};
use crate::geo::{fetch_location, GeoInfo};
use crate::network::{watch_network_changes, NetworkEvent};
use crate::tray::{NetworkTray, TrayCommand};
use ksni::TrayMethods;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use notify_rust::Notification;
use tracing::{error, info, warn};

const REFRESH_INTERVAL: Duration = Duration::from_secs(1 * 60); // 1 minutes

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Handle --update flag
    if args.iter().any(|a| a == "--update" || a == "-u") {
        run_update();
        return;
    }

    // Handle --check flag
    if args.iter().any(|a| a == "--check" || a == "-c") {
        run_check().await;
        return;
    }

    // Handle --version flag
    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("network-monitor {}", updater::VERSION);
        return;
    }

    // Handle --help flag
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("Network Monitor v{} starting...", updater::VERSION);
    info!("Loaded {} flag icons", icons::flag_count());

    // Fetch location FIRST before showing tray (so flag is ready)
    info!("Fetching initial location...");
    let initial_geo = match fetch_location().await {
        Ok(info) => {
            info!("Initial location: {} ({}) - {}", info.country, info.country_code, info.query);
            Some(info)
        }
        Err(e) => {
            warn!("Failed to fetch initial location: {}", e);
            None
        }
    };

    // Shared state for geo info (pre-populated with initial fetch)
    let geo_info: Arc<Mutex<Option<GeoInfo>>> = Arc::new(Mutex::new(initial_geo));

    // Command channel from tray menu
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<TrayCommand>(16);

    // Network event channel
    let (net_tx, mut net_rx) = mpsc::channel::<NetworkEvent>(16);

    // Check autostart status
    let autostart_enabled = is_autostart_enabled();
    info!("Autostart enabled: {}", autostart_enabled);

    // Create tray (geo_info already has location data)
    let tray = NetworkTray::new(geo_info.clone(), cmd_tx.clone(), autostart_enabled);

    // Start tray service - icon will show correct flag immediately
    let tray_handle = tray.spawn().await.expect("Failed to spawn tray service");

    // Load persisted update state or check for updates (once per day)
    if let Some(persisted_version) = updater::load_available_update() {
        info!("Persisted update available: {}", persisted_version);
        tray_handle.update(move |tray: &mut NetworkTray| {
            tray.update_available = Some(persisted_version.clone());
        }).await;
    } else if let Some(new_version) = updater::check_for_update().await {
        updater::save_available_update(&new_version);
        tray_handle.update(move |tray: &mut NetworkTray| {
            tray.update_available = Some(new_version.clone());
        }).await;
    }

    // Spawn network monitor task
    let net_tx_clone = net_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = watch_network_changes(net_tx_clone).await {
            error!("Network monitor error: {}", e);
        }
    });

    // Spawn periodic refresh task
    let geo_info_refresh = geo_info.clone();
    let tray_handle_refresh = tray_handle.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(REFRESH_INTERVAL);
        interval.tick().await; // Skip immediate tick

        loop {
            interval.tick().await;
            info!("Periodic refresh triggered");

            match fetch_location().await {
                Ok(info) => {
                    info!("Location updated: {} ({})", info.country, info.country_code);
                    if let Ok(mut guard) = geo_info_refresh.lock() {
                        *guard = Some(info);
                    }
                    tray_handle_refresh.update(|_| {}).await;
                }
                Err(e) => {
                    warn!("Failed to refresh location: {}", e);
                }
            }
        }
    });

    // Main event loop
    let mut current_autostart = autostart_enabled;

    loop {
        tokio::select! {
            // Handle tray menu commands
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    TrayCommand::Refresh => {
                        info!("Manual refresh requested");
                        match fetch_location().await {
                            Ok(info) => {
                                info!("Location: {} ({})", info.country, info.country_code);
                                if let Ok(mut guard) = geo_info.lock() {
                                    *guard = Some(info);
                                }
                                tray_handle.update(|_| {}).await;
                            }
                            Err(e) => {
                                error!("Refresh failed: {}", e);
                            }
                        }
                    }
                    TrayCommand::ToggleAutostart => {
                        if current_autostart {
                            if let Err(e) = remove_autostart() {
                                error!("Failed to disable autostart: {}", e);
                            } else {
                                current_autostart = false;
                                info!("Autostart disabled");
                            }
                        } else {
                            if let Err(e) = setup_autostart() {
                                error!("Failed to enable autostart: {}", e);
                            } else {
                                current_autostart = true;
                                info!("Autostart enabled");
                            }
                        }
                        // Update tray to reflect new autostart state
                        let new_autostart = current_autostart;
                        tray_handle.update(move |tray: &mut NetworkTray| {
                            tray.autostart_enabled = new_autostart;
                        }).await;
                    }
                    TrayCommand::CheckUpdate => {
                        info!("Check for updates requested");
                        // Show spinner and notification
                        tray_handle.update(|tray: &mut NetworkTray| {
                            tray.checking_update = true;
                        }).await;
                        let _ = Notification::new()
                            .summary("Network Monitor")
                            .body("Checking for updates...")
                            .icon("network-monitor")
                            .timeout(2000)
                            .show();

                        let result = updater::check_for_update_forced().await;

                        // Hide spinner and update result
                        if let Some(new_version) = result {
                            info!("Update available: {}", new_version);
                            updater::save_available_update(&new_version);
                            let _ = Notification::new()
                                .summary("Network Monitor")
                                .body(&format!("Update {} available! Click tray menu to install.", new_version))
                                .icon("network-monitor")
                                .timeout(5000)
                                .show();
                            tray_handle.update(move |tray: &mut NetworkTray| {
                                tray.checking_update = false;
                                tray.update_available = Some(new_version.clone());
                            }).await;
                        } else {
                            info!("Already on latest version");
                            let _ = Notification::new()
                                .summary("Network Monitor")
                                .body("You're running the latest version!")
                                .icon("network-monitor")
                                .timeout(3000)
                                .show();
                            tray_handle.update(|tray: &mut NetworkTray| {
                                tray.checking_update = false;
                            }).await;
                        }
                    }
                    TrayCommand::RunUpdate => {
                        info!("Running update...");
                        updater::clear_available_update();
                        // Spawn update in background and quit
                        std::process::Command::new("bash")
                            .args(["-c", "curl -sSL https://raw.githubusercontent.com/btxTruong/network-monitor/main/install.sh | bash -s -- --update"])
                            .spawn()
                            .ok();
                        break;
                    }
                    TrayCommand::Quit => {
                        info!("Quit requested");
                        break;
                    }
                }
            }

            // Handle network events
            Some(event) = net_rx.recv() => {
                match event {
                    NetworkEvent::Connected => {
                        info!("Network connected - refreshing location");
                        // Small delay to allow network to stabilize
                        tokio::time::sleep(Duration::from_secs(2)).await;

                        match fetch_location().await {
                            Ok(info) => {
                                info!("Location: {} ({})", info.country, info.country_code);
                                if let Ok(mut guard) = geo_info.lock() {
                                    *guard = Some(info);
                                }
                                tray_handle.update(|_| {}).await;
                            }
                            Err(e) => {
                                warn!("Failed to fetch location after connect: {}", e);
                            }
                        }
                    }
                    NetworkEvent::Disconnected => {
                        info!("Network disconnected");
                        // Optionally clear geo info or show disconnected state
                    }
                }
            }
        }
    }

    info!("Network Monitor shutting down");
}

fn print_help() {
    println!("network-monitor {}", updater::VERSION);
    println!();
    println!("System tray app displaying country flag based on network location.");
    println!();
    println!("USAGE:");
    println!("    network-monitor [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help      Show this help message");
    println!("    -v, --version   Show version");
    println!("    -c, --check     Check for updates");
    println!("    -u, --update    Update to latest version");
}

fn run_update() {
    println!("Updating Network Monitor...");
    let status = std::process::Command::new("bash")
        .args(["-c", "curl -sSL https://raw.githubusercontent.com/btxTruong/network-monitor/main/install.sh | bash -s -- --update"])
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Update failed"),
        Err(e) => eprintln!("Failed to run update: {}", e),
    }
}

async fn run_check() {
    println!("network-monitor {}", updater::VERSION);
    println!();
    println!("Checking for updates...");

    match updater::check_for_update_forced().await {
        Some(new_version) => {
            println!("Update available: {}", new_version);
            println!();
            println!("Run 'network-monitor --update' to update.");
        }
        None => {
            println!("You're up to date!");
        }
    }
}
