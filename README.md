# CapeIt

CapeIt is a simple hobby project that provides a graphical interface for common Linux terminal commands used to control CPU and GPU power. It acts as a wrapper around tools like cpupower and nvidia-smi, making it easier to manage your hardware limits without typing commands every time.

## Features
- View real-time CPU and GPU usage, temperature, and clock speeds.
- Switch between different power profiles (like powersave or performance).
- Manually set limits for CPU clock speeds, GPU clocks, and temperature targets.
- View basic system and hardware information in one place.

## How it works
The project is split into two small parts:
1. **capeitd**: A background service (daemon) that runs as root to talk to your hardware.
2. **capeit-gui**: The actual window you see and interact with. It sends your settings to the daemon.

## System Requirements
Since this is just a wrapper, you need to have these tools installed on your system for everything to work:
- **cpupower**: To control CPU frequencies.
- **nvidia-smi**: For NVIDIA GPU monitoring and control.
- **intel-undervolt**: For setting CPU power limits and thermal offsets.
- **inxi**: To gather the hardware details shown in the info tab.
- **systemd**: To manage the background service.

## Installation

The easiest way to get it running is to use the included script:

1. Clone the repository:
   ```bash
   git clone https://github.com/rmia46/capeit.git
   cd capeit
   ```

2. Run the installer:
   ```bash
   chmod +x install.sh
   ./install.sh
   ```
   This script builds the project, moves the files to /usr/local/bin/, and sets up the background service for you.

## Safety
Even though this is a simple tool, the background service has some hardcoded safety limits to help prevent accidental hardware damage:
- CPU Clock: 800MHz to 5000MHz
- GPU Clock: 200MHz to 3500MHz
- Power Limits: 10W to 150W
- Thermal Offset: Maximum 20°C offset

## License
This project is shared under the MIT License.
