//! Hardware-agnostic MotionController for shared use by host and simulator
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::motion::{MotionMode};
use crate::motion::planner::MotionPlanner;
use crate::motion::adaptive::{PerformanceMonitor, VibrationAnalyzer, PerformanceMetrics, AdaptiveOptimizer, AdaptiveConfig};
use crate::trajectory::{MotionError, MotionQueueState, MotionType};

/// Abstract trait for printer state (host/sim must implement)
pub trait PrinterStateTrait: Send + Sync {
    fn set_position(&mut self, pos: [f64; 3]);
}

/// Abstract trait for hardware manager (host/sim must implement)
#[async_trait::async_trait]
pub trait HardwareManagerTrait: Send + Sync {
    async fn send_command(&mut self, command: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync + 'static>>;
}

/// Hardware-agnostic motion controller
pub struct MotionController<S: PrinterStateTrait, H: HardwareManagerTrait> {
    state: Arc<RwLock<S>>,
    hardware_manager: Arc<RwLock<H>>,
    mode: MotionMode,
    planner: MotionPlanner, // Only one planner
    adaptive_enabled: bool,
    snap_crackle_enabled: bool,
    current_position: [f64; 4],
    performance_monitor: PerformanceMonitor,
    vibration_analyzer: VibrationAnalyzer,
    adaptive_optimizer: Option<AdaptiveOptimizer>,
}

impl<S: PrinterStateTrait, H: HardwareManagerTrait> MotionController<S, H> {
    pub fn new(
        state: Arc<RwLock<S>>,
        hardware_manager: Arc<RwLock<H>>,
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
            }
            MotionMode::Adaptive => {
                self.adaptive_enabled = true;
                self.snap_crackle_enabled = false;
            }
            MotionMode::SnapCrackle => {
                self.adaptive_enabled = false;
                self.snap_crackle_enabled = true;
            }
        }
    }

    pub async fn queue_linear_move(
        &mut self,
        target: [f64; 3],
        feedrate: Option<f64>,
        extrude: Option<f64>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let current_e = self.current_position[3];
        let target_e = if let Some(e) = extrude {
            current_e + e
        } else {
            current_e
        };
        let feedrate = feedrate.unwrap_or(300.0);
        let target_4d = [target[0], target[1], target[2], target_e];
        if self.snap_crackle_enabled {
            self.planner.plan_move(target_4d, feedrate, MotionType::Print).await?;
        } else if self.adaptive_enabled {
            self.planner.plan_move(target_4d, feedrate, MotionType::Print).await?;
        } else {
            self.planner.plan_move(target_4d, feedrate, MotionType::Print).await?;
        }
        let simulated_metrics = PerformanceMetrics {
            avg_vibration: 0.01,
            max_vibration: 0.05,
            vibration_trend: -0.001,
            position_accuracy: 0.005,
            processing_load: 0.3,
            thermal_stability: 0.95,
            speed_efficiency: 0.85,
            quality_score: 0.92,
        };
        self.performance_monitor.update(simulated_metrics.clone());
        self.vibration_analyzer.add_sample(simulated_metrics.avg_vibration);
        if let Some(opt) = &mut self.adaptive_optimizer {
            let vibration_analysis = self.vibration_analyzer.analyze_vibrations();
            opt.update_with_data(&simulated_metrics, &vibration_analysis).await?;
            let params = opt.get_optimized_params();
            self.planner.set_max_acceleration(params.max_acceleration);
            self.planner.set_max_jerk(params.max_jerk);
            self.planner.set_junction_deviation(params.junction_deviation);
        }
        self.current_position = target_4d;
        {
            let mut state = self.state.write().await;
            state.set_position([target_4d[0], target_4d[1], target_4d[2]]);
        }
        Ok(())
    }

    pub async fn queue_home(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let home_target = [0.0, 0.0, 0.0, self.current_position[3]];
        self.planner.plan_move(home_target, 50.0, MotionType::Home).await?;
        self.current_position = home_target;
        let mut hw = self.hardware_manager.write().await;
        let _ = hw.send_command("home_all").await;
        {
            let mut state = self.state.write().await;
            state.set_position([0.0, 0.0, 0.0]);
        }
        Ok(())
    }

    pub async fn update(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        self.planner.update().await?;
        Ok(())
    }

    pub fn emergency_stop(&mut self) {
        self.planner.clear_queue();
    }

    pub fn switch_mode(&mut self, new_mode: MotionMode) {
        self.mode = new_mode.clone();
        self.configure_features();
    }

    pub fn get_queue_length(&self) -> usize {
        self.planner.queue_length()
    }
}
