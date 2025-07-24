pub use controller::MotionController;
// src/motion/mod.rs - Activate advanced features

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
