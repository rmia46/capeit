# GMode GUI Application Plan

## 1. Objective
Create a native Linux GUI application to replace the `gmode` bash script. The application will manage CPU/GPU performance by offering distinct operational modes:
*   **Fixed Mode:** Hard locks for clock speeds and strict power (wattage) budgets.
*   **Thermal Mode:** Unlocks clocks and power, allowing hardware to boost dynamically until specific user-defined temperature targets are reached.
*   **Hybrid/Custom Mode:** Allows advanced mixing of power limits and thermal throttling targets.

## 2. Tech Stack Recommendation
*   **Language:** Python 3. (A wrapper around system commands doesn't benefit from C++ speeds, and Python allows rapid GUI development).
*   **GUI Framework:** `CustomTkinter` (Modern, dark-themed, straightforward) or `PySide6` (Qt for Python, highly robust).
*   **Permissions:** Polkit (`pkexec`) to prompt for a GUI password once, or running the backend daemon as root with a user-space frontend.

## 3. UI/UX Layout
*   **Dashboard Layout:** A single window divided into sections.
*   **Section A: Operational Mode Toggle**
    *   [ ] Fixed Performance
    *   [ ] Thermal Priority
    *   [ ] Custom Override
*   **Section B: Hardware Controls (Disabled/Enabled based on Mode)**
    *   **CPU Power Limits:** PL1 (Long) and PL2 (Short). Logic enforcement: PL2 $\ge$ PL1.
    *   **CPU Clock Cap:** Slider/Input for max MHz (`cpupower`).
    *   **GPU Clock Lock:** Slider/Input for fixed MHz (`nvidia-smi`).
*   **Section C: Thermal Targets**
    *   **CPU Max Temp:** Calculates and sets TCC Offset (100°C - Target = Offset).
    *   **GPU Target Temp:** Direct target application.

## 4. Backend Implementation (System Hooks)
The Python backend will utilize the `subprocess` module to execute the commands we verified during debugging:

### A. Power & CPU Thermals (via `intel-undervolt`)
The application will read, parse, and rewrite `/etc/intel-undervolt.conf`, then apply:
*   **Power:** `power package <PL2>:enabled <PL1>:enabled`
*   **Thermal:** `tjoffset -<Offset>`
*   **Apply:** `pkexec intel-undervolt apply`

### B. CPU Clocks (via `cpupower`)
*   **Set Max Cap:** `pkexec cpupower frequency-set -u <MHz>MHz`
*   **Reset/Unlock:** `pkexec cpupower frequency-set -u <Hardware_Max>MHz`

### C. GPU Clocks & Thermals (via `nvidia-smi`)
*   **Lock Clock:** `pkexec nvidia-smi --lock-gpu-clocks=<MHz>,<MHz>`
*   **Set Thermal Target:** `pkexec nvidia-smi -tt <Temp>`
*   **Reset:** `pkexec nvidia-smi --reset-gpu-clocks`

## 5. Next Steps for Implementation
1. Initialize the Python project and build the UI skeleton.
2. Create the backend class to safely manage `/etc/intel-undervolt.conf` parsing and writing.
3. Wire the UI toggles to the subprocess execution functions.
4. Add a status polling loop (every 2 seconds) to display current temps, clocks, and active limits in the GUI.