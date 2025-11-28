#!/bin/bash
# Network Monitor - Install/Update script
# Usage:
#   Install: curl -sSL https://raw.githubusercontent.com/btxTruong/network-monitor/main/install.sh | bash
#   Update:  network-monitor --update  OR  curl -sSL ... | bash -s -- --update
#   Check:   curl -sSL ... | bash -s -- --check

set -e

REPO="btxTruong/network-monitor"
INSTALL_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/512x512/apps"
CONFIG_DIR="${HOME}/.config/network-monitor"
AUTOSTART_DIR="${HOME}/.config/autostart"
BINARY_NAME="network-monitor"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

print_status() { echo -e "${GREEN}▶${NC} $1"; }
print_warn() { echo -e "${YELLOW}▶${NC} $1"; }
print_error() { echo -e "${RED}▶${NC} $1"; }
print_info() { echo -e "${CYAN}▶${NC} $1"; }

# Parse arguments
UPDATE_ONLY=false
CHECK_ONLY=false
for arg in "$@"; do
    case $arg in
        --update|-u)
            UPDATE_ONLY=true
            ;;
        --check|-c)
            CHECK_ONLY=true
            ;;
    esac
done

# Get latest release info
print_status "Fetching latest release info..."
RELEASE_INFO=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null) || {
    print_error "Could not connect to GitHub"
    exit 1
}
LATEST=$(echo "$RELEASE_INFO" | grep '"tag_name"' | cut -d'"' -f4)

if [ -z "$LATEST" ]; then
    print_error "Could not fetch latest release. Check if releases exist."
    exit 1
fi

# Check current version
CURRENT=""
if [ -f "${CONFIG_DIR}/version" ]; then
    CURRENT=$(cat "${CONFIG_DIR}/version")
fi

# Check only mode
if [ "$CHECK_ONLY" = true ]; then
    echo ""
    print_info "Current version: ${CURRENT:-not installed}"
    print_info "Latest version:  ${LATEST}"
    if [ "$CURRENT" = "$LATEST" ]; then
        print_status "You're up to date!"
    elif [ -z "$CURRENT" ]; then
        print_warn "Not installed. Run without --check to install."
    else
        print_warn "Update available! Run: network-monitor --update"
    fi
    exit 0
fi

# Update mode
if [ "$UPDATE_ONLY" = true ]; then
    if [ -z "$CURRENT" ]; then
        print_error "Network Monitor is not installed. Run without --update to install."
        exit 1
    fi
    if [ "$CURRENT" = "$LATEST" ]; then
        print_status "Already on latest version: ${LATEST}"
        exit 0
    fi
    print_status "Updating from ${CURRENT} to ${LATEST}..."
else
    if [ -n "$CURRENT" ]; then
        print_info "Current version: ${CURRENT}"
    fi
    print_status "Installing Network Monitor ${LATEST}..."
fi

# Download binary
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${LATEST}/network-monitor-linux-x86_64.tar.gz"
TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

print_status "Downloading..."
curl -sSL "$DOWNLOAD_URL" -o release.tar.gz || {
    print_error "Download failed. Check if release ${LATEST} exists."
    rm -rf "$TMP_DIR"
    exit 1
}
tar -xzf release.tar.gz

# Create directories
mkdir -p "$INSTALL_DIR" "$APP_DIR" "$ICON_DIR" "$CONFIG_DIR" "$AUTOSTART_DIR"

# Install binary
print_status "Installing binary..."
mv "$BINARY_NAME" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# Save version
echo "$LATEST" > "${CONFIG_DIR}/version"
date +%s > "${CONFIG_DIR}/last-check"


# Download app icon
print_status "Setting up application icon..."
curl -sSL "https://raw.githubusercontent.com/btxTruong/network-monitor/main/assets/app-icon.png" -o "$ICON_DIR/network-monitor.png" 2>/dev/null && {
    ICON_NAME="network-monitor"
    # Update icon cache
    if command -v gtk-update-icon-cache &> /dev/null; then
        gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true
    fi
} || {
    ICON_NAME="network-workgroup"
    print_warn "Using system icon (custom icon download failed)"
}

# Create .desktop file for app launcher
print_status "Creating application entry..."
cat > "$APP_DIR/network-monitor.desktop" << EOF
[Desktop Entry]
Type=Application
Name=Network Monitor
Comment=Display network location country flag in system tray
Exec=${INSTALL_DIR}/${BINARY_NAME}
Icon=${ICON_NAME}
Terminal=false
Categories=Network;System;Monitor;
Keywords=network;ip;location;vpn;flag;tray;
StartupNotify=false
EOF

# Create autostart entry
print_status "Enabling autostart..."
cat > "$AUTOSTART_DIR/network-monitor.desktop" << EOF
[Desktop Entry]
Type=Application
Name=Network Monitor
Comment=Display network location country flag in system tray
Exec=${INSTALL_DIR}/${BINARY_NAME}
Icon=${ICON_NAME}
Terminal=false
Categories=Network;System;Monitor;
X-GNOME-Autostart-enabled=true
StartupNotify=false
EOF

# Update desktop database
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$APP_DIR" 2>/dev/null || true
fi

# Cleanup
cd - > /dev/null
rm -rf "$TMP_DIR"

# Check PATH
if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    echo ""
    print_warn "Add to ~/.bashrc or ~/.zshrc:"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

echo ""
print_status "Installation complete!"
echo ""
echo "  Version:    ${LATEST}"
echo "  Binary:     ${INSTALL_DIR}/${BINARY_NAME}"
echo "  Autostart:  Enabled"
echo ""
echo "  Commands:"
echo "    network-monitor           # Run app"
echo "    network-monitor --update  # Update to latest"
echo "    network-monitor --check   # Check for updates"
echo ""
echo "  Find 'Network Monitor' in your applications menu!"
echo ""

# Ask to start now (only if not piped)
if [ -t 0 ]; then
    read -p "Start Network Monitor now? [Y/n] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]] || [[ -z $REPLY ]]; then
        print_status "Starting Network Monitor..."
        nohup "$INSTALL_DIR/$BINARY_NAME" > /dev/null 2>&1 &
        disown
        print_status "Running! Check your system tray."
    fi
else
    print_info "Run 'network-monitor' to start the app."
fi
