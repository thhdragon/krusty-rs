// src/motion/planner/mod.rs

use std::collections::VecDeque;
use crate::config::Config; // Assuming Config is needed for new_from_config

// Re-export shared types if they were defined here or move them here.
// For now, assuming MotionType and core structs are defined here.

#[derive(Debug, Clone, PartialEq)]
pub enum MotionType {
    Print,
    Travel,
    Home,
    // Extruder, // Commented out as not used immediately
}

#[derive(Debug, Clone)]
pub struct MotionConfig {
    pub max_velocity: [f64; 4],
    pub max_acceleration: [f64; 4],
    pub max_jerk: [f64; 4],
    pub junction_deviation: f64,
    pub axis_limits: [[f64; 2]; 3],
    // pub kinematics_type: KinematicsType, // Removed as not used
    pub minimum_step_distance: f64,
    pub lookahead_buffer_size: usize,
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
            // kinematics_type: KinematicsType::Cartesian, // Removed
            minimum_step_distance: 0.001, // mm
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
}

impl MotionPlanner {
    pub fn new(config: MotionConfig) -> Self {
        Self {
            config,
            current_position: [0.0, 0.0, 0.0, 0.0],
            motion_queue: VecDeque::new(),
            planner_state: PlannerState {
                active: false,
                current_segment: None,
                segment_time: 0.0,
                last_update: std::time::Instant::now(),
            },
        }
    }

    pub fn new_from_config(config: &Config) -> Self {
        let planner_config = MotionConfig::new_from_config(config);
        Self::new(planner_config)
    }

    pub async fn plan_move(
        &mut self,
        target: [f64; 4],
        feedrate: f64,
        motion_type: MotionType,
    ) -> Result<(), Box<dyn std::error::Error>> {
        tracing::debug!("Planning move to {:?}", target);

        let distance = self.calculate_distance(&self.current_position, &target);

        if distance < self.config.minimum_step_distance {
            tracing::debug!("Skipping very small move: {}mm", distance);
            return Ok(());
        }

        // --- Basic Acceleration Limiting (Placeholder logic, improve later) ---
        // A very simplified approach: limit feedrate based on the axis with the highest demand relative to its max accel.
        let mut limited_feedrate = feedrate;
        if distance > 0.0 {
            let dx = (target[0] - self.current_position[0]).abs() / distance;
            let dy = (target[1] - self.current_position[1]).abs() / distance;
            let dz = (target[2] - self.current_position[2]).abs() / distance;
            let de = (target[3] - self.current_position[3]).abs() / distance;

            // Find the most constrained axis based on acceleration
            let mut max_acceleration_component = 0.0;
            if dx > 0.0 { max_acceleration_component = f64::max(max_acceleration_component, self.config.max_acceleration[0] / dx); }
            if dy > 0.0 { max_acceleration_component = f64::max(max_acceleration_component, self.config.max_acceleration[1] / dy); }
            if dz > 0.0 { max_acceleration_component = f64::max(max_acceleration_component, self.config.max_acceleration[2] / dz); }
            if de > 0.0 { max_acceleration_component = f64::max(max_acceleration_component, self.config.max_acceleration[3] / de); }

            // Estimate max velocity based on acceleration and distance (v_max = sqrt(2 * a * d))
            // This is a simplification assuming we accelerate halfway and decelerate halfway.
            if max_acceleration_component > 0.0 {
                let accel_limited_v = (2.0 * max_acceleration_component * distance).sqrt();
                limited_feedrate = limited_feedrate.min(accel_limited_v).max(0.1); // Min feedrate to avoid divide by zero issues later
            }
        }

        // Create the segment with calculated values
        let duration = if limited_feedrate > 0.0 { distance / limited_feedrate } else { 1.0 }; // Avoid division by zero
        // Acceleration calculation is complex for coordinated moves, use a placeholder average for now.
        let avg_accel = (self.config.max_acceleration[0] + self.config.max_acceleration[1] + self.config.max_acceleration[2]) / 3.0 / 10.0; // Very rough

        let segment = MotionSegment {
            target,
            feedrate,
            limited_feedrate,
            distance,
            duration,
            acceleration: avg_accel, // Placeholder
            entry_speed: 0.0, // Placeholder
            exit_speed: 0.0, // Placeholder
            motion_type,
        };

        self.motion_queue.push_back(segment);
        // Don't update self.current_position here yet, it should be updated when the move is executed.
        // self.current_position = target;

        tracing::debug!("Planned move: {:?} mm, dist: {:.3}mm, req v: {:.1}mm/s, lim v: {:.1}mm/s, dur: {:.3}s", target, distance, feedrate, limited_feedrate, duration);

        // Trigger replanning if queue has enough moves (placeholder)
        if self.motion_queue.len() >= self.config.lookahead_buffer_size / 2 {
            // self.replan_queue()?; // Implement later
            tracing::debug!("Queue size {} reached lookahead trigger ({}). Replanning would happen here.", self.motion_queue.len(), self.config.lookahead_buffer_size / 2);
        }

        Ok(())
    }

    fn calculate_distance(&self, start: &[f64; 4], end: &[f64; 4]) -> f64 {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let de = end[3] - start[3];
        (dx * dx + dy * dy + dz * dz + de * de).sqrt()
    }

    // Placeholder for acceleration limiting
    fn limit_feedrate_by_acceleration(&self, _segment: &MotionSegment) -> f64 {
        // Implement acceleration-based feedrate limiting
        300.0 // Placeholder
    }

    // Placeholder for junction deviation
    fn apply_junction_deviation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Implement junction deviation logic using JunctionDeviation
        // This would typically involve looking at the queue of segments
        Ok(())
    }

    fn replan_queue(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let queue_len = self.motion_queue.len();
        if queue_len < 2 {
            return Ok(());
        }
        // println!("Replanning {} motion segments", queue_len);

        // Placeholder for complex replanning logic (lookahead, junctions)
        // For now, just demonstrate accessing segments
        /*
        let mut segments: Vec<MotionSegment> = self.motion_queue.drain(..).collect();
        // ... perform optimizations ...
        for segment in segments {
            self.motion_queue.push_back(segment);
        }
        */
        Ok(())
    }

    /// Main update loop for motion execution.
    /// This should be called at a high frequency.
    pub async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = std::time::Instant::now();
        let dt = (now - self.planner_state.last_update).as_secs_f64();
        self.planner_state.last_update = now;

        // Check if we need to start a new segment
        if self.planner_state.current_segment.is_none() {
            if let Some(segment) = self.motion_queue.pop_front() {
                self.planner_state.current_segment = Some(segment);
                self.planner_state.segment_time = 0.0;
                self.planner_state.active = true;
                tracing::debug!("Starting new segment to {:?}", self.planner_state.current_segment.as_ref().unwrap().target);
            } else {
                self.planner_state.active = false;
                // tracing::trace!("No segments in queue");
                return Ok(());
            }
        }

        // Process current segment
        if let Some(segment) = self.planner_state.current_segment.as_mut() {
            self.planner_state.segment_time += dt;

            // Check if segment is complete
            if self.planner_state.segment_time >= segment.duration {
                tracing::debug!(
                    "Completed move to [{:.3}, {:.3}, {:.3}, {:.3}]",
                    segment.target[0], segment.target[1], segment.target[2], segment.target[3]
                );
                self.current_position = segment.target;
                self.planner_state.current_segment = None;
            } else {
                // Interpolate position within segment
                let progress = self.planner_state.segment_time / segment.duration;
                let current_pos = [
                    self.current_position[0] + (segment.target[0] - self.current_position[0]) * progress,
                    self.current_position[1] + (segment.target[1] - self.current_position[1]) * progress,
                    self.current_position[2] + (segment.target[2] - self.current_position[2]) * progress,
                    self.current_position[3] + (segment.target[3] - self.current_position[3]) * progress,
                ];
                tracing::trace!(
                    "Interpolated position: [{:.3}, {:.3}, {:.3}, {:.3}]",
                    current_pos[0], current_pos[1], current_pos[2], current_pos[3]
                );
                // In a real implementation, generate steps here based on current_pos
            }
        }
        Ok(())
    }

    // Placeholder for step generation - will connect to hardware later
    // Using `&mut self` because we might need to interact with hardware state later (e.g., stepper drivers, IO buffers).
    async fn generate_steps(&mut self, _position: &[f64; 4], _segment: &MotionSegment) -> Result<(), Box<dyn std::error::Error>> {
        // This function would calculate the steps needed to reach _position
        // and send them to the hardware.
        // tracing::trace!("Generating steps for position {:?}", _position);
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
}


// Implement Clone manually if needed and direct fields support it
impl Clone for MotionPlanner {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            current_position: self.current_position,
            motion_queue: self.motion_queue.clone(),
            planner_state: self.planner_state.clone(),
        }
    }
}

// Implement Debug manually if needed due to non-Debug fields
impl std::fmt::Debug for MotionPlanner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MotionPlanner")
            // .field("config", &self.config)
            // .field("kinematics", &"Box<dyn Kinematics>")
            // .field("junction_deviation", &self.junction_deviation)
            .field("current_position", &self.current_position)
            .field("motion_queue_len", &self.motion_queue.len())
            .field("planner_state", &self.planner_state)
            .finish()
    }
}
