# CapeIt

**CapeIt** is a high-performance, professional-grade hardware and thermal management suite for Linux. It provides a modular, secure, and visually sophisticated environment for monitoring and controlling system performance, built using **Rust** and the **Slint** GUI toolkit.

It serves as a modern, high-fidelity replacement for the legacy `gmode` bash script, utilizing a secure Daemon/Client architecture.

## 🚀 Key Features
- **Real-Time Telemetry**: Monitor CPU and GPU usage, temperatures, and clock speeds with zero GUI overhead.
- **Profile Management**: Switch between built-in system profiles (`powersave`, `balanced`, `max`) or create your own custom performance templates.
- **Dynamic GPU Scaling**: Automatically allows GPUs to downclock when idle while strictly enforcing maximum frequency caps under load.
- **Hardware Overrides**: Manual control over CPU Max Clock, Thermal TJ Offsets, and GPU Temperature limits.
- **Modern UI**: A sleek Nordic-themed interface with Lucide icons, smooth animations, and a responsive layout.
- **System Caching**: High-performance hardware reporting with intelligent disk-based caching for near-instant load times.

## 🏗️ Architecture
- **`capeitd` (Daemon)**: A root-level systemd service that manages direct hardware interaction. It enforces safety boundaries and manages persistent configurations.
- **`capeit-gui` (Client)**: A lightweight user-space GUI that communicates with the daemon via Unix Domain Sockets (`/tmp/capeit.sock`).
- **`capeit-common`**: Shared IPC protocol and data structures.

## 🛠️ System Dependencies
The backend daemon and hardware reporting require the following tools to be installed on your system:
- **`cpupower`**: For CPU frequency management.
- **`nvidia-smi`**: For NVIDIA GPU telemetry and clock/temp control.
- **`intel-undervolt`**: For CPU power limits (PL1/PL2) and thermal offsets.
- **`inxi`**: For detailed hardware specification reporting.
- **`systemd`**: To manage the background daemon service.

## 📦 Installation

The easiest way to install CapeIt is using the provided installation script:

1. **Clone the repository**:
   ```bash
   git clone https://github.com/rmia46/capeit.git
   cd capeit
   ```

2. **Run the installer**:
   ```bash
   chmod +x install.sh
   ./install.sh
   ```
   *Note: The script will build the project in release mode, install binaries to `/usr/local/bin/`, set up the `capeitd.service`, and install the desktop entry.*

3. **Manual Build** (Optional):
   ```bash
   cargo build --release
   ```

## 🛡️ Safety Net
The daemon enforces strict hardcoded boundaries to protect your hardware from accidental damage:
- **CPU Clock**: 800MHz - 5000MHz
- **GPU Clock**: 200MHz - 3500MHz
- **Power Limits**: 10W - 150W
- **Thermal Offset**: Maximum 20°C offset from TJMax.

## 📄 License
This project is licensed under the MIT License.
