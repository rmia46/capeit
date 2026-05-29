# Capeit

Capeit is a high-performance hardware and thermal control application for Linux, written in Rust with a Slint GUI. It replaces the legacy `gmode` bash script with a secure Daemon/Client architecture.

## Architecture
- **`capeitd`**: A root-level daemon that manages hardware interactions (via `cpupower`, `nvidia-smi`, and `intel-undervolt`). It enforces strict safety bounds to protect your hardware.
- **`capeit-gui`**: A lightweight Slint-based GUI that provides a real-time dashboard and control interface.
- **`capeit-common`**: Shared types and IPC logic using Unix Domain Sockets.

## Safety Net
The daemon enforces the following hard limits:
- CPU: 800MHz - 5000MHz
- GPU: 200MHz - 3500MHz
- Power: 10W - 150W
- Thermal: Maximum 20°C target offset

## Installation
1. Build the project: `cargo build --release`
2. Install the daemon: `sudo cp target/release/capeitd /usr/local/bin/`
3. Install the GUI: `sudo cp target/release/capeit-gui /usr/local/bin/`
4. Setup the service: `sudo cp capeitd.service /etc/systemd/system/`
5. Enable the service: `sudo systemctl enable --now capeitd`

## Future Features (Planned)
- Auto-Game Profile Detection
- AC Power Supply Sync
- Intel Hybrid Core (E-Core) Toggle
- System Fan Curve Triggers
