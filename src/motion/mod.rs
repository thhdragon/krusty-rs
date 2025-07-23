// src/motion/mod.rs

// --- Submodules ---
pub mod kinematics;
pub mod junction;
pub mod shaper;
pub mod trajectory;
pub mod stepper;
pub mod planner; // This now points to src/motion/planner/mod.rs

// --- Re-exports for external use ---
pub use planner::{MotionPlanner, MotionConfig, MotionType, MotionSegment};

// --- Imports ---
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::PrinterState;
use crate::hardware::HardwareManager;


#[derive(Clone)]
pub struct MotionController {
    state: Arc<RwLock<PrinterState>>,
    hardware_manager: HardwareManager,
    planner: MotionPlanner,
    step_generator: crate::motion::stepper::StepGenerator,
}

impl MotionController {
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        hardware_manager: HardwareManager,
        config: &crate::config::Config,
    ) -> Self {
        let planner = MotionPlanner::new_from_config(config);
        let get_steps_per_mm = |axis: &str| {
            config.steppers.get(axis).map_or(80.0, |s| {
                let steps = (s.full_steps_per_rotation * s.microsteps) as f64;
                steps / s.rotation_distance
            })
        };
        let steps_per_mm = [
            get_steps_per_mm("x"),
            get_steps_per_mm("y"),
            get_steps_per_mm("z"),
            get_steps_per_mm("e"),
        ];
        let direction_invert = [false, false, false, false];
        let step_generator = crate::motion::stepper::StepGenerator::new(steps_per_mm, direction_invert);
        Self {
            state,
            hardware_manager,
            planner,
            step_generator,
        }
    }

    pub async fn queue_linear_move(
        &mut self,
        target: [f64; 3],
        feedrate: Option<f64>,
        extrude: Option<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let current_pos = self.planner.get_current_position();
        let current_e = current_pos[3];
        let target_e = if let Some(e) = extrude {
            current_e + e
        } else {
            current_e
        };
        let feedrate = feedrate.unwrap_or(300.0);
        let target_4d = [target[0], target[1], target[2], target_e];
        let motion_type = if extrude.is_some() && extrude.unwrap() > 0.0 {
            planner::MotionType::Print
        } else {
            planner::MotionType::Travel
        };
        self.planner.plan_move(target_4d, feedrate, motion_type).await?;
        {
            let mut state = self.state.write().await;
            state.position = [target_4d[0], target_4d[1], target_4d[2]];
        }
        Ok(())
    }

    pub async fn queue_home(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Homing all axes");
        let current_pos = self.planner.get_current_position();
        let home_target = [0.0, 0.0, 0.0, current_pos[3]];
        self.planner.plan_move(home_target, 50.0, planner::MotionType::Home).await?;
        let _ = self.hardware_manager.send_command("home_all").await;
        {
            let mut state = self.state.write().await;
            state.position = [0.0, 0.0, 0.0];
        }
        Ok(())
    }

    pub fn emergency_stop(&mut self) {
        tracing::warn!("Emergency stop activated - clearing motion queue");
        self.planner.clear_queue();
        // Add hardware emergency stop commands if needed in the future
    }

    pub async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.planner.update().await
    }

    pub fn queue_length(&self) -> usize {
        self.planner.queue_length()
    }

    pub fn get_current_position(&self) -> [f64; 4] {
        self.planner.get_current_position()
    }

    pub fn set_position(&mut self, position: [f64; 4]) {
        self.planner.set_position(position);
    }
}

// Implement Debug manually for MotionController if derive(Debug) causes issues.
impl std::fmt::Debug for MotionController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MotionController")
            .field("state", &"Arc<RwLock<PrinterState>>")
            .field("hardware_manager", &self.hardware_manager)
            .field("planner", &self.planner)
            .finish()
    }
}

impl Default for MotionController {
    fn default() -> Self {
        use crate::config::Config;
        let state = Arc::new(RwLock::new(PrinterState::new()));
        let hardware_manager = HardwareManager::new(Config::default());
        let config = Config::default();
        MotionController::new(state, hardware_manager, &config)
    }
}
