slint::include_modules!();
use anyhow::{Context, Result};
use capeit_common::{Action, DaemonResponse, PowerProfile};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use slint::{VecModel, Image, Rgba8Pixel, SharedPixelBuffer, ComponentHandle};
use std::rc::Rc;
use std::time::Duration;
use tiny_skia::*;

const SOCKET_PATH: &str = "/tmp/capeit.sock";
const HISTORY_SIZE: usize = 60;

struct Series {
    data: Vec<f32>, // Normalized 0.0 - 1.0
    color: [u8; 3],
}

struct AppState {
    cpu_usage_hist: Vec<f32>,
    cpu_temp_hist: Vec<f32>,
    cpu_clock_hist: Vec<f32>,
    gpu_usage_hist: Vec<f32>,
    gpu_temp_hist: Vec<f32>,
    gpu_clock_hist: Vec<f32>,
    first_telemetry: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let ui = AppWindow::new().context("Failed to create UI")?;
    let ui_handle = ui.as_weak();

    let (tx, mut rx) = mpsc::channel::<Action>(32);
    let mut state = AppState {
        cpu_usage_hist: vec![0.0; HISTORY_SIZE],
        cpu_temp_hist: vec![0.0; HISTORY_SIZE],
        cpu_clock_hist: vec![0.0; HISTORY_SIZE],
        gpu_usage_hist: vec![0.0; HISTORY_SIZE],
        gpu_temp_hist: vec![0.0; HISTORY_SIZE],
        gpu_clock_hist: vec![0.0; HISTORY_SIZE],
        first_telemetry: true,
    };

    // Callbacks
    {
        let tx = tx.clone();
        ui.on_apply_power(move |name| { let _ = tx.try_send(Action::ApplyPowerProfile(name.into())); });
    }
    {
        let tx = tx.clone();
        ui.on_apply_thermal(move |name| { let _ = tx.try_send(Action::ApplyThermalProfile(name.into())); });
    }
    {
        let tx = tx.clone();
        ui.on_delete_power(move |name| { 
            let _ = tx.try_send(Action::DeletePowerProfile(name.into()));
            let _ = tx.try_send(Action::GetConfig);
        });
    }
    {
        let tx = tx.clone();
        ui.on_save_power(move |p_ui| {
            let p = PowerProfile {
                name: p_ui.name.into(),
                description: p_ui.description.into(),
                cpu_max_mhz: p_ui.cpu_mhz as u32,
                gpu_lock_mhz: p_ui.gpu_mhz as u32,
                p_short_w: p_ui.p_short as u32,
                p_long_w: p_ui.p_long as u32,
            };
            let _ = tx.try_send(Action::SavePowerProfile(p));
            let _ = tx.try_send(Action::GetConfig);
        });
    }
    {
        let tx = tx.clone();
        ui.on_set_manual(move |t, v| {
            let action = match t.as_str() {
                "cpu" => Action::SetCpuMaxClock(v as u32),
                "thermal" => Action::SetThermalTarget(v as u32),
                "gputemp" => Action::SetGpuTempLimit(v as u32),
                _ => return,
            };
            let _ = tx.try_send(action);
        });
    }

    // Spawn networking
    tokio::spawn(async move {
        loop {
            if let Ok(mut stream) = UnixStream::connect(SOCKET_PATH).await {
                let _ = send_action_raw(&mut stream, Action::GetConfig, &ui_handle, None).await;

                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(1000)) => {
                            if let Err(_) = send_action_raw(&mut stream, Action::GetTelemetry, &ui_handle, Some(&mut state)).await { break; }
                        }
                        Some(action) = rx.recv() => {
                            if let Err(_) = send_action_raw(&mut stream, action, &ui_handle, None).await { break; }
                        }
                    }
                }
            } else {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    });

    // Fetch initial system info via inxi in background
    let ui_info_handle = ui.as_weak();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        match fetch_inxi_data().await {
            Ok(info) => {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_info_handle.upgrade() {
                        ui.set_sys_distro(info.distro.into());
                        ui.set_sys_desktop(info.desktop.into());
                        ui.set_sys_kernel(info.kernel.into());
                        ui.set_sys_cpu(info.cpu.into());
                        ui.set_sys_gpu(info.gpu.into());
                        ui.set_sys_ram(info.ram.into());
                        ui.set_sys_model(info.model.into());
                        ui.set_sys_raw_info(info.raw.into());
                    }
                });
            }
            Err(e) => {
                let err_msg = format!("Failed to fetch hardware specifications: {}\n\nPlease ensure 'inxi' is installed on your system.", e);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_info_handle.upgrade() {
                        ui.set_sys_raw_info(err_msg.into());
                    }
                });
            }
        }
    });

    ui.run().context("Failed to run Slint loop")?;
    Ok(())
}

struct InxiData {
    distro: String,
    desktop: String,
    kernel: String,
    cpu: String,
    gpu: String,
    ram: String,
    model: String,
    raw: String,
}

async fn fetch_inxi_data() -> Result<InxiData> {
    use tokio::process::Command;
    let output = Command::new("inxi")
        .args(["-Fxz", "-c0"]) 
        .output().await?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if !output.status.success() && stdout.is_empty() {
        anyhow::bail!("inxi failed: {}", stderr);
    }

    let mut data = InxiData {
        distro: "Unknown".into(),
        desktop: "Unknown".into(),
        kernel: "Unknown".into(),
        cpu: "Unknown".into(),
        gpu: "Unknown".into(),
        ram: "Unknown".into(),
        model: "Unknown".into(),
        raw: if stdout.is_empty() { stderr.to_string() } else { stdout.to_string() },
    };

    let mut current_section = "";

    for line in stdout.lines() {
        let trimmed = line.trim();
        if line.starts_with("System:") { current_section = "System"; continue; }
        if line.starts_with("Machine:") { current_section = "Machine"; continue; }
        if line.starts_with("CPU:") { current_section = "CPU"; continue; }
        if line.starts_with("Graphics:") { current_section = "Graphics"; continue; }
        if line.starts_with("Info:") { current_section = "Info"; continue; }

        match current_section {
            "System" => {
                if trimmed.contains("Kernel:") {
                    data.kernel = trimmed.split("Kernel:").nth(1).unwrap_or("").split("arch:").next().unwrap_or("").trim().to_string();
                }
                if trimmed.contains("Desktop:") {
                    data.desktop = trimmed.split("Desktop:").nth(1).unwrap_or("").split("Distro:").next().unwrap_or("").trim().to_string();
                }
                if trimmed.contains("Distro:") {
                    data.distro = trimmed.split("Distro:").nth(1).unwrap_or("").split("base:").next().unwrap_or("").trim().to_string();
                }
            }
            "Machine" => {
                if trimmed.contains("product:") {
                    data.model = trimmed.split("product:").nth(1).unwrap_or("").split("v:").next().unwrap_or("").trim().to_string();
                }
            }
            "CPU" => {
                if trimmed.contains("model:") {
                    data.cpu = trimmed.split("model:").nth(1).unwrap_or("").split("bits:").next().unwrap_or("").trim().to_string();
                }
            }
            "Graphics" => {
                if trimmed.contains("Device-") && trimmed.contains("model:") {
                    let g = trimmed.split("model:").nth(1).unwrap_or("").split("vendor:").next().unwrap_or("").trim().to_string();
                    if data.gpu == "Unknown" { data.gpu = g; }
                    else if !data.gpu.contains(&g) { data.gpu = format!("{} | {}", data.gpu, g); }
                }
            }
            "Info" => {
                if trimmed.contains("Memory:") {
                    data.ram = trimmed.split("total:").nth(1).unwrap_or("").split("note:").next().unwrap_or("").trim().to_string();
                }
            }
            _ => {}
        }
    }

    Ok(data)
}

async fn send_action_raw(stream: &mut UnixStream, action: Action, ui_handle: &slint::Weak<AppWindow>, state: Option<&mut AppState>) -> Result<()> {
    let req_bytes = serde_json::to_vec(&action)?;
    stream.write_all(&req_bytes).await?;

    let mut buffer = [0u8; 16384];
    let n = stream.read(&mut buffer).await?;
    if n == 0 { anyhow::bail!("Closed"); }

    let resp: DaemonResponse = serde_json::from_slice(&buffer[..n])?;
    
    let mut cpu_px = None;
    let mut gpu_px = None;
    let mut first_tel_sync = false;
    let mut tel_copy = None;

    let scale_factor = ui_handle.upgrade().map(|u| u.window().scale_factor()).unwrap_or(1.0);

    if let DaemonResponse::Telemetry(ref tel) = resp {
        tel_copy = Some(tel.clone());
        if let Some(s) = state {
            s.cpu_usage_hist.remove(0); s.cpu_usage_hist.push(tel.cpu_usage / 100.0);
            s.cpu_temp_hist.remove(0); s.cpu_temp_hist.push(tel.cpu_temp / 100.0);
            s.cpu_clock_hist.remove(0); s.cpu_clock_hist.push(tel.cpu_clock_mhz as f32 / 5000.0);
            s.gpu_usage_hist.remove(0); s.gpu_usage_hist.push(tel.gpu_usage / 100.0);
            s.gpu_temp_hist.remove(0); s.gpu_temp_hist.push(tel.gpu_temp / 100.0);
            s.gpu_clock_hist.remove(0); s.gpu_clock_hist.push(tel.gpu_clock_mhz as f32 / 2500.0);
            
            if s.first_telemetry {
                first_tel_sync = true;
                s.first_telemetry = false;
            }

            let cpu_series = vec![
                Series { data: s.cpu_clock_hist.clone(), color: [0, 229, 255] },
                Series { data: s.cpu_usage_hist.clone(), color: [41, 121, 255] },
                Series { data: s.cpu_temp_hist.clone(), color: [255, 23, 68] },
            ];
            cpu_px = render_multi_chart_skia(&cpu_series, scale_factor).ok();

            let gpu_series = vec![
                Series { data: s.gpu_clock_hist.clone(), color: [0, 229, 255] },
                Series { data: s.gpu_usage_hist.clone(), color: [0, 230, 118] },
                Series { data: s.gpu_temp_hist.clone(), color: [255, 23, 68] },
            ];
            gpu_px = render_multi_chart_skia(&gpu_series, scale_factor).ok();
        }
    }

    let ui_h = ui_handle.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui_h.upgrade() {
            match resp {
                DaemonResponse::Telemetry(_) => {
                    let tel = tel_copy.unwrap();
                    ui.set_cpu_name(tel.cpu_name.into());
                    ui.set_gpu_name(tel.gpu_name.into());
                    ui.set_cpu_temp(tel.cpu_temp);
                    ui.set_gpu_temp(tel.gpu_temp);
                    ui.set_cpu_clock(tel.cpu_clock_mhz as i32);
                    ui.set_gpu_clock(tel.gpu_clock_mhz as i32);
                    ui.set_cpu_usage(tel.cpu_usage);
                    ui.set_gpu_usage(tel.gpu_usage);
                    ui.set_vram_used(tel.vram_used_mb as i32);
                    ui.set_vram_total(tel.vram_total_mb as i32);
                    ui.set_ram_used(tel.ram_used_gb);
                    ui.set_ram_total(tel.ram_total_gb);
                    ui.set_uptime(format_uptime(tel.uptime_secs).into());
                    ui.set_kernel(tel.kernel.into());
                    ui.set_ac_connected(tel.ac_connected);
                    ui.set_active_power(tel.active_power_profile.into());
                    ui.set_active_thermal(tel.active_thermal_profile.into());
                    ui.set_is_throttled(tel.is_throttled);
                    
                    ui.set_active_cpu_limit(tel.active_cpu_limit as i32);
                    ui.set_active_gpu_limit(tel.active_gpu_limit as i32);
                    ui.set_active_thermal_limit(tel.active_thermal_limit as i32);
                    ui.set_active_gpu_temp_limit(tel.active_gpu_temp_limit as i32);

                    if first_tel_sync {
                        ui.set_manual_cpu_val(tel.active_cpu_limit as f32);
                        ui.set_manual_thermal_val(tel.active_thermal_limit as f32);
                        ui.set_manual_gpu_temp_val(tel.active_gpu_temp_limit as f32);
                    }
                    if let Some(px) = cpu_px { ui.set_cpu_graph(Image::from_rgba8(px)); }
                    if let Some(px) = gpu_px { ui.set_gpu_graph(Image::from_rgba8(px)); }
                }
                DaemonResponse::Config(conf) => {
                    let mut p_ui: Vec<PowerProfileUI> = conf.power_profiles.values().map(|p| {
                        PowerProfileUI {
                            name: p.name.clone().into(),
                            description: p.description.clone().into(),
                            cpu_mhz: p.cpu_max_mhz as i32,
                            gpu_mhz: p.gpu_lock_mhz as i32,
                            p_short: p.p_short_w as i32,
                            p_long: p.p_long_w as i32,
                            is_system: is_system_p(&p.name),
                        }
                    }).collect();
                    p_ui.sort_by(|a, b| a.name.cmp(&b.name));
                    ui.set_power_profiles(Rc::new(VecModel::from(p_ui)).into());

                    let mut t_ui: Vec<ThermalProfileUI> = conf.thermal_profiles.values().map(|t| {
                        ThermalProfileUI {
                            name: t.name.clone().into(),
                            description: t.description.clone().into(),
                            tj_offset: t.cpu_tj_offset as i32,
                            gpu_limit: t.gpu_temp_limit as i32,
                            is_system: t.name == "Normal" || t.name == "None",
                        }
                    }).collect();
                    t_ui.sort_by(|a, b| a.name.cmp(&b.name));
                    ui.set_thermal_profiles(Rc::new(VecModel::from(t_ui)).into());
                }
                DaemonResponse::Ok => { show_toast(&ui, "Action Applied Successfully!", false); }
                DaemonResponse::Error(e) => { show_toast(&ui, &format!("Error: {}", e), true); }
            }
        }
    });

    Ok(())
}

fn render_multi_chart_skia(series_list: &[Series], scale: f32) -> Result<SharedPixelBuffer<Rgba8Pixel>> {
    let logical_w = 400.0;
    let logical_h = 200.0;
    let w = (logical_w * scale) as u32;
    let h = (logical_h * scale) as u32;
    if w == 0 || h == 0 { anyhow::bail!("Zero size"); }
    let mut pixmap = Pixmap::new(w, h).context("Failed to create pixmap")?;
    pixmap.fill(Color::from_rgba8(11, 14, 20, 255));
    let step_x = w as f32 / (HISTORY_SIZE - 1) as f32;
    for series in series_list {
        if series.data.len() < 2 { continue; }
        let mut pb = PathBuilder::new();
        for (i, &v) in series.data.iter().enumerate() {
            let x = i as f32 * step_x;
            let y = h as f32 - (v.clamp(0.0, 1.0) * h as f32);
            if i == 0 { pb.move_to(x, y); } else { pb.line_to(x, y); }
        }
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(Color::from_rgba8(series.color[0], series.color[1], series.color[2], 255));
            paint.anti_alias = true;
            let mut stroke = Stroke::default();
            stroke.width = 3.0 * scale;
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            if series.color == series_list[0].color {
                let mut fill_pb = PathBuilder::new();
                for (i, &v) in series.data.iter().enumerate() {
                    let x = i as f32 * step_x;
                    let y = h as f32 - (v.clamp(0.0, 1.0) * h as f32);
                    if i == 0 { fill_pb.move_to(x, y); } else { fill_pb.line_to(x, y); }
                }
                fill_pb.line_to(w as f32, h as f32);
                fill_pb.line_to(0.0, h as f32);
                fill_pb.close();
                if let Some(fill_path) = fill_pb.finish() {
                    let mut f_paint = Paint::default();
                    f_paint.set_color(Color::from_rgba8(series.color[0], series.color[1], series.color[2], 30));
                    f_paint.anti_alias = true;
                    pixmap.fill_path(&fill_path, &f_paint, FillRule::Winding, Transform::identity(), None);
                }
            }
        }
    }
    let mut slint_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
    slint_buffer.make_mut_slice().copy_from_slice(bytemuck::cast_slice(pixmap.data()));
    Ok(slint_buffer)
}

fn show_toast(ui: &AppWindow, msg: &str, is_error: bool) {
    ui.set_toast_msg(msg.into());
    ui.set_toast_is_error(is_error);
    ui.set_show_toast(true);
    let ui_h = ui.as_weak();
    slint::Timer::single_shot(Duration::from_secs(3), move || {
        if let Some(ui) = ui_h.upgrade() { ui.set_show_toast(false); }
    });
}

fn is_system_p(name: &str) -> bool {
    matches!(name, "powersave" | "balanced" | "gmode-lite" | "gmode-max" | "stock")
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 { format!("{}d {}h {}m", days, hours, mins) }
    else { format!("{}h {}m", hours, mins) }
}
