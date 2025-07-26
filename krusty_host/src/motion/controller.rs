// src/motion/controller.rs - Motion controller that uses your advanced planners
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::host_os::PrinterState;
use crate::hardware::HardwareManager;
use crate::motion::planner::MotionPlanner;
use crate::motion::planner::adaptive::{PerformanceMonitor, VibrationAnalyzer, PerformanceMetrics, AdaptiveOptimizer, AdaptiveConfig};

#[derive(Debug, Clone)]
// MotionController struct definition follows
pub enum MotionMode {
    Basic,
    Adaptive,
    SnapCrackle,
}

#[derive(Debug)]
pub struct MotionController {
    state: Arc<RwLock<PrinterState>>,
    hardware_manager: HardwareManager,
    mode: MotionMode,
    planner: MotionPlanner, // Only one planner
    adaptive_enabled: bool,
    snap_crackle_enabled: bool,
    current_position: [f64; 4],
    performance_monitor: PerformanceMonitor,
    vibration_analyzer: VibrationAnalyzer,
    adaptive_optimizer: Option<AdaptiveOptimizer>,
}

impl MotionController {
    /// For testing only: force the planner state to Running
    pub fn set_queue_running_for_test(&mut self) {
        self.planner.set_running();
    }
    pub fn get_current_position(&self) -> [f64; 4] {
        self.current_position
    }
    pub fn pause_queue(&mut self) -> Result<(), crate::motion::planner::MotionError> {
        self.planner.pause()
    }

    pub fn resume_queue(&mut self) -> Result<(), crate::motion::planner::MotionError> {
        self.planner.resume()
    }

    pub fn cancel_queue(&mut self) -> Result<(), crate::motion::planner::MotionError> {
        self.planner.cancel()
    }

    pub fn get_queue_state(&self) -> crate::motion::planner::MotionQueueState {
        self.planner.get_state()
    }
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        hardware_manager: HardwareManager,
        mode: MotionMode,
        config: &crate::config::Config,
    ) -> Self {
        let adaptive_optimizer = if matches!(mode, MotionMode::Adaptive) {
            Some(AdaptiveOptimizer::new(AdaptiveConfig::default()))
        } else {
            None
        };
        let mut controller = Self {
            state,
            hardware_manager,
            mode: mode.clone(),
            planner: MotionPlanner::new_from_config(config),
            adaptive_enabled: matches!(mode, MotionMode::Adaptive),
            snap_crackle_enabled: matches!(mode, MotionMode::SnapCrackle),
            current_position: [0.0, 0.0, 0.0, 0.0],
            performance_monitor: PerformanceMonitor::new(100),
            vibration_analyzer: VibrationAnalyzer::new(1000.0, 1024),
            adaptive_optimizer,
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
        
        // Simulate feedback after move (replace with real sensor data when available)
        let simulated_metrics = PerformanceMetrics {
            avg_vibration: 0.01, // 10 microns RMS
            max_vibration: 0.05, // 50 microns peak
            vibration_trend: -0.001,
            position_accuracy: 0.005,
            processing_load: 0.3,
            thermal_stability: 0.95,
            speed_efficiency: 0.85,
            quality_score: 0.92,
        };
        self.performance_monitor.update(simulated_metrics.clone());
        self.vibration_analyzer.add_sample(simulated_metrics.avg_vibration);
        // If adaptive mode, update optimizer and planner parameters
        if let Some(opt) = &mut self.adaptive_optimizer {
            let vibration_analysis = self.vibration_analyzer.analyze_vibrations();
            opt.update_with_data(&simulated_metrics, &vibration_analysis).await?;
            let params = opt.get_optimized_params();
            // Update planner config with optimized parameters
            self.planner.set_max_acceleration(params.max_acceleration);
            self.planner.set_max_jerk(params.max_jerk);
            self.planner.set_junction_deviation(params.junction_deviation);
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
        // Optionally, update feedback here as well if needed
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