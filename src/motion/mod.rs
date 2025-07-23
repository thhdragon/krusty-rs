// src/motion/mod.rs - Proper module declaration
pub mod kinematics;
pub mod junction;
pub mod shaper;
pub mod trajectory;
pub mod stepper;
pub mod planner;
pub mod snap_crackle;
pub mod advanced_planner;
pub mod adaptive_planner;

// Re-export commonly used items
pub use kinematics::{Kinematics, KinematicsType, create_kinematics};
pub use junction::JunctionDeviation;
pub use planner::{MotionPlanner, MotionConfig, MotionType};
pub use stepper::StepGenerator;

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::PrinterState;
use crate::hardware::HardwareManager;
use crate::config::Config;

/// Main motion controller that orchestrates all motion operations
#[derive(Clone)]
pub struct MotionController {
    /// Shared printer state
    state: Arc<RwLock<PrinterState>>,
    
    /// Hardware interface
    hardware_manager: HardwareManager,
    
    /// Motion planner
    planner: MotionPlanner,
    
    /// Kinematics handler
    kinematics: Box<dyn Kinematics>,
    
    /// Junction deviation calculator
    junction_deviation: JunctionDeviation,
    
    /// Current position [X, Y, Z, E]
    current_position: [f64; 4],
    
    /// Step generator
    step_generator: StepGenerator,
}

// Manual Debug implementation
impl std::fmt::Debug for MotionController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MotionController")
            .field("state", &"Arc<RwLock<PrinterState>>")
            .field("hardware_manager", &self.hardware_manager)
            .field("planner", &self.planner)
            .field("kinematics", &"Box<dyn Kinematics>")
            .field("junction_deviation", &self.junction_deviation)
            .field("current_position", &self.current_position)
            .field("step_generator", &self.step_generator)
            .finish()
    }
}

impl Default for MotionController {
    fn default() -> Self {
        let state = Arc::new(RwLock::new(PrinterState::new()));
        let config = Config::default();
        let hardware_manager = HardwareManager::new(config);
        MotionController::new(state, hardware_manager)
    }
}

impl MotionController {
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        hardware_manager: HardwareManager,
    ) -> Self {
        // Create motion configuration
        let motion_config = MotionConfig {
            max_velocity: [300.0, 300.0, 25.0, 50.0],
            max_acceleration: [3000.0, 3000.0, 100.0, 1000.0],
            max_jerk: [20.0, 20.0, 0.5, 2.0],
            junction_deviation: 0.05,
            axis_limits: [[0.0, 200.0], [0.0, 200.0], [0.0, 200.0]],
            kinematics_type: KinematicsType::Cartesian,
            minimum_step_distance: 0.001,
            lookahead_buffer_size: 16,
        };
        
        let planner = MotionPlanner::new(motion_config);
        
        // Create kinematics handler
        let kinematics = create_kinematics(KinematicsType::Cartesian, [[0.0, 200.0], [0.0, 200.0], [0.0, 200.0]]);
        
        // Create junction deviation calculator
        let junction_deviation = JunctionDeviation::new(0.05);
        
        // Create step generator with typical steps/mm values
        let step_generator = StepGenerator::new(
            [80.0, 80.0, 400.0, 100.0], // Steps per mm for X, Y, Z, E
            [false, false, false, false], // No direction inversion
        );
        
        Self {
            state,
            hardware_manager,
            planner,
            kinematics,
            junction_deviation,
            current_position: [0.0, 0.0, 0.0, 0.0],
            step_generator,
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
        
        // Convert Cartesian to motor coordinates using kinematics
        let motor_target = self.kinematics.cartesian_to_motors(&[target[0], target[1], target[2]])?;
        let motor_target_4d = [motor_target[0], motor_target[1], motor_target[2], target_e];
        
        // Determine motion type
        let motion_type = if extrude.is_some() && extrude.unwrap() > 0.0 {
            MotionType::Print
        } else {
            MotionType::Travel
        };
        
        // Plan the move with proper kinematics
        self.planner.plan_move(motor_target_4d, feedrate, motion_type)?;
        
        // Apply junction deviation optimization if there are queued moves
        self.apply_junction_optimization(&motor_target_4d)?;
        
        // Update current position
        self.current_position = target_4d;
        
        // Generate and send steps
        self.generate_and_send_steps(&motor_target_4d).await?;
        
        // Update printer state
        {
            let mut state = self.state.write().await;
            state.position = [target_4d[0], target_4d[1], target_4d[2]];
        }
        
        Ok(())
    }

    /// Apply junction deviation optimization
    fn apply_junction_optimization(&self, target: &[f64; 4]) -> Result<(), Box<dyn std::error::Error>> {
        // Calculate unit vector for this move
        let _unit_vector = JunctionDeviation::calculate_unit_vector(&self.current_position, target);
        
        // In a real implementation, this would optimize the motion queue
        tracing::debug!("Applying junction deviation optimization");
        
        Ok(())
    }

    pub async fn queue_home(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Homing all axes");
        
        // Home in Cartesian space, convert to motor space
        let home_cartesian = [0.0, 0.0, 0.0];
        let home_motors = self.kinematics.cartesian_to_motors(&home_cartesian)?;
        let home_target = [home_motors[0], home_motors[1], home_motors[2], self.current_position[3]];
        
        self.planner.plan_move(
            home_target,
            50.0, // Slow homing speed
            MotionType::Home,
        )?;
        
        // Update position
        self.current_position = [0.0, 0.0, 0.0, self.current_position[3]];
        
        // Send home command to hardware
        let _ = self.hardware_manager.send_command("home_all").await;
        
        // Update printer state
        {
            let mut state = self.state.write().await;
            state.position = [0.0, 0.0, 0.0];
        }
        
        Ok(())
    }

    async fn generate_and_send_steps(&mut self, target: &[f64; 4]) -> Result<(), Box<dyn std::error::Error>> {
        // Generate step commands using the step generator
        let steps = self.step_generator.generate_steps(target);
        
        // Send each step command to hardware
        for step_cmd in steps {
            let mcu_cmd = step_cmd.to_mcu_command();
            let _ = self.hardware_manager.send_command(&mcu_cmd).await;
        }
        
        Ok(())
    }

    pub fn emergency_stop(&mut self) {
        tracing::warn!("Emergency stop activated - clearing motion queue");
        self.planner.clear_queue();
    }

    pub fn get_current_position(&self) -> [f64; 4] {
        self.current_position
    }
}

// Remove the manual Clone implementation since we're using #[derive(Clone)]