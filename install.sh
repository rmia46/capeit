#!/bin/bash

# Capeit Installer Script
# Handles building, installing binaries, systemd service, and desktop entry.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Starting Capeit Installation...${NC}"

# 1. Check for Cargo
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found. Please install Rust and Cargo.${NC}"
    exit 1
fi

# 2. Build the project
echo -e "${BLUE}Building Capeit (Release mode)...${NC}"
cargo build --release

# 3. Stop existing service if running
if systemctl is-active --quiet capeitd; then
    echo -e "${BLUE}Stopping existing capeitd service...${NC}"
    sudo systemctl stop capeitd
fi

# 4. Install Binaries
echo -e "${BLUE}Installing binaries to /usr/local/bin/...${NC}"
sudo cp target/release/capeitd /usr/local/bin/
sudo cp target/release/capeit-gui /usr/local/bin/

# 5. Setup Systemd Service
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

# 6. Setup Desktop Entry
echo -e "${BLUE}Setting up desktop entry...${NC}"
if [ -f "capeit.desktop" ]; then
    sudo cp capeit.desktop /usr/share/applications/
    echo -e "${GREEN}Desktop entry installed.${NC}"
else
    echo -e "${RED}Warning: capeit.desktop file not found in current directory.${NC}"
fi

echo -e "${GREEN}Installation Complete!${NC}"
echo -e "You can now run ${BLUE}capeit-gui${NC} from your application menu or terminal."
