# Network Monitor

System tray app displaying country flag based on network location (geo-IP).

![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![License](https://img.shields.io/badge/License-MIT-blue)
![Platform](https://img.shields.io/badge/Platform-Linux%20(Wayland)-green)

## Features

- **Country Flag Icon** - Shows your current location's flag in system tray
- **Network Info** - Click to see: IP, country, city, ISP
- **Auto-refresh** - Updates every 1 minute + on network change
- **App Launcher** - Shows in Ubuntu "All Apps" menu
- **Autostart** - Starts automatically on login
- **Auto-update** - Checks for updates daily, notifies in tray menu
- **Wayland Native** - Uses StatusNotifierItem (SNI) protocol

## Requirements

- Linux with Wayland (tested on Ubuntu/GNOME)
- D-Bus (for NetworkManager integration)
- GNOME Shell: Install [AppIndicator extension](https://extensions.gnome.org/extension/615/)

## Installation

### One-line install

```bash
curl -sSL https://raw.githubusercontent.com/btxTruong/network-monitor/main/install.sh | bash
```

This will:
- Download the latest release
- Install to `~/.local/bin/`
- Add to Ubuntu app launcher (All Apps)
- Enable autostart on login
- Offer to start immediately

### Update

```bash
network-monitor --update
```

### Build from source

```bash
git clone https://github.com/btxTruong/network-monitor
cd network-monitor
cargo build --release
./target/release/network-monitor
```

## Usage

1. **Launch** - Find "Network Monitor" in apps menu, or run `network-monitor`
2. **Tray Icon** - Country flag appears in system tray
3. **Click Menu** - Shows IP, country, city, ISP
4. **Refresh** - Manual refresh button
5. **Autostart** - Toggle in menu (enabled by default after install)
6. **Update** - Run `network-monitor --update` or shows notification in tray when new version available
7. **Check** - Run `network-monitor --check` to check for updates
8. **Quit** - Exit application

## Architecture

```
src/
├── main.rs        # Entry point, event loop
├── tray.rs        # System tray (ksni)
├── geo.rs         # Geo-IP client (ip-api.com)
├── network.rs     # NetworkManager D-Bus
├── icons.rs       # Embedded flag icons
├── updater.rs     # Auto-update checker
└── autostart.rs   # XDG autostart
```

## Credits

- [circle-flags](https://github.com/HatScripts/circle-flags) - Country flag icons
- [ip-api.com](https://ip-api.com/) - Geo-IP service

## License

MIT License - See [LICENSE](LICENSE)
