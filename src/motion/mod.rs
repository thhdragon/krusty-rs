// src/motion/mod.rs - Activate advanced features
use crate::config::Config;
use crate::hardware::HardwareManager;
use crate::motion::planner::{MotionConfig, MotionPlanner, MotionType};
use crate::printer::PrinterState;
use std::sync::Arc;
use tokio::sync::RwLock;

mod junction;
mod kinematics;
mod planner;
pub mod controller;
pub mod shaper;

pub use planner::MotionError;

#[derive(Debug)]
pub struct MotionController {
    pub state: Arc<RwLock<PrinterState>>,
    pub hardware_manager: HardwareManager,
    pub planner: MotionPlanner,
    pub current_position: [f64; 4],
}

impl Default for MotionController {
    fn default() -> Self {
        // Provide dummy/default values for test usage
        use crate::hardware::HardwareManager;
        use crate::motion::planner::MotionPlanner;
        use crate::printer::PrinterState;
        use std::sync::Arc;
        use tokio::sync::RwLock;
        let dummy_config = crate::config::Config::default();
        Self {
            state: Arc::new(RwLock::new(PrinterState::default())),
            hardware_manager: HardwareManager::new(Default::default()),
            planner: MotionPlanner::new_from_config(&dummy_config),
            current_position: [0.0; 4],
        }
    }
}

impl MotionController {
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        hardware_manager: HardwareManager,
        config: &Config,
    ) -> Self {
        let planner = MotionPlanner::new_from_config(config);
        Self {
            state,
            hardware_manager,
            planner,
            current_position: [0.0; 4],
        }
    }

    pub async fn queue_linear_move(
        &mut self,
        target: [f64; 3],
        feedrate: Option<f64>,
        extrude: Option<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let current_e = self.current_position[3];
        let target_e = if let Some(e) = extrude {
            current_e + e
        } else {
            current_e
        };
        
        let feedrate = feedrate.unwrap_or(300.0);
        let target_4d = [target[0], target[1], target[2], target_e];
        
        let motion_type = if extrude.is_some() && extrude.unwrap() > 0.0 {
            MotionType::Print
        } else {
            MotionType::Travel
        };
        
        // Plan the move with advanced motion planning
        self.planner.plan_move(target_4d, feedrate, motion_type).await?;
        
        // Update current position
        self.current_position = target_4d;
        
        Ok(())
    }

    pub async fn queue_home(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Homing all axes");
        
        // Plan home move to [0, 0, 0, current_e]
        let home_target = [0.0, 0.0, 0.0, self.current_position[3]];
        self.planner.plan_move(home_target, 50.0, MotionType::Home).await?;
        
        // Update position
        self.current_position = home_target;
        
        // Send home command to hardware
        let _ = self.hardware_manager.send_command("home_all").await;
        
        // Update printer state
        {
            let mut state = self.state.write().await;
            state.position = [0.0, 0.0, 0.0];
        }
        
        Ok(())
    }

    pub async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Update the motion planner at high frequency
        self.planner.update().await?;
        Ok(())
    }

    pub fn emergency_stop(&mut self) {
        tracing::warn!("Emergency stop activated - clearing motion queue");
        self.planner.clear_queue();
    }

    pub fn get_current_position(&self) -> [f64; 4] {
        self.planner.get_current_position()
    }
}