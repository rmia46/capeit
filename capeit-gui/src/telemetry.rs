use anyhow::Result;

pub struct AppState {
    pub first_telemetry: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            first_telemetry: true,
        }
    }
}
