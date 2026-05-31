# Capeit Project Context (v0.2.3)

## Overview
**Capeit** is a high-performance, professional-grade hardware and thermal management suite for Linux. It provides a modular, secure, and visually sophisticated environment for monitoring and controlling system performance, built using **Rust** and the **Slint** GUI toolkit.

## Architecture
Capeit uses a **Daemon/Client** architecture to bridge secure hardware access with a responsive user interface:

### 1. `capeitd` (Daemon)
- **Role**: Root-level service (`systemd`) managing hardware interfaces.
- **Interfaces**: Communicates with `sysfs` (CPU/Thermal), `nvidia-smi` (GPU), and `intel-undervolt` (Power/TDP).
- **Safety**: Enforces hardcoded boundaries to prevent hardware damage.
- **Persistence**: Manages a global `config.toml` in the system or user config directory.

### 2. `capeit-gui` (Client)
- **Role**: A user-space GUI for visualization and control.
- **IPC**: Communicates with the daemon via **Unix Domain Sockets** (`/tmp/capeit.sock`) using JSON serialization.
- **Compatibility**: Uses `#[serde(default)]` in telemetry structures to maintain compatibility between different GUI/Daemon versions.

### 3. `capeit-common`
- **Role**: Shared library defining the IPC protocol (`Action` and `DaemonResponse` enums) and core data structures (`Telemetry`, `PowerProfile`, `ThermalProfile`).

## Design & Coding Paradigms

### 1. Modular Slint Architecture
The UI is split into focused files to maintain scalability:
- **`theme.slint`**: Defines global colors (Nordic theme), gradients, and UI structures.
- **`widgets.slint`**: Reusable components (e.g., `HubCard`, `MiniStat`, `TabBtn`, `ThemedSlider`).
- **`app.slint`**: Orchestrates the main window layout and category views.

### 2. Icon Integration
Icons are integrated using the `lucide-slint` library. The project uses a standardized component pattern:
- **`IconDisplay`**: A wrapper that accepts an `IconSet` enum value.
- **Usage**: `IconDisplay { icon: IconSet.Cpu, stroke: Theme.text_main, size: 16px; }`
- **Vertical Alignment**: Icons and text are strictly aligned using `VerticalLayout` or `VerticalBox` wrappers with `alignment: center`.

### 3. Performance-First Telemetry
- **No Graphs**: Heavy vector graphing (`tiny-skia`) was removed in v0.2.x to ensure near-zero GUI overhead.
- **Real-Time Sensors**: Telemetry is visualized via `MiniStat` bars (progress indicators) for Usage, Temperature, and Clock Speeds.
- **Throttling UI**: The `SYSTEM HEALTH` badge uses an **outline style** for "OPTIMAL" status and a **solid red** style for "THROTTLED" alerts.

### 4. System Info Caching
- **Efficiency**: Detailed hardware reports (`inxi`) are expensive to generate. The app caches this data in `~/.cache/capeit/sys_info.json`.
- **Logic**: On launch, the app attempts to load the cache. A background task updates the cache only if it's missing or if a refresh is requested, ensuring instant UI availability.

## Technical Stack
- **Languages**: Rust (1.75+)
- **UI Framework**: Slint (1.16)
- **IPC**: Unix Domain Sockets + Serde/JSON
- **Icons**: lucide-slint
- **CLI Helpers**: `inxi` (hardware specs), `nvidia-smi` (GPU), `intel-undervolt` (CPU Power).

## Directory Structure
- `capeit-gui/src/`:
    - `main.rs`: UI orchestration and IPC loop.
    - `telemetry.rs`: App state management.
    - `sys_info.rs`: Hardware detection and caching logic.
    - `utils.rs`: Formatting and profile helpers.
- `capeitd/src/`:
    - `main.rs`: Socket server and hardware control logic.
- `capeit-common/src/`:
    - `lib.rs`: Shared types and IPC definitions.
