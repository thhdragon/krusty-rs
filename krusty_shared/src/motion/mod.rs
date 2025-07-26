// krusty_shared::motion::mod.rs
// Expose motion planning and adaptive modules

pub mod planner;
pub mod adaptive;
pub mod controller;

pub use controller::{MotionController, PrinterStateTrait, HardwareManagerTrait};

/// Statistics for the motion queue
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub length: usize,
    pub max_length: usize,
    pub last_command: Option<String>,
}

/// Represents planner feature toggles (Basic, Adaptive, SnapCrackle)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotionMode {
    Basic,
    Adaptive,
    SnapCrackle,
}
