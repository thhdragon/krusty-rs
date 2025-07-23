// src/motion/mod.rs

// --- Submodules ---
pub mod planner; // src/motion/planner/mod.rs
pub mod kinematics;
pub mod junction;
pub mod stepper;
pub mod shaper;
pub mod trajectory;
// ... add other modules as needed

// --- Re-exports for external use ---
pub use planner::{MotionPlanner, MotionConfig, MotionType};
// Add more as needed, e.g.:
// pub use planner::{MotionSegment, AdaptiveMotionPlanner, SnapCrackleMotion};

// --- Imports ---
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::PrinterState;
use crate::hardware::HardwareManager;


#[derive(Clone)] // Remove Debug derive or implement manually
pub struct MotionController {
    state: Arc<RwLock<PrinterState>>,
    hardware_manager: HardwareManager,
    planner: MotionPlanner, // Use the new type
    current_position: [f64; 4],
    // Remove or update other fields like kinematics, junction_deviation if they are now internal to the planner
    // kinematics: Box<dyn Kinematics>,
    // junction_deviation: JunctionDeviation,
}

impl MotionController {
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        hardware_manager: HardwareManager,
        config: &crate::config::Config, // Pass config to configure the planner
    ) -> Self {
        // Create motion configuration using the new function
        // let motion_config = MotionConfig::new_from_config(config);
        // let planner = MotionPlanner::new(motion_config);

        // Or use the direct constructor from config
        let planner = MotionPlanner::new_from_config(config);

        // Create kinematics handler (if still needed outside planner)
        // let kinematics = create_kinematics(KinematicsType::Cartesian, [[0.0, 200.0], [0.0, 200.0], [0.0, 200.0]]);

        // Create junction deviation calculator (if still needed outside planner)
        // let junction_deviation = JunctionDeviation::new(0.05);

        Self {
            state,
            hardware_manager,
            planner, // Use the new planner instance
            current_position: [0.0, 0.0, 0.0, 0.0],
            // kinematics,
            // junction_deviation,
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

        // Determine motion type
        let motion_type = if extrude.is_some() && extrude.unwrap() > 0.0 {
            MotionType::Print
        } else {
            MotionType::Travel
        };

        // Plan the move using the new planner
        self.planner.plan_move(target_4d, feedrate, motion_type).await?;

        // Update current position in the controller as well (or rely on planner?)
        self.current_position = target_4d;

        // Generate and send steps (placeholder)
        // self.generate_and_send_steps(&target_4d).await?;

        // Update printer state
        {
            let mut state = self.state.write().await;
            state.position = [target_4d[0], target_4d[1], target_4d[2]];
        }

        Ok(())
    }

    pub async fn queue_home(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // println!("Homing all axes");
        // Plan a home move to [0, 0, 0, current_e]
        let home_target = [0.0, 0.0, 0.0, self.current_position[3]];
        self.planner.plan_move(home_target, 50.0, MotionType::Home).await?; // Slow homing speed

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

    // Placeholder for step generation
    /*
    async fn generate_and_send_steps(&mut self, target: &[f64; 4]) -> Result<(), Box<dyn std::error::Error>> {
        // Generate step commands using the step generator
        let steps = self.step_generator.generate_steps(target); // Assuming step_generator exists

        // Send each step command to hardware
        for step_cmd in steps {
            let mcu_cmd = step_cmd.to_mcu_command();
            let _ = self.hardware_manager.send_command(&mcu_cmd).await;
        }

        Ok(())
    }
    */

    pub fn emergency_stop(&mut self) {
        // println!("Emergency stop activated - clearing motion queue");
        self.planner.clear_queue();
        // Add hardware emergency stop commands if needed
        // let _ = self.hardware_manager.send_command("emergency_stop").await;
    }

    pub fn get_current_position(&self) -> [f64; 4] {
        // Could get from planner or maintain locally
        // self.planner.get_current_position()
        self.current_position
    }

    // Forward the update call to the planner
    pub async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.planner.update().await
    }

    // Add other methods as needed, forwarding to the planner where appropriate
    pub fn queue_length(&self) -> usize {
        self.planner.queue_length()
    }
}

// Implement Debug manually for MotionController if it contains non-Debug types
impl std::fmt::Debug for MotionController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MotionController")
            .field("state", &"Arc<RwLock<PrinterState>>")
            .field("hardware_manager", &self.hardware_manager)
            .field("planner", &self.planner) // This should work now if MotionPlanner implements Debug
            .field("current_position", &self.current_position)
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
