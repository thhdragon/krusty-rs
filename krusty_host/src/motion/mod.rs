pub use controller::MotionController;
// src/motion/mod.rs - Activate advanced features

mod junction;
mod kinematics;
pub mod planner;
pub mod controller;
pub mod shaper;
pub mod stepper;

pub use planner::MotionError;

use std::sync::{Arc, Mutex};
use crate::hardware::board_config::BoardConfig;
#[cfg(feature = "sim-in-host")]
use krusty_simulator::simulator::event_queue::SimEventQueue;

/// Statistics for the motion queue
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub length: usize,
    pub max_length: usize,
    pub last_command: Option<String>,
}

#[cfg(feature = "sim-in-host")]
pub struct MotionSystem {
    pub event_queue: Arc<Mutex<SimEventQueue>>,
    pub board: BoardConfig,
}

#[cfg(not(feature = "sim-in-host"))]
pub struct MotionSystem {
    pub board: BoardConfig,
}

impl MotionSystem {
    #[cfg(feature = "sim-in-host")]
    pub fn new(event_queue: Arc<Mutex<SimEventQueue>>, board: BoardConfig) -> Self {
        Self { event_queue, board }
    }
    #[cfg(not(feature = "sim-in-host"))]
    pub fn new(board: BoardConfig) -> Self {
        Self { board }
    }
}
