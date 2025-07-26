#[derive(Debug, Clone, PartialEq)]
pub enum MotionQueueState {
    Idle,
    Running,
    Paused,
    Cancelled,
}
// # Motion Planner: Config-Driven Shaper and Blending Assignment
//
// This module integrates advanced motion planning with per-axis input shaper and blending configuration.
//
// ## Usage Example
//
// 1. Define your shaper/blending config in TOML (see `config.rs` docs).
// 2. Parse your config and construct the planner:
//
// use crate::config::Config;
// use crate::motion::planner::MotionPlanner;
// let config: Config = toml::from_str(toml_str).unwrap();
// let planner = MotionPlanner::new_from_config(&config);
// // The planner will assign the correct shaper to each axis at runtime.
//
// - Extendable: Add new shaper types or parameters by updating the config schema and Rust enums.
// - See also: `src/config.rs` for config structs and validation, `src/motion/shaper.rs` for shaper implementations.

// src/motion/planner/mod.rs

use std::collections::VecDeque;
use crate::config::Config;
use crate::motion::kinematics::{Kinematics, create_kinematics};
use crate::motion::junction::JunctionDeviation;
use crate::motion::kinematics::KinematicsType;
use thiserror::Error;
use krusty_shared::shaper::{InputShaperTrait, PerAxisInputShapers, InputShaperType, ZVDShaper, SineWaveShaper};

#[derive(Debug, Error)]
pub enum MotionError {
    #[error("Junction deviation error: {0}")]
    JunctionDeviation(String),
    #[error("Kinematics error: {0}")]
    Kinematics(String),
    #[error("Other: {0}")]
    Other(String),
}

// Re-export shared types if they were defined here or move them here.
// Use shared MotionType from krusty_shared
use krusty_shared::trajectory::MotionType;

#[derive(Debug, Clone)]
pub struct MotionConfig {
    pub max_velocity: [f64; 4],
    pub max_acceleration: [f64; 4],
    pub max_jerk: [f64; 4],
    pub junction_deviation: f64,
    pub axis_limits: [[f64; 2]; 3],
    pub kinematics_type: KinematicsType,
    pub minimum_step_distance: f64,
    pub lookahead_buffer_size: usize,
}

fn parse_kinematics_type(s: &str) -> KinematicsType {
    match s.to_lowercase().as_str() {
        "cartesian" => KinematicsType::Cartesian,
        "corexy" => KinematicsType::CoreXY,
        "delta" => KinematicsType::Delta,
        "hangprinter" => KinematicsType::Hangprinter,
        _ => KinematicsType::Cartesian,
    }
}

impl MotionConfig {
    pub fn new_from_config(config: &Config) -> Self {
        // Simplified implementation based on your config structure
        Self {
            max_velocity: [config.printer.max_velocity, config.printer.max_velocity, config.printer.max_z_velocity, 50.0], // Example E max vel
            max_acceleration: [config.printer.max_accel, config.printer.max_accel, config.printer.max_z_accel, 1000.0], // Example E max accel
            max_jerk: [20.0, 20.0, 0.5, 2.0], // Example jerk values
            junction_deviation: 0.05, // mm
            axis_limits: [[0.0, 200.0], [0.0, 200.0], [0.0, 200.0]], // Example limits, should come from config ideally
            kinematics_type: parse_kinematics_type(&config.printer.kinematics),
            minimum_step_distance: 0.001, // mm
            lookahead_buffer_size: 16,
        }
    }
}

impl Default for MotionConfig {
    fn default() -> Self {
        Self {
            max_velocity: [100.0, 100.0, 10.0, 50.0],
            max_acceleration: [1000.0, 1000.0, 100.0, 1000.0],
            max_jerk: [20.0, 20.0, 0.5, 2.0],
            junction_deviation: 0.05,
            axis_limits: [[0.0, 200.0], [0.0, 200.0], [0.0, 200.0]],
            kinematics_type: KinematicsType::Cartesian,
            minimum_step_distance: 0.001,
            lookahead_buffer_size: 16,
        }
    }
}

// Rename MotionBlock to MotionSegment for consistency or keep MotionBlock
#[derive(Debug, Clone)]
pub struct MotionSegment { // Was MotionBlock
    pub target: [f64; 4],
    pub feedrate: f64,
    pub limited_feedrate: f64, // Will be calculated
    pub distance: f64,
    pub duration: f64, // Will be calculated
    pub acceleration: f64, // Will be calculated or placeholder
    pub entry_speed: f64, // Placeholder for now
    pub exit_speed: f64, // Placeholder for now
    pub motion_type: MotionType,
}

// Internal state for the planner
#[derive(Debug, Clone)]
struct PlannerState {
    active: bool,
    current_segment: Option<MotionSegment>,
    segment_time: f64,
    last_update: std::time::Instant,
}

pub struct MotionPlanner {
    config: MotionConfig, // Store the config!
    current_position: [f64; 4],
    motion_queue: VecDeque<MotionSegment>,
    planner_state: PlannerState,
    kinematics: Box<dyn Kinematics + Send + Sync>,
    junction_deviation: JunctionDeviation,
    pub input_shapers: PerAxisInputShapers, // Per-axis shapers (enum-based)
    state: MotionQueueState,
}

impl MotionPlanner {
    pub fn new(config: MotionConfig) -> Self {
        // Prefer config-driven construction; use new_from_config for all planner creation
        // This function is retained for compatibility but delegates to new_from_config
        // You must pass a Config (not just MotionConfig) to use config-driven shaper/blending
        panic!("Use MotionPlanner::new_from_config(&Config) for config-driven planner construction");
    }

    pub fn new_from_config(config: &Config) -> Self {
        let planner_config = MotionConfig::new_from_config(config);
        // --- Input shaper integration ---
        let mut input_shapers = PerAxisInputShapers::new(4); // X, Y, Z, E
        if let Some(motion_cfg) = &config.motion {
            for (axis_name, shaper_cfg) in &motion_cfg.shaper {
                let axis_idx = match axis_name.as_str() {
                    "x" | "X" => 0,
                    "y" | "Y" => 1,
                    "z" | "Z" => 2,
                    "e" | "E" => 3,
                    _ => continue, // Ignore unknown axes
                };
                let shaper = match shaper_cfg.r#type {
                    crate::config::ShaperType::Zvd => {
                        // Example: ZVDShaper with delay and coeffs (stub, real params TBD)
                        let delay = 1;
                        let coeffs = [1.0, 0.0];
                        InputShaperType::ZVD(ZVDShaper::new(delay, coeffs))
                    }
                    crate::config::ShaperType::Sine => {
                        // Map config fields to SineWaveShaper
                        let magnitude = 1.0; // Could be configurable
                        let frequency = shaper_cfg.frequency as f64;
                        let sample_time = 0.01; // Example value
                        InputShaperType::SineWave(SineWaveShaper::new(magnitude, frequency, sample_time))
                    }
                };
                input_shapers.set_shaper(axis_idx, shaper);
            }
        }
        Self {
            config: planner_config.clone(),
            current_position: [0.0, 0.0, 0.0, 0.0],
            motion_queue: VecDeque::new(),
            planner_state: PlannerState {
                active: false,
                current_segment: None,
                segment_time: 0.0,
                last_update: std::time::Instant::now(),
            },
            kinematics: create_kinematics(
                planner_config.kinematics_type,
                planner_config.axis_limits,
            ),
            junction_deviation: JunctionDeviation::new(planner_config.junction_deviation),
            input_shapers,
            state: MotionQueueState::Idle,
        }
    }

    pub fn pause(&mut self) -> Result<(), MotionError> {
        match self.state {
            MotionQueueState::Running => {
                self.state = MotionQueueState::Paused;
                tracing::info!("Motion queue paused");
                Ok(())
            }
            MotionQueueState::Paused => Err(MotionError::Other("Queue is already paused".to_string())),
            MotionQueueState::Idle => Err(MotionError::Other("Queue is idle, nothing to pause".to_string())),
            MotionQueueState::Cancelled => Err(MotionError::Other("Queue is cancelled, cannot pause".to_string())),
        }
    }

    pub fn resume(&mut self) -> Result<(), MotionError> {
        match self.state {
            MotionQueueState::Paused => {
                self.state = MotionQueueState::Running;
                tracing::info!("Motion queue resumed");
                Ok(())
            }
            MotionQueueState::Running => Err(MotionError::Other("Queue is already running".to_string())),
            MotionQueueState::Idle => Err(MotionError::Other("Queue is idle, nothing to resume".to_string())),
            MotionQueueState::Cancelled => Err(MotionError::Other("Queue is cancelled, cannot resume".to_string())),
        }
    }

    pub fn cancel(&mut self) -> Result<(), MotionError> {
        match self.state {
            MotionQueueState::Cancelled => Err(MotionError::Other("Queue is already cancelled".to_string())),
            _ => {
                self.state = MotionQueueState::Cancelled;
                self.clear_queue();
                tracing::warn!("Motion queue cancelled and cleared");
                Ok(())
            }
        }
    }

    pub fn set_running(&mut self) {
        self.state = MotionQueueState::Running;
    }

    pub fn get_state(&self) -> MotionQueueState {
        self.state.clone()
    }

    pub fn set_input_shaper(&mut self, axis: usize, shaper: InputShaperType) {
        self.input_shapers.set_shaper(axis, shaper);
    }

    pub async fn plan_move(
        &mut self,
        target: [f64; 4],
        feedrate: f64,
        motion_type: MotionType,
    ) -> Result<(), MotionError> {
        let distance = self.calculate_distance(&self.current_position, &target);
        if distance < self.config.minimum_step_distance {
            return Ok(());
        }
        let limited_feedrate = self.limit_feedrate_by_acceleration(&target, feedrate);
        let mut segment = MotionSegment {
            target,
            feedrate,
            limited_feedrate,
            distance,
            duration: distance / limited_feedrate.max(0.1),
            acceleration: self.calculate_acceleration(&target),
            entry_speed: self.motion_queue.back().map_or(0.0, |prev| prev.exit_speed),
            exit_speed: limited_feedrate, // Start with max, will be optimized in passes
            motion_type,
        };
        self.apply_junction_deviation(&mut segment)?;
        self.motion_queue.push_back(segment);
        self.current_position = target;
        if self.motion_queue.len() >= self.config.lookahead_buffer_size / 2 {
            self.replan_queue().await?;
        }
        Ok(())
    }

    /// Limit feedrate based on acceleration capabilities
    fn limit_feedrate_by_acceleration(&self, target: &[f64; 4], requested_feedrate: f64) -> f64 {
        let distance = self.calculate_distance(&self.current_position, target);
        if distance == 0.0 {
            return requested_feedrate;
        }
        let unit_vector = self.calculate_unit_vector(&self.current_position, target);
        // Find limiting acceleration for each axis
        let mut max_acceleration = f64::INFINITY;
        for i in 0..4 {
            let axis_component = unit_vector[i].abs();
            if axis_component > 0.0 {
                let axis_accel_limit = self.config.max_acceleration[i] / axis_component;
                max_acceleration = max_acceleration.min(axis_accel_limit);
            }
        }
        // Convert acceleration limit to velocity limit
        let acceleration_limited_feedrate = (2.0 * max_acceleration * distance).sqrt();
        requested_feedrate.min(acceleration_limited_feedrate).max(0.1)
    }

    /// Apply junction deviation optimization
    fn apply_junction_deviation(&self, segment: &mut MotionSegment) -> Result<(), MotionError> {
        // Calculate unit vector for this move
        let unit_vector = self.calculate_unit_vector(&self.current_position, &segment.target);
        // If there's a previous segment, calculate junction speed
        if let Some(prev_segment) = self.motion_queue.back() {
            let prev_unit_vector = self.calculate_unit_vector(&self.current_position, &prev_segment.target);
            let junction_speed = self.junction_deviation.calculate_junction_speed(
                &prev_unit_vector,
                &unit_vector,
                segment.acceleration,
            );
            // Limit entry speed by junction deviation
            segment.entry_speed = segment.entry_speed.min(junction_speed);
        }
        Ok(())
    }

    /// Replan motion queue for optimal performance
    async fn replan_queue(&mut self) -> Result<(), MotionError> {
        let queue_len = self.motion_queue.len();
        if queue_len < 2 {
            return Ok(());
        }
        tracing::debug!("Replanning {} motion segments", queue_len);
        // Convert to vector for easier manipulation
        let mut segments: Vec<MotionSegment> = self.motion_queue.drain(..).collect();
        // Forward pass: calculate entry/exit speeds
        self.forward_pass(&mut segments)?;
        // Backward pass: optimize speeds
        self.backward_pass(&mut segments)?;
        // Recalculate durations with optimized speeds
        self.recalculate_durations(&mut segments)?;
        // Put optimized segments back in queue
        for segment in segments {
            self.motion_queue.push_back(segment);
        }
        Ok(())
    }

    /// Forward pass through motion segments
    fn forward_pass(&mut self, segments: &mut [MotionSegment]) -> Result<(), MotionError> {
        let mut previous_unit_vector = None;
        for i in 0..segments.len() {
            // Calculate unit vector for this move
            let unit_vector = if i == 0 {
                self.calculate_unit_vector(&self.current_position, &segments[i].target)
            } else {
                self.calculate_unit_vector(&segments[i-1].target, &segments[i].target)
            };
            // Calculate junction speed if we have previous vector
            if let Some(prev_unit) = previous_unit_vector {
                let junction_speed = self.junction_deviation.calculate_junction_speed(
                    &prev_unit,
                    &unit_vector,
                    segments[i].acceleration,
                );
                // Limit entry speed by junction deviation
                segments[i].entry_speed = segments[i].entry_speed.min(junction_speed);
            }
            // Calculate maximum exit speed based on acceleration and distance
            let max_exit_speed = ((segments[i].entry_speed * segments[i].entry_speed) + 
                                 2.0 * segments[i].acceleration * segments[i].distance).sqrt();
            segments[i].exit_speed = segments[i].limited_feedrate.min(max_exit_speed);
            // Update for next iteration
            previous_unit_vector = Some(unit_vector);
        }
        Ok(())
    }

    /// Backward pass through motion segments
    fn backward_pass(&mut self, segments: &mut [MotionSegment]) -> Result<(), MotionError> {
        for i in (0..segments.len()).rev() {
            let exit_speed = if i == segments.len() - 1 {
                0.0 // Last segment stops
            } else {
                segments[i + 1].entry_speed
            };
            // Calculate maximum entry speed based on exit speed and deceleration
            let max_entry_speed = ((exit_speed * exit_speed) + 
                                  2.0 * segments[i].acceleration * segments[i].distance).sqrt();
            // Limit entry speed
            segments[i].entry_speed = segments[i].entry_speed.min(max_entry_speed);
            // Recalculate exit speed based on limited entry speed
            segments[i].exit_speed = ((segments[i].entry_speed * segments[i].entry_speed) + 
                                     2.0 * segments[i].acceleration * segments[i].distance).sqrt()
                                     .min(segments[i].limited_feedrate);
        }
        Ok(())
    }

    /// Recalculate segment durations with optimized speeds
    fn recalculate_durations(&mut self, segments: &mut [MotionSegment]) -> Result<(), MotionError> {
        for segment in segments {
            // For trapezoidal profile: t = (v_end - v_start + sqrt((v_end - v_start)² + 2*a*d)) / a
            let delta_v = segment.exit_speed - segment.entry_speed;
            let discriminant = delta_v * delta_v + 2.0 * segment.acceleration * segment.distance;
            if discriminant >= 0.0 {
                let time = (delta_v + discriminant.sqrt()) / segment.acceleration;
                segment.duration = time.max(0.0);
            } else {
                // Fallback - should not happen with proper optimization
                segment.duration = segment.distance / segment.limited_feedrate;
            }
        }
        Ok(())
    }

    /// Calculate unit vector between two positions
    fn calculate_unit_vector(&self, start: &[f64; 4], end: &[f64; 4]) -> [f64; 4] {
        let delta = [
            end[0] - start[0],
            end[1] - start[1],
            end[2] - start[2],
            end[3] - start[3],
        ];
        let distance = (delta[0] * delta[0] + delta[1] * delta[1] + 
                       delta[2] * delta[2] + delta[3] * delta[3]).sqrt();
        if distance > 0.0 {
            [
                delta[0] / distance,
                delta[1] / distance,
                delta[2] / distance,
                delta[3] / distance,
            ]
        } else {
            [0.0, 0.0, 0.0, 0.0]
        }
    }

    /// Calculate 3D Euclidean distance
    fn calculate_distance(&self, start: &[f64; 4], end: &[f64; 4]) -> f64 {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let de = end[3] - start[3];
        (dx * dx + dy * dy + dz * dz + de * de).sqrt()
    }

    /// Calculate appropriate acceleration for a move
    fn calculate_acceleration(&self, target: &[f64; 4]) -> f64 {
        let distance = self.calculate_distance(&self.current_position, target);
        if distance == 0.0 {
            return self.config.max_acceleration[0];
        }
        let unit_vector = self.calculate_unit_vector(&self.current_position, target);
        // Weighted average based on axis movement
        let weighted_accel = 
            unit_vector[0].abs() * self.config.max_acceleration[0] +
            unit_vector[1].abs() * self.config.max_acceleration[1] +
            unit_vector[2].abs() * self.config.max_acceleration[2] +
            unit_vector[3].abs() * self.config.max_acceleration[3];
        weighted_accel
    }

    /// Main update function - called at high frequency
    pub async fn update(&mut self) -> Result<(), MotionError> {
        // Respect queue state
        match self.state {
            MotionQueueState::Paused => {
                // Do nothing, hold current segment/queue
                return Ok(());
            }
            MotionQueueState::Cancelled => {
                // Already cleared, set to Idle
                self.state = MotionQueueState::Idle;
                return Ok(());
            }
            MotionQueueState::Idle => {
                // Do nothing if idle
                return Ok(());
            }
            MotionQueueState::Running => {
                // Continue as normal
            }
        }
        let now = std::time::Instant::now();
        let dt = (now - self.planner_state.last_update).as_secs_f64();
        self.planner_state.last_update = now;
        // Check if we need to start a new segment
        if self.planner_state.current_segment.is_none() {
            if let Some(segment) = self.motion_queue.pop_front() {
                self.planner_state.current_segment = Some(segment);
                self.planner_state.segment_time = 0.0;
                self.planner_state.active = true;
                self.state = MotionQueueState::Running;
            } else {
                self.planner_state.active = false;
                self.state = MotionQueueState::Idle;
                return Ok(());
            }
        }
        // Process current segment
        let mut clear_segment = false;
        if let Some(ref mut segment) = self.planner_state.current_segment {
            self.planner_state.segment_time += dt;
            // Check if segment is complete
            if self.planner_state.segment_time >= segment.duration {
                // Segment complete - update position
                self.current_position = segment.target;
                clear_segment = true;
                tracing::debug!(
                    "Completed move to [{:.3}, {:.3}, {:.3}, {:.3}]",
                    segment.target[0], segment.target[1], segment.target[2], segment.target[3]
                );
            } else {
                // Interpolate position within segment
                let progress = self.planner_state.segment_time / segment.duration;
                // Simple linear interpolation (in advanced version, use proper motion profiles)
                let current_pos = [
                    self.current_position[0] + (segment.target[0] - self.current_position[0]) * progress,
                    self.current_position[1] + (segment.target[1] - self.current_position[1]) * progress,
                    self.current_position[2] + (segment.target[2] - self.current_position[2]) * progress,
                    self.current_position[3] + (segment.target[3] - self.current_position[3]) * progress,
                ];
                // Copy segment for use after borrow ends
                let segment_copy = segment.clone();
                let _ = segment; // End mutable borrow
                // Generate steps for current position
                self.generate_steps(&current_pos, &segment_copy).await?;
            }
        }
        if clear_segment {
            self.planner_state.current_segment = None;
        }
        Ok(())
    }

    /// Generate step commands for current position
    async fn generate_steps(
        &mut self,
        position: &[f64; 4],
        segment: &MotionSegment,
    ) -> Result<(), MotionError> {
        // Apply input shaping per axis using PerAxisInputShapers
        let mut shaped_position = [0.0; 4];
        for i in 0..4 {
            shaped_position[i] = self.input_shapers.do_step(i, position[i]);
        }
        // Convert Cartesian position to motor positions
        let cartesian = [shaped_position[0], shaped_position[1], shaped_position[2]];
        let motor_positions = self.kinematics.cartesian_to_motors(&cartesian)
            .map_err(|e| MotionError::Other(e.to_string()))?;
        tracing::trace!(
            "Position: [{:.3}, {:.3}, {:.3}, {:.3}] Motors: [{:.3}, {:.3}, {:.3}, {:.3}] Segment: {:?}",
            shaped_position[0], shaped_position[1], shaped_position[2], shaped_position[3],
            motor_positions[0], motor_positions[1], motor_positions[2], motor_positions[3],
            segment
        );
        Ok(())
    }

    pub fn clear_queue(&mut self) {
        self.motion_queue.clear();
        self.planner_state.current_segment = None;
        self.planner_state.segment_time = 0.0;
        tracing::warn!("Motion queue cleared");
    }

    pub fn queue_length(&self) -> usize {
        self.motion_queue.len()
    }

    pub fn is_active(&self) -> bool {
        self.planner_state.active
    }

    pub fn get_current_position(&self) -> [f64; 4] {
        self.current_position
    }

    // Add other necessary methods like set_position, home, etc.
    pub fn set_position(&mut self, position: [f64; 4]) {
        self.current_position = position;
        tracing::debug!("Planner position set to {:?}", position);
        // If there's a current segment, this might need more careful handling depending on sync strategy.
    }

    pub fn set_max_acceleration(&mut self, max_accel: [f64; 4]) {
        self.config.max_acceleration = max_accel;
    }
    pub fn set_max_jerk(&mut self, max_jerk: [f64; 4]) {
        self.config.max_jerk = max_jerk;
    }
    pub fn set_junction_deviation(&mut self, jd: f64) {
        self.config.junction_deviation = jd;
        self.junction_deviation = crate::motion::junction::JunctionDeviation::new(jd);
    }

    pub fn lookahead_buffer_size(&self) -> usize {
        self.config.lookahead_buffer_size
    }
}

pub mod adaptive;

/// Configuration for advanced motion features (input shapers, blending)
#[derive(Debug, Clone)]
pub struct AdvancedMotionConfig {
    pub input_shapers: Option<Vec<Option<InputShaperType>>>, // Per-axis
    pub bezier_blending: Option<BezierBlendingConfig>,
}

#[derive(Debug, Clone)]
pub struct BezierBlendingConfig {
    pub enabled: bool,
    pub degree: usize,
    pub max_deviation: f64,
}

impl Default for AdvancedMotionConfig {
    fn default() -> Self {
        Self {
            input_shapers: None, // No shapers by default
            bezier_blending: Some(BezierBlendingConfig {
                enabled: false,
                degree: 15,
                max_deviation: 0.5,
            }),
        }
    }
}

// Example: Accept AdvancedMotionConfig in MotionPlanner::new
// and use it to configure input shapers and blending
// (Stub for future config file/API integration)

// Implement Clone manually if needed and direct fields support it
impl Clone for MotionPlanner {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            current_position: self.current_position,
            motion_queue: self.motion_queue.clone(),
            planner_state: self.planner_state.clone(),
            kinematics: self.kinematics.clone_box(),
            junction_deviation: self.junction_deviation.clone(),
            input_shapers: self.input_shapers.clone(), // Properly clone PerAxisInputShapers
            state: self.state.clone(),
        }
    }
}

// Implement Debug manually if needed due to non-Debug fields
impl std::fmt::Debug for MotionPlanner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MotionPlanner")
            .field("current_position", &self.current_position)
            .field("motion_queue_len", &self.motion_queue.len())
            .field("planner_state", &self.planner_state)
            .field("kinematics", &"Box<dyn Kinematics + Send>")
            .field("junction_deviation", &self.junction_deviation)
            .field("state", &self.state)
            .finish()
    }
}
