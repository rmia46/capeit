pub fn is_system_profile(name: &str) -> bool {
    matches!(name, "powersave" | "balanced" | "gmode-lite" | "gmode-max" | "stock")
}

pub fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 { format!("{}d {}h {}m", days, hours, mins) }
    else { format!("{}h {}m", hours, mins) }
}
