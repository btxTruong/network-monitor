//! System tray module using ksni (StatusNotifierItem protocol)
//! Displays country flag icon with network info menu.

use crate::geo::GeoInfo;
use crate::icons::{get_flag, ICON_SIZE};
use ksni::{menu::{CheckmarkItem, StandardItem}, Icon, MenuItem, Tray};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Commands that can be sent from tray menu
#[derive(Debug, Clone)]
pub enum TrayCommand {
    Refresh,
    ToggleAutostart,
    CheckUpdate,
    RunUpdate,
    Quit,
}

/// Network monitor tray application
pub struct NetworkTray {
    /// Current geo-location info (shared with refresh task)
    geo_info: Arc<Mutex<Option<GeoInfo>>>,
    /// Channel to send commands to main loop
    command_tx: mpsc::Sender<TrayCommand>,
    /// Whether autostart is enabled
    pub autostart_enabled: bool,
    /// New version available (if any)
    pub update_available: Option<String>,
    /// Whether currently checking for updates
    pub checking_update: bool,
}

impl NetworkTray {
    pub fn new(
        geo_info: Arc<Mutex<Option<GeoInfo>>>,
        command_tx: mpsc::Sender<TrayCommand>,
        autostart_enabled: bool,
    ) -> Self {
        Self {
            geo_info,
            command_tx,
            autostart_enabled,
            update_available: None,
            checking_update: false,
        }
    }

    /// Updates the geo info (called from refresh task)
    #[allow(dead_code)]
    pub fn update_geo_info(&mut self, info: Option<GeoInfo>) {
        if let Ok(mut guard) = self.geo_info.lock() {
            *guard = info;
        }
    }

    /// Gets current country code for icon lookup
    fn current_country_code(&self) -> String {
        self.geo_info
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|i| i.country_code.clone()))
            .unwrap_or_else(|| "xx".to_string())
    }

    /// Gets display text for current location
    #[allow(dead_code)]
    fn location_text(&self) -> String {
        self.geo_info
            .lock()
            .ok()
            .and_then(|g| {
                g.as_ref().map(|i| {
                    format!("{}, {} ({})", i.city, i.country, i.country_code)
                })
            })
            .unwrap_or_else(|| "Unknown location".to_string())
    }
}

impl Tray for NetworkTray {
    fn id(&self) -> String {
        "network-monitor".to_string()
    }

    fn title(&self) -> String {
        "Network Monitor".to_string()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        let country_code = self.current_country_code();
        let flag = get_flag(&country_code);

        // Decode PNG to get RGBA pixels
        if let Ok(img) = image::load_from_memory(flag.data) {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();

            // Convert to ARGB format (ksni expects ARGB)
            let mut argb_data = Vec::with_capacity((width * height * 4) as usize);
            for pixel in rgba.pixels() {
                // ARGB order: Alpha, Red, Green, Blue
                argb_data.push(pixel[3]); // A
                argb_data.push(pixel[0]); // R
                argb_data.push(pixel[1]); // G
                argb_data.push(pixel[2]); // B
            }

            vec![Icon {
                width: width as i32,
                height: height as i32,
                data: argb_data,
            }]
        } else {
            // Fallback: empty icon (shouldn't happen)
            vec![Icon {
                width: ICON_SIZE as i32,
                height: ICON_SIZE as i32,
                data: vec![0; (ICON_SIZE * ICON_SIZE * 4) as usize],
            }]
        }
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let info = self.geo_info.lock().ok().and_then(|g| g.clone());

        let (title, description) = if let Some(geo) = info {
            (
                format!("{} ({})", geo.country, geo.country_code),
                format!("IP: {}\nCity: {}\nISP: {}", geo.query, geo.city, geo.isp),
            )
        } else {
            ("Network Monitor".to_string(), "Fetching location...".to_string())
        };

        ksni::ToolTip {
            title,
            description,
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let info = self.geo_info.lock().ok().and_then(|g| g.clone());

        let mut items: Vec<MenuItem<Self>> = Vec::new();

        // Network info items (non-clickable labels)
        if let Some(geo) = info {
            items.push(MenuItem::Standard(StandardItem {
                label: format!("IP: {}", geo.query),
                enabled: false,
                ..Default::default()
            }));
            items.push(MenuItem::Standard(StandardItem {
                label: format!("Country: {} ({})", geo.country, geo.country_code),
                enabled: false,
                ..Default::default()
            }));
            items.push(MenuItem::Standard(StandardItem {
                label: format!("City: {}", geo.city),
                enabled: false,
                ..Default::default()
            }));
            items.push(MenuItem::Standard(StandardItem {
                label: format!("ISP: {}", geo.isp),
                enabled: false,
                ..Default::default()
            }));
        } else {
            items.push(MenuItem::Standard(StandardItem {
                label: "Fetching location...".to_string(),
                enabled: false,
                ..Default::default()
            }));
        }

        // Separator
        items.push(MenuItem::Separator);

        // Actions
        let refresh_tx = self.command_tx.clone();
        items.push(MenuItem::Standard(StandardItem {
            label: "Refresh".to_string(),
            activate: Box::new(move |_| {
                let _ = refresh_tx.try_send(TrayCommand::Refresh);
            }),
            ..Default::default()
        }));

        let autostart_tx = self.command_tx.clone();
        items.push(MenuItem::Checkmark(CheckmarkItem {
            label: "Launch on Login".to_string(),
            checked: self.autostart_enabled,
            activate: Box::new(move |_| {
                let _ = autostart_tx.try_send(TrayCommand::ToggleAutostart);
            }),
            ..Default::default()
        }));

        // Update section
        items.push(MenuItem::Separator);

        if self.checking_update {
            // Show spinner while checking
            items.push(MenuItem::Standard(StandardItem {
                label: "⏳ Checking for updates...".to_string(),
                enabled: false,
                ..Default::default()
            }));
        } else if let Some(ref version) = self.update_available {
            // Show clickable update button
            items.push(MenuItem::Standard(StandardItem {
                label: format!("🔴 Update to {} (click to install)", version),
                activate: Box::new({
                    let tx = self.command_tx.clone();
                    move |_| {
                        let _ = tx.try_send(TrayCommand::RunUpdate);
                    }
                }),
                ..Default::default()
            }));
        } else {
            // Show check for updates option
            let update_tx = self.command_tx.clone();
            items.push(MenuItem::Standard(StandardItem {
                label: "Check for Updates".to_string(),
                activate: Box::new(move |_| {
                    let _ = update_tx.try_send(TrayCommand::CheckUpdate);
                }),
                ..Default::default()
            }));
        }

        items.push(MenuItem::Separator);

        let quit_tx = self.command_tx.clone();
        items.push(MenuItem::Standard(StandardItem {
            label: "Quit".to_string(),
            activate: Box::new(move |_| {
                let _ = quit_tx.try_send(TrayCommand::Quit);
            }),
            ..Default::default()
        }));

        items
    }
}
