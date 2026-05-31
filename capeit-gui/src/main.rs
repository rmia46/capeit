slint::include_modules!();

mod telemetry;
mod sys_info;
mod utils;

use anyhow::{Context, Result};
use capeit_common::{Action, DaemonResponse, PowerProfile};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use slint::{VecModel, ComponentHandle};
use std::rc::Rc;
use std::time::Duration;

use crate::telemetry::AppState;
use crate::sys_info::fetch_inxi_data;
use crate::utils::{is_system_profile, format_uptime};

const SOCKET_PATH: &str = "/tmp/capeit.sock";

#[tokio::main]
async fn main() -> Result<()> {
    let ui = AppWindow::new().context("Failed to create UI")?;

    let (tx, mut rx) = mpsc::channel::<Action>(32);
    let mut state = AppState::new();

    // UI Callbacks setup
    setup_callbacks(&ui, tx.clone());

    // Main background loop for networking and telemetry
    let loop_ui_handle = ui.as_weak();
    tokio::spawn(async move {
        loop {
            if let Ok(mut stream) = UnixStream::connect(SOCKET_PATH).await {
                // Initial config fetch
                let _ = send_action_raw(&mut stream, Action::GetConfig, &loop_ui_handle, &mut state).await;

                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(1000)) => {
                            if let Err(_) = send_action_raw(&mut stream, Action::GetTelemetry, &loop_ui_handle, &mut state).await { break; }
                        }
                        Some(action) = rx.recv() => {
                            if let Err(_) = send_action_raw(&mut stream, action, &loop_ui_handle, &mut state).await { break; }
                        }
                    }
                }
            } else {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    });

    // Async System Info Fetching
    let sys_ui_handle = ui.as_weak();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        match fetch_inxi_data().await {
            Ok(info) => {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = sys_ui_handle.upgrade() {
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
                let err_msg = format!("Failed to fetch hardware specifications: {}\n\nPlease ensure 'inxi' is installed.", e);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = sys_ui_handle.upgrade() {
                        ui.set_sys_raw_info(err_msg.into());
                    }
                });
            }
        }
    });

    ui.run().context("Failed to run Slint loop")?;
    Ok(())
}

fn setup_callbacks(ui: &AppWindow, tx: mpsc::Sender<Action>) {
    let tx_apply_p = tx.clone();
    ui.on_apply_power(move |name| { let _ = tx_apply_p.try_send(Action::ApplyPowerProfile(name.into())); });

    let tx_apply_t = tx.clone();
    ui.on_apply_thermal(move |name| { let _ = tx_apply_t.try_send(Action::ApplyThermalProfile(name.into())); });

    let tx_delete = tx.clone();
    ui.on_delete_power(move |name| { 
        let _ = tx_delete.try_send(Action::DeletePowerProfile(name.into()));
        let _ = tx_delete.try_send(Action::GetConfig);
    });

    let tx_save = tx.clone();
    ui.on_save_power(move |p_ui| {
        let p = PowerProfile {
            name: p_ui.name.into(),
            description: p_ui.description.into(),
            cpu_max_mhz: p_ui.cpu_mhz as u32,
            gpu_lock_mhz: p_ui.gpu_mhz as u32,
            p_short_w: p_ui.p_short as u32,
            p_long_w: p_ui.p_long as u32,
        };
        let _ = tx_save.try_send(Action::SavePowerProfile(p));
        let _ = tx_save.try_send(Action::GetConfig);
    });

    let tx_manual = tx.clone();
    ui.on_set_manual(move |t, v| {
        let action = match t.as_str() {
            "cpu" => Action::SetCpuMaxClock(v as u32),
            "thermal" => Action::SetThermalTarget(v as u32),
            "gputemp" => Action::SetGpuTempLimit(v as u32),
            _ => return,
        };
        let _ = tx_manual.try_send(action);
    });
}

async fn send_action_raw(
    stream: &mut UnixStream, 
    action: Action, 
    ui_handle: &slint::Weak<AppWindow>, 
    state: &mut AppState,
) -> Result<()> {
    let req_bytes = serde_json::to_vec(&action)?;
    stream.write_all(&req_bytes).await?;

    let mut buffer = [0u8; 16384];
    let n = stream.read(&mut buffer).await?;
    if n == 0 { anyhow::bail!("Closed"); }

    let resp: DaemonResponse = serde_json::from_slice(&buffer[..n])?;
    
    let mut first_tel_sync = false;
    let mut tel_copy = None;

    if let DaemonResponse::Telemetry(ref tel) = resp {
        tel_copy = Some(tel.clone());
        if state.first_telemetry {
            first_tel_sync = true;
            state.first_telemetry = false;
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
                            is_system: is_system_profile(&p.name),
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

fn show_toast(ui: &AppWindow, msg: &str, is_error: bool) {
    ui.set_toast_msg(msg.into());
    ui.set_toast_is_error(is_error);
    ui.set_show_toast(true);
    let ui_h = ui.as_weak();
    slint::Timer::single_shot(Duration::from_secs(3), move || {
        if let Some(ui) = ui_h.upgrade() { ui.set_show_toast(false); }
    });
}
