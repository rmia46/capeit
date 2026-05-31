use anyhow::Result;
use tokio::process::Command;

pub struct InxiData {
    pub distro: String,
    pub desktop: String,
    pub kernel: String,
    pub cpu: String,
    pub gpu: String,
    pub ram: String,
    pub model: String,
    pub raw: String,
}

pub async fn fetch_inxi_data() -> Result<InxiData> {
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
