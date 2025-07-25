pub use controller::MotionController;
// src/motion/mod.rs - Activate advanced features

mod junction;
mod kinematics;
pub mod planner;
pub mod controller;
pub mod shaper;

pub use planner::MotionError;

use std::sync::{Arc, Mutex};
use crate::hardware::board_config::BoardConfig;
use crate::simulator::event_queue::SimEventQueue;

/// Statistics for the motion queue
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub length: usize,
    pub max_length: usize,
    pub last_command: Option<String>,
}

pub struct MotionSystem {
    pub event_queue: Arc<Mutex<SimEventQueue>>,
    pub board: BoardConfig,
}

impl MotionSystem {
    pub fn new(event_queue: Arc<Mutex<SimEventQueue>>, board: BoardConfig) -> Self {
        // Example: Schedule initial motion event
        // event_queue.push(SimEvent { ... });
        Self { event_queue, board }
    }
}
