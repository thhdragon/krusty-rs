// krusty_shared::motion::adaptive.rs
// Adaptive motion planning logic migrated from krusty_host

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct AdaptiveConfig {
    pub adaptation_rate: f64,
    pub learning_rate: f64,
    pub performance_window: usize,
    pub vibration_threshold: f64,
    pub quality_threshold: f64,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            adaptation_rate: 0.01,
            learning_rate: 0.001,
            performance_window: 100,
            vibration_threshold: 0.02,
            quality_threshold: 0.95,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceData {
    pub timestamp: std::time::Instant,
    pub print_quality: f64,
    pub vibration_level: f64,
    pub position_error: f64,
    pub processing_time: f64,
    pub power_consumption: f64,
    pub print_speed_ratio: f64,
    pub temperature_stability: f64,
}

#[derive(Debug, Clone)]
pub struct OptimizationParams {
    pub junction_deviation: f64,
    pub max_acceleration: [f64; 4],
    pub max_jerk: [f64; 4],
    pub lookahead_distance: usize,
    pub smoothing_factor: f64,
    pub safety_margin: f64,
    pub velocity_limit_factor: f64,
}

impl Default for OptimizationParams {
    fn default() -> Self {
        Self {
            junction_deviation: 0.05,
            max_acceleration: [3000.0, 3000.0, 100.0, 1000.0],
            max_jerk: [15.0, 15.0, 0.5, 2.0],
            lookahead_distance: 32,
            smoothing_factor: 0.1,
            safety_margin: 1.2,
            velocity_limit_factor: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResonancePeak {
    pub frequency: f64,
    pub amplitude: f64,
    pub bandwidth: Option<f64>,
    pub q_factor: Option<f64>,
}

#[derive(Debug)]
pub struct AdaptiveOptimizer {
    performance_history: VecDeque<PerformanceData>,
    optimization_params: OptimizationParams,
    config: AdaptiveConfig,
}

impl AdaptiveOptimizer {
    pub fn new(config: AdaptiveConfig) -> Self {
        Self {
            performance_history: VecDeque::new(),
            optimization_params: OptimizationParams::default(),
            config,
        }
    }

    pub async fn update_with_data(
        &mut self,
        metrics: &PerformanceMetrics,
        vibration_analysis: &VibrationAnalysis,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let performance_data = PerformanceData {
            timestamp: std::time::Instant::now(),
            print_quality: self.calculate_print_quality(metrics, vibration_analysis),
            vibration_level: metrics.avg_vibration,
            position_error: metrics.position_accuracy,
            processing_time: 0.0,
            power_consumption: 0.0,
            print_speed_ratio: metrics.speed_efficiency,
            temperature_stability: metrics.thermal_stability,
        };
        self.performance_history.push_back(performance_data);
        while self.performance_history.len() > self.config.performance_window {
            self.performance_history.pop_front();
        }
        self.adapt_parameters(metrics, vibration_analysis).await?;
        Ok(())
    }

    fn calculate_print_quality(
        &self,
        metrics: &PerformanceMetrics,
        vibration_analysis: &VibrationAnalysis,
    ) -> f64 {
        let vibration_score = (1.0 - (metrics.avg_vibration / 0.1).min(1.0)) * 0.4;
        let accuracy_score = (1.0 - (metrics.position_accuracy / 0.02).min(1.0)) * 0.3;
        let stability_score = metrics.thermal_stability * 0.2;
        let resonance_score = (1.0 - (vibration_analysis.resonance_peaks.len() as f64 / 10.0)).max(0.0) * 0.1;
        (vibration_score + accuracy_score + stability_score + resonance_score).min(1.0)
    }

    async fn adapt_parameters(
        &mut self,
        metrics: &PerformanceMetrics,
        vibration_analysis: &VibrationAnalysis,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let adaptation_rate = self.config.adaptation_rate;
        if metrics.avg_vibration < 0.005 && metrics.position_accuracy < 0.002 {
            self.optimization_params.junction_deviation =
                (self.optimization_params.junction_deviation * (1.0 + adaptation_rate)).min(0.2);
        } else if metrics.avg_vibration > 0.03 {
            self.optimization_params.junction_deviation =
                (self.optimization_params.junction_deviation * (1.0 - adaptation_rate * 0.5)).max(0.01);
        }
        let acceleration_factor = if metrics.quality_score > 0.9 {
            1.0 + adaptation_rate * 0.5
        } else if metrics.quality_score < 0.7 {
            1.0 - adaptation_rate
        } else {
            1.0
        };
        for i in 0..4 {
            self.optimization_params.max_acceleration[i] *= acceleration_factor;
            self.optimization_params.max_acceleration[i] =
                self.optimization_params.max_acceleration[i].max(100.0).min(10000.0);
        }
        if !vibration_analysis.resonance_peaks.is_empty() {
            let avg_resonance = vibration_analysis.resonance_peaks.iter().map(|p| p.frequency).sum::<f64>()
                / vibration_analysis.resonance_peaks.len() as f64;
            for i in 0..4 {
                if avg_resonance > 50.0 {
                    self.optimization_params.max_jerk[i] *= 1.0 - (adaptation_rate * 0.3);
                }
            }
        }
        Ok(())
    }

    pub fn get_optimized_params(&self) -> OptimizationParams {
        self.optimization_params.clone()
    }

    pub fn set_config(&mut self, config: AdaptiveConfig) {
        self.config = config;
    }
}

pub struct AdaptiveMotionPlanner {
    error_predictor: ErrorPredictor,
    vibration_analyzer: VibrationAnalyzer,
    config: AdaptiveConfig,
}

impl AdaptiveMotionPlanner {
    pub fn new(config: AdaptiveConfig) -> Self {
        let _optimizer = AdaptiveOptimizer::new(config.clone());
        Self {
            error_predictor: ErrorPredictor::new(12),
            vibration_analyzer: VibrationAnalyzer::new(1000.0, 1024),
            config,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub avg_vibration: f64,
    pub max_vibration: f64,
    pub vibration_trend: f64,
    pub position_accuracy: f64,
    pub processing_load: f64,
    pub thermal_stability: f64,
    pub speed_efficiency: f64,
    pub quality_score: f64,
}

#[derive(Debug, Clone)]
pub struct TrainingSample {
    pub conditions: Vec<f64>,
    pub actual_error: f64,
    pub predicted_error: f64,
    pub correction_applied: f64,
    pub timestamp: std::time::Instant,
}

#[derive(Debug)]
pub struct PerformanceMonitor {
    buffer: VecDeque<PerformanceMetrics>,
    buffer_size: usize,
}

impl PerformanceMonitor {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(buffer_size),
            buffer_size,
        }
    }
    pub fn update(&mut self, metrics: PerformanceMetrics) {
        if self.buffer.len() == self.buffer_size {
            self.buffer.pop_front();
        }
        self.buffer.push_back(metrics);
    }
    pub fn get_current_metrics(&self) -> Option<&PerformanceMetrics> {
        self.buffer.back()
    }
    pub fn get_trend(&self) -> Option<f64> {
        if self.buffer.len() < 2 {
            return None;
        }
        let first = self.buffer.front()?.avg_vibration;
        let last = self.buffer.back()?.avg_vibration;
        Some((last - first) / self.buffer.len() as f64)
    }
}

pub struct ErrorPredictor {
    samples: VecDeque<TrainingSample>,
    max_samples: usize,
}

impl ErrorPredictor {
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }
    pub fn update_model(&mut self, sample: TrainingSample) {
        if self.samples.len() == self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }
    pub fn predict_error(&self, _conditions: &[f64]) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.samples.iter().map(|s| s.actual_error).sum();
        sum / self.samples.len() as f64
    }
}

#[derive(Debug, Clone)]
pub struct VibrationAnalysis {
    pub frequency_spectrum: Vec<f64>,
    pub resonance_peaks: Vec<ResonancePeak>,
    pub dominant_frequencies: Vec<f64>,
    pub overall_level: f64,
}

#[derive(Debug)]
pub struct VibrationAnalyzer {
    sample_rate: f64,
    analysis_window: usize,
    samples: Vec<f64>,
}

impl VibrationAnalyzer {
    pub fn new(sample_rate: f64, analysis_window: usize) -> Self {
        Self {
            sample_rate,
            analysis_window,
            samples: Vec::with_capacity(analysis_window),
        }
    }
    pub fn add_sample(&mut self, value: f64) {
        if self.samples.len() == self.analysis_window {
            self.samples.remove(0);
        }
        self.samples.push(value);
    }
    pub fn analyze_vibrations(&self) -> VibrationAnalysis {
        VibrationAnalysis {
            frequency_spectrum: vec![],
            resonance_peaks: vec![],
            dominant_frequencies: vec![],
            overall_level: self.samples.iter().map(|x| x.abs()).sum::<f64>() / self.samples.len().max(1) as f64,
        }
    }
}
