#!/bin/bash

# CapeIt Installer Script
# Handles building, installing binaries, systemd service, icons, and desktop entry.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}Starting CapeIt Installation...${NC}"

# 1. Dependency Checks
dependencies=("cpupower" "nvidia-smi" "intel-undervolt" "inxi")
missing_deps=()

echo -e "${BLUE}Checking dependencies...${NC}"
for dep in "${dependencies[@]}"; do
    if ! command -v "$dep" &> /dev/null; then
        missing_deps+=("$dep")
    fi
done

if [ ${#missing_deps[@]} -ne 0 ]; then
    echo -e "${YELLOW}Warning: The following dependencies are missing:${NC}"
    for dep in "${missing_deps[@]}"; do
        echo -e "  - ${RED}$dep${NC}"
    done
    echo -e "${YELLOW}CapeIt requires these tools to control your hardware.${NC}"
    read -p "Do you want to continue anyway? (y/N): " choice
    if [[ ! "$choice" =~ ^[Yy]$ ]]; then
        echo -e "${RED}Installation cancelled. Please install missing tools and try again.${NC}"
        exit 1
    fi
fi

# 2. Check for Cargo
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found. Please install Rust and Cargo.${NC}"
    exit 1
fi

# 3. Build the project
echo -e "${BLUE}Building CapeIt (Release mode)...${NC}"
cargo build --release

# 4. Stop existing service if running
if systemctl is-active --quiet capeitd; then
    echo -e "${BLUE}Stopping existing capeitd service...${NC}"
    sudo systemctl stop capeitd
fi

# 5. Install Binaries
echo -e "${BLUE}Installing binaries to /usr/local/bin/...${NC}"
sudo cp target/release/capeitd /usr/local/bin/
sudo cp target/release/capeit-gui /usr/local/bin/

# 6. Install Icon
echo -e "${BLUE}Installing application icon...${NC}"
ICON_DIR="/usr/share/icons/hicolor/scalable/apps"
sudo mkdir -p "$ICON_DIR"
sudo cp capeit-gui/src/logo.svg "$ICON_DIR/capeit.svg"
sudo gtk-update-icon-cache /usr/share/icons/hicolor 2>/dev/null || true

# 7. Setup Systemd Service
echo -e "${BLUE}Setting up systemd service...${NC}"
if [ -f "capeitd.service" ]; then
    sudo cp capeitd.service /etc/systemd/system/
    sudo systemctl daemon-reload
    sudo systemctl enable capeitd
    sudo systemctl start capeitd
    echo -e "${GREEN}Daemon started successfully.${NC}"
else
    echo -e "${RED}Warning: capeitd.service file not found in current directory.${NC}"
fi

# 8. Setup Desktop Entry
echo -e "${BLUE}Setting up desktop entry...${NC}"
if [ -f "capeit.desktop" ]; then
    sudo cp capeit.desktop /usr/share/applications/
    echo -e "${GREEN}Desktop entry installed.${NC}"
else
    echo -e "${RED}Warning: capeit.desktop file not found in current directory.${NC}"
fi

echo -e "${GREEN}Installation Complete!${NC}"
echo -e "You can now run ${BLUE}CapeIt${NC} from your application menu or terminal."
