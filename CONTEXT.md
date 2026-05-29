# Capeit Project Context

## Overview
**Capeit** is a high-performance, professional-grade hardware and thermal management suite for Linux. It is designed to replace the legacy `gmode` bash script with a secure, responsive, and visually sophisticated application built using **Rust** and the **Slint** GUI toolkit.

## Architecture
Capeit follows a **Daemon/Client** architecture to ensure secure hardware access and persistent background monitoring:

- **`capeitd` (Daemon)**: A root-level service that communicates directly with hardware via `sysfs`, `cpupower`, `nvidia-smi`, and `intel-undervolt`. It enforces strict safety bounds and manages the system-wide configuration.
- **`capeit-gui` (Client)**: A high-fidelity user-space application that provides real-time visualization and control. It communicates with the daemon via Unix Domain Sockets (`/tmp/capeit.sock`).
- **`capeit-common`**: A shared library containing IPC protocols and data structures used by both the daemon and the GUI.

## Implemented Features

### 1. High-Fidelity Dashboard
- **Segmented Hubs**: Dedicated, fluid-responsive cards for CPU Core and GPU Graphics metrics.
- **Real-Time Graphing**: High-resolution, anti-aliased vector charts rendered via `tiny_skia`. Each hub displays a 60-second history overlaying Usage, Temperature, and Clock Speeds.
- **Responsive Layout**: A modern Nordic dark theme that scales perfectly across different screen sizes and high-DPI displays.
- **Hardware Identifier**: Dynamic detection of CPU and GPU models, including live monitoring of active hardware limits (MHz and Thermal offsets).

### 2. Advanced Profile Management
- **Persistent Storage**: All settings are stored in `~/.config/capeit/config.toml` using the TOML format.
- **System Profiles**: Built-in templates including `powersave`, `balanced`, `gmode-lite`, `gmode-max`, and a `stock` profile for factory defaults.
- **Custom Templates**: Users can create, edit, and delete their own performance profiles through a dedicated modal dialog with input validation.
- **Thermal Profiles**: Independent thermal management templates (e.g., `Normal`, `None`) to control throttling behavior.

### 3. Manual Hardware Overrides
- **Zero-Jitter Sliders**: Stabilized manual controls for CPU Max Clock, Thermal TJ Offset, and GPU Temperature limits.
- **Immediate Application**: One-click "Set" buttons that trigger atomic hardware updates via the daemon.

### 4. User Experience (UX)
- **Toast Notifications**: Sleek floating notifications at the bottom of the screen providing immediate confirmation or error feedback for every action.
- **Collapsible Sidebar**: A space-saving navigation menu for switching between Dashboard, Power Manager, and System Information.
- **Safety Net**: Hardcoded daemon-level boundaries to prevent accidental hardware damage from extreme values.

## Technical Stack
- **Languages**: Rust (1.75+)
- **UI Framework**: Slint (1.9)
- **Graphics Engine**: tiny-skia (0.11) for vector rendering
- **IPC**: Unix Domain Sockets with Serde/JSON serialization
- **Configuration**: TOML
- **External Dependencies**: `cpupower`, `nvidia-smi`, `intel-undervolt`

## Installation & Deployment
- **Installer**: A comprehensive `install.sh` script automates the build, binary deployment, systemd service setup, and desktop entry creation.
- **Persistence**: Managed via `capeitd.service` for automatic boot-start and hardware limit enforcement.
