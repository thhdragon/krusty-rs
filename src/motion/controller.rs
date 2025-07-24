// src/motion/controller.rs - Motion controller that uses your advanced planners
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::PrinterState;
use crate::hardware::HardwareManager;
use crate::motion::planner::MotionPlanner;

#[derive(Debug, Clone)]
pub enum MotionMode {
    Basic,
    Adaptive,
    SnapCrackle,
}

pub struct MotionController {
    state: Arc<RwLock<PrinterState>>,
    hardware_manager: HardwareManager,
    mode: MotionMode,
    planner: MotionPlanner, // Only one planner
    adaptive_enabled: bool,
    snap_crackle_enabled: bool,
    current_position: [f64; 4],
}

impl MotionController {
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        hardware_manager: HardwareManager,
        mode: MotionMode,
    ) -> Self {
        let mut controller = Self {
            state,
            hardware_manager,
            mode: mode.clone(),
            planner: MotionPlanner::new(crate::motion::planner::MotionConfig::default()),
            adaptive_enabled: matches!(mode, MotionMode::Adaptive),
            snap_crackle_enabled: matches!(mode, MotionMode::SnapCrackle),
            current_position: [0.0, 0.0, 0.0, 0.0],
        };
        controller.configure_features();
        controller
    }

    fn configure_features(&mut self) {
        match self.mode {
            MotionMode::Basic => {
                self.adaptive_enabled = false;
                self.snap_crackle_enabled = false;
                tracing::info!("Initialized basic motion planner");
            }
            MotionMode::Adaptive => {
                self.adaptive_enabled = true;
                self.snap_crackle_enabled = false;
                tracing::info!("Enabled adaptive feature on motion planner");
            }
            MotionMode::SnapCrackle => {
                self.adaptive_enabled = false;
                self.snap_crackle_enabled = true;
                tracing::info!("Enabled snap/crackle feature on motion planner");
            }
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
        // Use feature flags to determine planner behavior
        if self.snap_crackle_enabled {
            // Call snap/crackle logic as a layer on top of planner
            // e.g., self.planner.apply_snap_crackle(...)
            self.planner.plan_move(target_4d, feedrate, crate::motion::planner::MotionType::Print).await?;
        } else if self.adaptive_enabled {
            // Call adaptive logic as a layer on top of planner
            // e.g., self.planner.apply_adaptive(...)
            self.planner.plan_move(target_4d, feedrate, crate::motion::planner::MotionType::Print).await?;
        } else {
            // Basic planner logic
            self.planner.plan_move(target_4d, feedrate, crate::motion::planner::MotionType::Print).await?;
        }
        
        // Update current position
        self.current_position = target_4d;
        
        // Update printer state
        {
            let mut state = self.state.write().await;
            state.position = [target_4d[0], target_4d[1], target_4d[2]];
        }
        
        Ok(())
    }

    pub async fn queue_home(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Homing all axes");
        let home_target = [0.0, 0.0, 0.0, self.current_position[3]];
        if self.snap_crackle_enabled {
            tracing::info!("Homing with snap/crackle feature enabled");
            // Insert snap/crackle-specific logic here if needed
            self.planner.plan_move(home_target, 50.0, crate::motion::planner::MotionType::Home).await?;
        } else if self.adaptive_enabled {
            tracing::info!("Homing with adaptive feature enabled");
            // Insert adaptive-specific logic here if needed
            self.planner.plan_move(home_target, 50.0, crate::motion::planner::MotionType::Home).await?;
        } else {
            self.planner.plan_move(home_target, 50.0, crate::motion::planner::MotionType::Home).await?;
        }
        self.current_position = home_target;
        let _ = self.hardware_manager.send_command("home_all").await;
        {
            let mut state = self.state.write().await;
            state.position = [0.0, 0.0, 0.0];
        }
        Ok(())
    }

    pub async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Update the planner, with feature logic if needed
        if self.snap_crackle_enabled {
            // Insert snap/crackle update logic here if needed
            self.planner.update().await?;
        } else if self.adaptive_enabled {
            // Insert adaptive update logic here if needed
            self.planner.update().await?;
        } else {
            self.planner.update().await?;
        }
        Ok(())
    }

    pub fn emergency_stop(&mut self) {
        tracing::warn!("Emergency stop activated");
        self.planner.clear_queue();
        // Insert additional feature-specific emergency logic if needed
    }

    pub fn switch_mode(&mut self, new_mode: MotionMode) {
        self.mode = new_mode.clone();
        self.configure_features();
        tracing::info!("Switched to {:?} motion mode", new_mode);
    }

    pub fn get_queue_length(&self) -> usize {
        self.planner.queue_length()
    }
}