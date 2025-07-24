pub use controller::MotionController;
// src/motion/mod.rs - Activate advanced features
use crate::config::Config;
use crate::hardware::HardwareManager;
use crate::motion::planner::{MotionPlanner, MotionType};
use crate::printer::PrinterState;
use std::sync::Arc;
use tokio::sync::RwLock;

mod junction;
mod kinematics;
pub mod planner;
pub mod controller;
pub mod shaper;

pub use planner::MotionError;

/// Statistics for the motion queue
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub length: usize,
    pub max_length: usize,
    pub last_command: Option<String>,
}
