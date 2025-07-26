pub use controller::MotionController;
pub use krusty_shared::trajectory::MotionError; // Use shared MotionError for use in host_os.rs and elsewhere
// src/motion/mod.rs - Activate advanced features

// mod junction; // migrated to krusty_shared
// mod kinematics; // migrated to krusty_shared
// pub mod stepper; // if not present, comment out

// pub mod planner; // Removed: now using krusty_shared::motion::planner
pub mod controller;

use krusty_shared::board_config::BoardConfig;
#[cfg(feature = "sim-in-host")]
use krusty_shared::event_queue::SimEventQueue;

pub use krusty_shared::trajectory::{
    MotionType, TrajectoryConfig, TrajectoryError, TrajectoryGenerator, TrajectorySegment,
};

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
