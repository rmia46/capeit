use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Telemetry {
    pub cpu_name: String,
    pub gpu_name: String,
    pub cpu_temp: f32,
    pub gpu_temp: f32,
    pub cpu_clock_mhz: u32,
    pub gpu_clock_mhz: u32,
    pub cpu_usage: f32,
    pub gpu_usage: f32,
    pub vram_used_mb: u32,
    pub vram_total_mb: u32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub uptime_secs: u64,
    pub kernel: String,
    pub ac_connected: bool,
    pub active_power_profile: String,
    pub active_thermal_profile: String,
    pub is_throttled: bool,
    // Active Limits
    pub active_cpu_limit: u32,
    pub active_gpu_limit: u32,
    pub active_thermal_limit: i32,
    pub active_gpu_temp_limit: u32,
    pub active_p_short: u32,
    pub active_p_long: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PowerProfile {
    pub name: String,
    pub description: String,
    pub cpu_max_mhz: u32,
    pub gpu_lock_mhz: u32,
    pub p_short_w: u32,
    pub p_long_w: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalProfile {
    pub name: String,
    pub description: String,
    pub cpu_tj_offset: u32,
    pub gpu_temp_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdvancedSettings {
    pub e_cores_enabled: bool,
    pub fan_boost: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub power_profiles: HashMap<String, PowerProfile>,
    pub thermal_profiles: HashMap<String, ThermalProfile>,
    pub advanced_settings: AdvancedSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    GetTelemetry,
    GetConfig,
    SavePowerProfile(PowerProfile),
    SaveThermalProfile(ThermalProfile),
    DeletePowerProfile(String),
    DeleteThermalProfile(String),
    ApplyPowerProfile(String),
    ApplyThermalProfile(String),
    UpdateAdvanced(AdvancedSettings),
    SetCpuMaxClock(u32),
    SetGpuLockedClock(u32),
    SetPowerLimits { short: u32, long: u32 },
    SetThermalTarget(u32),
    SetGpuTempLimit(u32),
    ToggleECores(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonResponse {
    Telemetry(Telemetry),
    Config(Config),
    Ok,
    Error(String),
}
