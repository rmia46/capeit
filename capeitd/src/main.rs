use anyhow::{Context, Result};
use capeit_common::{Action, Config, DaemonResponse, PowerProfile, ThermalProfile, Telemetry};
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use std::os::unix::fs::PermissionsExt;

const SOCKET_PATH: &str = "/tmp/capeit.sock";

static CPU_NAME: Lazy<String> = Lazy::new(|| {
    std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default().lines()
        .find(|l| l.starts_with("model name"))
        .map(|l| l.split(':').nth(1).unwrap_or("Unknown").trim().to_string()).unwrap_or("Unknown".into())
});

static KERNEL: Lazy<String> = Lazy::new(|| {
    std::fs::read_to_string("/proc/version").map(|s| s.split_whitespace().nth(2).unwrap_or("Unknown").to_string()).unwrap_or("Unknown".into())
});

struct CpuSample { total: u64, idle: u64 }

struct AppState {
    telemetry: Telemetry,
    last_cpu_sample: CpuSample,
    config: Config,
    active_power: String,
    active_thermal: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = std::fs::remove_file(SOCKET_PATH);
    let config = load_config().unwrap_or_default();
    
    let mut telemetry = Telemetry { cpu_name: CPU_NAME.clone(), kernel: KERNEL.clone(), ..Telemetry::default() };
    if let Some(p) = config.power_profiles.get("balanced") {
        telemetry.active_cpu_limit = p.cpu_max_mhz;
        telemetry.active_gpu_limit = p.gpu_lock_mhz;
        telemetry.active_p_short = p.p_short_w;
        telemetry.active_p_long = p.p_long_w;
    }
    if let Some(t) = config.thermal_profiles.get("Normal") {
        telemetry.active_thermal_limit = t.cpu_tj_offset as i32;
        telemetry.active_gpu_temp_limit = t.gpu_temp_limit;
    }

    let state = Arc::new(Mutex::new(AppState {
        telemetry,
        last_cpu_sample: get_cpu_sample().unwrap_or(CpuSample { total: 0, idle: 0 }),
        config,
        active_power: "balanced".into(),
        active_thermal: "Normal".into(),
    }));

    let t_state = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            let _ = update_telemetry(&t_state).await;
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    });

    let listener = UnixListener::bind(SOCKET_PATH)?;
    if let Ok(m) = std::fs::metadata(SOCKET_PATH) {
        let mut p = m.permissions();
        p.set_mode(0o666);
        let _ = std::fs::set_permissions(SOCKET_PATH, p);
    }

    loop {
        let (s, _) = listener.accept().await?;
        let h = Arc::clone(&state);
        tokio::spawn(async move { let _ = handle_client(s, h).await; });
    }
}

fn get_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut p| { p.push("capeit"); p.push("config.toml"); p })
}

fn is_system_profile(name: &str) -> bool {
    matches!(name, "powersave" | "balanced" | "gmode-lite" | "gmode-max" | "stock")
}

fn load_config() -> Result<Config> {
    let path = get_config_path().context("No config path")?;
    let mut config = if path.exists() {
        toml::from_str(&std::fs::read_to_string(&path)?)?
    } else {
        Config::default()
    };

    let mut changed = false;
    let system_defaults = [
        ("powersave", "Maximum battery life.", 1300, 210, 25, 25),
        ("balanced", "Standard daily use.", 2200, 2010, 35, 30),
        ("gmode-lite", "Light gaming profile.", 2000, 1905, 35, 30),
        ("gmode-max", "Maximum performance.", 2800, 2400, 45, 40),
        ("stock", "Factory defaults (no limits).", 5000, 2500, 200, 200),
    ];

    for (n, d, c, g, ps, pl) in system_defaults {
        if !config.power_profiles.contains_key(n) {
            config.power_profiles.insert(n.into(), PowerProfile { 
                name: n.into(), description: d.into(), cpu_max_mhz: c, gpu_lock_mhz: g, p_short_w: ps, p_long_w: pl 
            });
            changed = true;
        }
    }

    if !config.thermal_profiles.contains_key("Normal") {
        config.thermal_profiles.insert("Normal".into(), ThermalProfile { name: "Normal".into(), description: "Standard thermal limits.".into(), cpu_tj_offset: 10, gpu_temp_limit: 80 });
        changed = true;
    }
    if !config.thermal_profiles.contains_key("None") {
        config.thermal_profiles.insert("None".into(), ThermalProfile { name: "None".into(), description: "Unrestricted thermals.".into(), cpu_tj_offset: 0, gpu_temp_limit: 90 });
        changed = true;
    }

    if changed || !path.exists() { save_config(&config)?; }
    Ok(config)
}

fn save_config(config: &Config) -> Result<()> {
    let path = get_config_path().context("No config path")?;
    if let Some(p) = path.parent() { std::fs::create_dir_all(p)?; }
    std::fs::write(path, toml::to_string(config)?)?;
    Ok(())
}

async fn update_telemetry(state: &Arc<Mutex<AppState>>) -> Result<()> {
    let mut s = state.lock().await;
    if let Ok(new) = get_cpu_sample() {
        let t_diff = new.total - s.last_cpu_sample.total;
        let i_diff = new.idle - s.last_cpu_sample.idle;
        if t_diff > 0 { s.telemetry.cpu_usage = 100.0 * (1.0 - (i_diff as f32 / t_diff as f32)); }
        s.last_cpu_sample = new;
    }
    s.telemetry.cpu_temp = find_cpu_temp().unwrap_or(0.0);
    s.telemetry.cpu_clock_mhz = read_sysfs_u32("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq").unwrap_or(0) / 1000;
    
    if let Ok(out) = std::process::Command::new("nvidia-smi").args(["--query-gpu=name,temperature.gpu,clocks.sm,utilization.gpu,memory.used,memory.total", "--format=csv,noheader,nounits"]).output() {
        let out_raw = String::from_utf8_lossy(&out.stdout);
        for line in out_raw.lines() {
            let p: Vec<&str> = line.split(',').map(|x| x.trim()).collect();
            if p.len() >= 6 {
                s.telemetry.gpu_name = p[0].to_string();
                s.telemetry.gpu_temp = p[1].parse().unwrap_or(0.0);
                s.telemetry.gpu_clock_mhz = p[2].parse().unwrap_or(0);
                s.telemetry.gpu_usage = p[3].parse().unwrap_or(0.0);
                s.telemetry.vram_used_mb = p[4].parse().unwrap_or(0);
                s.telemetry.vram_total_mb = p[5].parse().unwrap_or(0);
                break;
            }
        }
    }
    
    if let Ok(m) = std::fs::read_to_string("/proc/meminfo") {
        let mut tot = 0.0; let mut av = 0.0;
        for l in m.lines() {
            if l.starts_with("MemTotal:") { tot = l.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0.0); }
            if l.starts_with("MemAvailable:") { av = l.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0.0); }
        }
        s.telemetry.ram_total_gb = tot / 1024.0 / 1024.0;
        s.telemetry.ram_used_gb = (tot - av) / 1024.0 / 1024.0;
    }

    if let Ok(u) = std::fs::read_to_string("/proc/uptime") {
        s.telemetry.uptime_secs = u.split_whitespace().next().unwrap_or("0").parse::<f32>().unwrap_or(0.0) as u64;
    }

    s.telemetry.ac_connected = std::fs::read_to_string("/sys/class/power_supply/AC/online").map(|x| x.trim() == "1").unwrap_or(true);
    s.telemetry.active_power_profile = s.active_power.clone();
    s.telemetry.active_thermal_profile = s.active_thermal.clone();
    s.telemetry.is_throttled = s.telemetry.cpu_temp > 95.0 || s.telemetry.gpu_temp > 87.0;
    Ok(())
}

fn get_cpu_sample() -> Result<CpuSample> {
    let s = std::fs::read_to_string("/proc/stat")?;
    let first = s.lines().next().context("No stat")?;
    let p: Vec<u64> = first.split_whitespace().skip(1).map(|x| x.parse().unwrap_or(0)).collect();
    if p.len() < 4 { anyhow::bail!("Invalid stat") }
    Ok(CpuSample { total: p.iter().sum(), idle: p[3] })
}

fn find_cpu_temp() -> Option<f32> {
    for i in 0..10 {
        let tp = format!("/sys/class/thermal/thermal_zone{}/type", i);
        if let Ok(t) = std::fs::read_to_string(&tp) {
            let tr = t.trim();
            if tr == "x86_pkg_temp" || tr == "acpitz" || tr == "TCPU" {
                if let Ok(ts) = std::fs::read_to_string(format!("/sys/class/thermal/thermal_zone{}/temp", i)) {
                    return ts.trim().parse::<f32>().ok().map(|v| v / 1000.0);
                }
            }
        }
    }
    None
}

fn read_sysfs_u32(p: &str) -> Result<u32> { Ok(std::fs::read_to_string(p)?.trim().parse()?) }

async fn handle_client(mut stream: UnixStream, state: Arc<Mutex<AppState>>) -> Result<()> {
    let mut buf = [0u8; 16384];
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 { return Ok(()); }
        let action: Action = serde_json::from_slice(&buf[..n])?;
        let resp = match action {
            Action::GetTelemetry => DaemonResponse::Telemetry(state.lock().await.telemetry.clone()),
            Action::GetConfig => DaemonResponse::Config(state.lock().await.config.clone()),
            Action::SavePowerProfile(p) => {
                let mut s = state.lock().await;
                s.config.power_profiles.insert(p.name.clone(), p);
                let _ = save_config(&s.config);
                DaemonResponse::Ok
            }
            Action::SaveThermalProfile(p) => {
                let mut s = state.lock().await;
                s.config.thermal_profiles.insert(p.name.clone(), p);
                let _ = save_config(&s.config);
                DaemonResponse::Ok
            }
            Action::DeletePowerProfile(n) => {
                if is_system_profile(&n) {
                    DaemonResponse::Error("Cannot delete system profile".into())
                } else {
                    let mut s = state.lock().await;
                    s.config.power_profiles.remove(&n);
                    let _ = save_config(&s.config);
                    DaemonResponse::Ok
                }
            }
            Action::DeleteThermalProfile(n) => {
                if n == "Normal" || n == "None" {
                    DaemonResponse::Error("Cannot delete system profile".into())
                } else {
                    let mut s = state.lock().await;
                    s.config.thermal_profiles.remove(&n);
                    let _ = save_config(&s.config);
                    DaemonResponse::Ok
                }
            }
            Action::ApplyPowerProfile(n) => {
                let mut s = state.lock().await;
                if let Some(p) = s.config.power_profiles.get(&n).cloned() {
                    let _ = apply_cpu(p.cpu_max_mhz);
                    let _ = apply_gpu(p.gpu_lock_mhz);
                    let _ = apply_power(p.p_short_w, p.p_long_w);
                    s.active_power = n;
                    s.telemetry.active_cpu_limit = p.cpu_max_mhz;
                    s.telemetry.active_gpu_limit = p.gpu_lock_mhz;
                    s.telemetry.active_p_short = p.p_short_w;
                    s.telemetry.active_p_long = p.p_long_w;
                    DaemonResponse::Ok
                } else { DaemonResponse::Error("Not found".into()) }
            }
            Action::ApplyThermalProfile(n) => {
                let mut s = state.lock().await;
                if let Some(p) = s.config.thermal_profiles.get(&n).cloned() {
                    let _ = apply_thermal(p.cpu_tj_offset);
                    let _ = apply_gpu_temp(p.gpu_temp_limit);
                    s.active_thermal = n;
                    s.telemetry.active_thermal_limit = p.cpu_tj_offset as i32;
                    s.telemetry.active_gpu_temp_limit = p.gpu_temp_limit;
                    DaemonResponse::Ok
                } else { DaemonResponse::Error("Not found".into()) }
            }
            Action::UpdateAdvanced(adv) => {
                let mut s = state.lock().await;
                s.config.advanced_settings = adv;
                let _ = save_config(&s.config);
                DaemonResponse::Ok
            }
            Action::SetCpuMaxClock(m) => { 
                let _ = apply_cpu(m); 
                state.lock().await.telemetry.active_cpu_limit = m;
                DaemonResponse::Ok 
            }
            Action::SetGpuLockedClock(m) => { 
                let _ = apply_gpu(m); 
                state.lock().await.telemetry.active_gpu_limit = m;
                DaemonResponse::Ok 
            }
            Action::SetPowerLimits { short, long } => { 
                let _ = apply_power(short, long); 
                let mut s = state.lock().await;
                s.telemetry.active_p_short = short;
                s.telemetry.active_p_long = long;
                DaemonResponse::Ok 
            }
            Action::SetThermalTarget(o) => { 
                let _ = apply_thermal(o); 
                state.lock().await.telemetry.active_thermal_limit = o as i32;
                DaemonResponse::Ok 
            }
            Action::SetGpuTempLimit(l) => { 
                let _ = apply_gpu_temp(l); 
                state.lock().await.telemetry.active_gpu_temp_limit = l;
                DaemonResponse::Ok 
            }
            Action::ToggleECores(_) => DaemonResponse::Error("Not implemented".into()),
        };
        let _ = stream.write_all(&serde_json::to_vec(&resp)?).await;
    }
}

fn apply_cpu(mhz: u32) -> Result<()> { 
    std::process::Command::new("cpupower").args(["frequency-set", "-u", &format!("{}MHz", mhz)]).status()?; Ok(()) 
}
fn apply_gpu(mhz: u32) -> Result<()> { 
    std::process::Command::new("nvidia-smi").args(["--lock-gpu-clocks", &format!("210,{}", mhz)]).status()?; Ok(()) 
}
fn apply_gpu_temp(lim: u32) -> Result<()> {
    std::process::Command::new("nvidia-smi").args(["-gpu-target-temp", &format!("{}", lim)]).status()?; Ok(())
}
fn apply_power(s: u32, l: u32) -> Result<()> {
    let cp = "/etc/intel-undervolt.conf";
    if let Ok(c) = std::fs::read_to_string(cp) {
        let mut lines: Vec<String> = c.lines().map(|x| x.to_string()).collect();
        let mut f = false;
        for line in lines.iter_mut() {
            if line.starts_with("power package") { *line = format!("power package {}:enabled {}:enabled", s, l); f = true; }
        }
        if !f { lines.push(format!("power package {}:enabled {}:enabled", s, l)); }
        std::fs::write(cp, lines.join("\n"))?;
        std::process::Command::new("intel-undervolt").arg("apply").status()?;
    }
    Ok(())
}
fn apply_thermal(o: u32) -> Result<()> {
    let cp = "/etc/intel-undervolt.conf";
    if let Ok(c) = std::fs::read_to_string(cp) {
        let mut lines: Vec<String> = c.lines().map(|x| x.to_string()).collect();
        let mut f = false;
        for line in lines.iter_mut() {
            if line.starts_with("tjoffset") { *line = format!("tjoffset -{}", o); f = true; }
        }
        if !f { lines.push(format!("tjoffset -{}", o)); }
        std::fs::write(cp, lines.join("\n"))?;
        std::process::Command::new("intel-undervolt").arg("apply").status()?;
    }
    Ok(())
}
