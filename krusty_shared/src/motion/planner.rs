// krusty_shared::motion::planner.rs
// Core motion planning logic migrated from krusty_host

use std::collections::VecDeque;
use crate::config::Config;
use crate::Kinematics;
use crate::shaper::{PerAxisInputShapers, InputShaperType, ZVDShaper, SineWaveShaper};
use crate::trajectory::{MotionQueueState, MotionError, MotionConfig, MotionType};
use crate::MotionConfigExt;

pub struct MotionPlanner {
    config: MotionConfig, // Store the config!
    current_position: [f64; 4],
    motion_queue: VecDeque<MotionSegment>,
    planner_state: PlannerState,
    kinematics: Box<dyn Kinematics + Send + Sync>,
    pub input_shapers: PerAxisInputShapers, // Per-axis shapers (enum-based)
    state: MotionQueueState,
}

#[derive(Debug, Clone)]
pub struct MotionSegment {
    pub target: [f64; 4],
    pub feedrate: f64,
    pub limited_feedrate: f64,
    pub distance: f64,
    pub duration: f64,
    pub acceleration: f64,
    pub entry_speed: f64,
    pub exit_speed: f64,
    pub motion_type: MotionType,
}

#[derive(Debug, Clone)]
pub struct PlannerState {
    pub active: bool,
    pub current_segment: Option<MotionSegment>,
    pub segment_time: f64,
    pub last_update: std::time::Instant,
}

impl MotionPlanner {
    pub fn new(_config: MotionConfig) -> Self {
        panic!("Use MotionPlanner::new_from_config(&Config) for config-driven planner construction");
    }

    pub fn new_from_config(config: &Config) -> Self {
        let planner_config = MotionConfig::new_from_config(config);
        let mut input_shapers = PerAxisInputShapers::new(4);
        if let Some(motion_cfg) = &config.motion {
            for (axis_name, shaper_cfg) in &motion_cfg.shaper {
                let axis_idx = match axis_name.as_str() {
                    "x" | "X" => 0,
                    "y" | "Y" => 1,
                    "z" | "Z" => 2,
                    "e" | "E" => 3,
                    _ => continue,
                };
                let shaper = match shaper_cfg.r#type {
                    crate::config::ShaperType::Zvd => {
                        let delay = 1;
                        let coeffs = [1.0, 0.0];
                        InputShaperType::ZVD(ZVDShaper::new(delay, coeffs))
                    }
                    crate::config::ShaperType::Sine => {
                        let magnitude = 1.0;
                        let frequency = shaper_cfg.frequency as f64;
                        let sample_time = 0.01;
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
            kinematics: todo!("Implement kinematics instantiation using shared crate or local logic"),
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
        let segment = MotionSegment {
            target,
            feedrate,
            limited_feedrate,
            distance,
            duration: distance / limited_feedrate.max(0.1),
            acceleration: self.calculate_acceleration(&target),
            entry_speed: self.motion_queue.back().map_or(0.0, |prev| prev.exit_speed),
            exit_speed: limited_feedrate,
            motion_type,
        };
        self.motion_queue.push_back(segment);
        self.current_position = target;
        if self.motion_queue.len() >= self.config.lookahead_buffer_size / 2 {
            self.replan_queue().await?;
        }
        Ok(())
    }

    fn limit_feedrate_by_acceleration(&self, target: &[f64; 4], requested_feedrate: f64) -> f64 {
        let distance = self.calculate_distance(&self.current_position, target);
        if distance == 0.0 {
            return requested_feedrate;
        }
        let unit_vector = self.calculate_unit_vector(&self.current_position, target);
        let mut max_acceleration = f64::INFINITY;
        for i in 0..4 {
            let axis_component = unit_vector[i].abs();
            if axis_component > 0.0 {
                let axis_accel_limit = self.config.max_acceleration[i] / axis_component;
                max_acceleration = max_acceleration.min(axis_accel_limit);
            }
        }
        let acceleration_limited_feedrate = (2.0 * max_acceleration * distance).sqrt();
        requested_feedrate.min(acceleration_limited_feedrate).max(0.1)
    }

    async fn replan_queue(&mut self) -> Result<(), MotionError> {
        let queue_len = self.motion_queue.len();
        if queue_len < 2 {
            return Ok(());
        }
        tracing::debug!("Replanning {} motion segments", queue_len);
        let mut segments: Vec<MotionSegment> = self.motion_queue.drain(..).collect();
        self.forward_pass(&mut segments)?;
        self.backward_pass(&mut segments)?;
        self.recalculate_durations(&mut segments)?;
        for segment in segments {
            self.motion_queue.push_back(segment);
        }
        Ok(())
    }

    fn forward_pass(&mut self, segments: &mut [MotionSegment]) -> Result<(), MotionError> {
        let mut previous_unit_vector = None;
        for i in 0..segments.len() {
            let unit_vector = if i == 0 {
                self.calculate_unit_vector(&self.current_position, &segments[i].target)
            } else {
                self.calculate_unit_vector(&segments[i-1].target, &segments[i].target)
            };
            if let Some(prev_unit) = previous_unit_vector {
                // TODO: Calculate junction speed using available shared types or reimplement locally
                // let junction_speed = self.junction_deviation.calculate_junction_speed(
                //     &prev_unit,
                //     &unit_vector,
                //     segments[i].acceleration,
                // );
                // segments[i].entry_speed = segments[i].entry_speed.min(junction_speed);
            }
            let max_exit_speed = ((segments[i].entry_speed * segments[i].entry_speed) + 
                                 2.0 * segments[i].acceleration * segments[i].distance).sqrt();
            segments[i].exit_speed = segments[i].limited_feedrate.min(max_exit_speed);
            previous_unit_vector = Some(unit_vector);
        }
        Ok(())
    }

    fn backward_pass(&mut self, segments: &mut [MotionSegment]) -> Result<(), MotionError> {
        for i in (0..segments.len()).rev() {
            let exit_speed = if i == segments.len() - 1 {
                0.0
            } else {
                segments[i + 1].entry_speed
            };
            let max_entry_speed = ((exit_speed * exit_speed) + 
                                  2.0 * segments[i].acceleration * segments[i].distance).sqrt();
            segments[i].entry_speed = segments[i].entry_speed.min(max_entry_speed);
            segments[i].exit_speed = ((segments[i].entry_speed * segments[i].entry_speed) + 
                                     2.0 * segments[i].acceleration * segments[i].distance).sqrt()
                                     .min(segments[i].limited_feedrate);
        }
        Ok(())
    }

    fn recalculate_durations(&mut self, segments: &mut [MotionSegment]) -> Result<(), MotionError> {
        for segment in segments {
            let delta_v = segment.exit_speed - segment.entry_speed;
            let discriminant = delta_v * delta_v + 2.0 * segment.acceleration * segment.distance;
            if discriminant >= 0.0 {
                let time = (delta_v + discriminant.sqrt()) / segment.acceleration;
                segment.duration = time.max(0.0);
            } else {
                segment.duration = segment.distance / segment.limited_feedrate;
            }
        }
        Ok(())
    }

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

    fn calculate_distance(&self, start: &[f64; 4], end: &[f64; 4]) -> f64 {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let de = end[3] - start[3];
        (dx * dx + dy * dy + dz * dz + de * de).sqrt()
    }

    fn calculate_acceleration(&self, target: &[f64; 4]) -> f64 {
        let distance = self.calculate_distance(&self.current_position, target);
        if distance == 0.0 {
            return self.config.max_acceleration[0];
        }
        let unit_vector = self.calculate_unit_vector(&self.current_position, target);
        let weighted_accel = 
            unit_vector[0].abs() * self.config.max_acceleration[0] +
            unit_vector[1].abs() * self.config.max_acceleration[1] +
            unit_vector[2].abs() * self.config.max_acceleration[2] +
            unit_vector[3].abs() * self.config.max_acceleration[3];
        weighted_accel
    }

    pub async fn update(&mut self) -> Result<(), MotionError> {
        match self.state {
            MotionQueueState::Paused => {
                return Ok(());
            }
            MotionQueueState::Cancelled => {
                self.state = MotionQueueState::Idle;
                return Ok(());
            }
            MotionQueueState::Idle => {
                return Ok(());
            }
            MotionQueueState::Running => {}
        }
        let now = std::time::Instant::now();
        let dt = (now - self.planner_state.last_update).as_secs_f64();
        self.planner_state.last_update = now;
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
        let mut clear_segment = false;
        if let Some(ref mut segment) = self.planner_state.current_segment {
            self.planner_state.segment_time += dt;
            if self.planner_state.segment_time >= segment.duration {
                self.current_position = segment.target;
                clear_segment = true;
                tracing::debug!(
                    "Completed move to [{:.3}, {:.3}, {:.3}, {:.3}]",
                    segment.target[0], segment.target[1], segment.target[2], segment.target[3]
                );
            } else {
                let progress = self.planner_state.segment_time / segment.duration;
                let current_pos = [
                    self.current_position[0] + (segment.target[0] - self.current_position[0]) * progress,
                    self.current_position[1] + (segment.target[1] - self.current_position[1]) * progress,
                    self.current_position[2] + (segment.target[2] - self.current_position[2]) * progress,
                    self.current_position[3] + (segment.target[3] - self.current_position[3]) * progress,
                ];
                let segment_copy = segment.clone();
                let _ = segment;
                self.generate_steps(&current_pos, &segment_copy).await?;
            }
        }
        if clear_segment {
            self.planner_state.current_segment = None;
        }
        Ok(())
    }

    async fn generate_steps(
        &mut self,
        position: &[f64; 4],
        segment: &MotionSegment,
    ) -> Result<(), MotionError> {
        let mut shaped_position = [0.0; 4];
        for i in 0..4 {
            shaped_position[i] = self.input_shapers.do_step(i, position[i]);
        }
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

    pub fn set_position(&mut self, position: [f64; 4]) {
        self.current_position = position;
        tracing::debug!("Planner position set to {:?}", position);
    }

    pub fn set_max_acceleration(&mut self, max_accel: [f64; 4]) {
        self.config.max_acceleration = max_accel;
    }
    pub fn set_max_jerk(&mut self, max_jerk: [f64; 4]) {
        self.config.max_jerk = max_jerk;
    }
    pub fn set_junction_deviation(&mut self, jd: f64) {
        self.config.junction_deviation = jd;
    }

    pub fn lookahead_buffer_size(&self) -> usize {
        self.config.lookahead_buffer_size
    }
}

impl Clone for MotionPlanner {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            current_position: self.current_position,
            motion_queue: self.motion_queue.clone(),
            planner_state: self.planner_state.clone(),
            kinematics: self.kinematics.clone_box(),
            input_shapers: self.input_shapers.clone(),
            state: self.state.clone(),
        }
    }
}

impl std::fmt::Debug for MotionPlanner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MotionPlanner")
            .field("current_position", &self.current_position)
            .field("motion_queue_len", &self.motion_queue.len())
            .field("planner_state", &self.planner_state)
            .field("kinematics", &"Box<dyn Kinematics + Send>")
            .field("state", &self.state)
            .finish()
    }
}

// Adaptive logic is in krusty_shared::motion::adaptive

#[derive(Debug, Clone)]
pub struct AdvancedMotionConfig {
    pub input_shapers: Option<Vec<Option<InputShaperType>>>,
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
            input_shapers: None,
            bezier_blending: Some(BezierBlendingConfig {
                enabled: false,
                degree: 15,
                max_deviation: 0.5,
            }),
        }
    }
}
