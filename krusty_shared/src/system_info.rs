/// Shared system information structure for host/simulator
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub version: String,
    pub name: String,
    pub rust_version: String,
    pub uptime: f64,
}
