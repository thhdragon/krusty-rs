// src/motion/planner/mod.rs

use std::collections::VecDeque;
use crate::motion::kinematics::{Kinematics, KinematicsType};
use crate::motion::junction::JunctionDeviation; // Make sure JunctionDeviation exists and is public in junction.rs
use crate::config::Config; // Assuming Config is needed for new_from_config

// Re-export shared types if they were defined here or move them here.
// For now, assuming MotionType and core structs are defined here.

#[derive(Debug, Clone, PartialEq)]
pub enum MotionType {
    Print,
    Travel,
    Home,
    Extruder,
}

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

impl MotionConfig {
    pub fn new_from_config(config: &Config) -> Self {
        // Implement based on your config structure
        // This is a placeholder implementation
        Self {
            max_velocity: [config.printer.max_velocity, config.printer.max_velocity, config.printer.max_z_velocity, 50.0], // Example E max vel
            max_acceleration: [config.printer.max_accel, config.printer.max_accel, config.printer.max_z_accel, 1000.0], // Example E max accel
            max_jerk: [20.0, 20.0, 0.5, 2.0], // Example jerk values
            junction_deviation: 0.05, // mm
            axis_limits: [[0.0, 200.0], [0.0, 200.0], [0.0, 200.0]], // Example limits
            kinematics_type: match config.printer.kinematics.as_str() {
                "corexy" => KinematicsType::CoreXY,
                "delta" => KinematicsType::Delta,
                _ => KinematicsType::Cartesian,
            },
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
    pub limited_feedrate: f64,
    pub distance: f64,
    pub duration: f64,
    pub acceleration: f64,
    pub entry_speed: f64,
    pub exit_speed: f64,
    pub motion_type: MotionType,
    // Add timestamp if needed
    // pub timestamp: std::time::Instant,
}

// Internal state for the planner
#[derive(Debug, Clone)]
struct PlannerState {
    active: bool,
    current_segment: Option<MotionSegment>, // Was MotionBlock
    segment_time: f64,
    last_update: std::time::Instant,
}

pub struct MotionPlanner { // Was AdvancedMotionPlanner
    // config: MotionConfig, // If needed as a field
    // kinematics: Box<dyn Kinematics>,
    // junction_deviation: JunctionDeviation,
    current_position: [f64; 4],
    motion_queue: VecDeque<MotionSegment>, // Was MotionBlock
    planner_state: PlannerState,
}

impl MotionPlanner {
    pub fn new(config: MotionConfig) -> Self { // Simplified constructor
        // let kinematics = create_kinematics(config.kinematics_type, config.axis_limits);
        // let junction_deviation = JunctionDeviation::new(config.junction_deviation);

        Self {
            // config,
            // kinematics,
            // junction_deviation,
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
        // println!("Planning move to {:?}", target); // Use tracing in real code

        let distance = self.calculate_distance(&self.current_position, &target);

        if distance < 0.001 { // Use config.minimum_step_distance
            // println!("Skipping very small move: {}mm", distance);
            return Ok(());
        }

        // Create a basic segment for now. Add acceleration limiting, junction deviation later.
        let segment = MotionSegment {
            target,
            feedrate,
            limited_feedrate: feedrate, // Placeholder, apply limits
            distance,
            duration: distance / feedrate.max(0.1), // Avoid division by zero
            acceleration: 1000.0, // Placeholder, calculate based on axes
            entry_speed: 0.0,
            exit_speed: 0.0,
            motion_type,
        };

        self.motion_queue.push_back(segment);
        self.current_position = target; // Update current position

        // Trigger replanning if queue has enough moves (placeholder)
        if self.motion_queue.len() >= 2 { // Use config.lookahead_buffer_size / 2
            self.replan_queue()?;
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
                // println!("Starting new segment");
            } else {
                self.planner_state.active = false;
                // println!("No segments in queue");
                return Ok(());
            }
        }

        // Process current segment
        if let Some(ref mut segment) = self.planner_state.current_segment { // Use ref mut
            self.planner_state.segment_time += dt;

            // Check if segment is complete
            if self.planner_state.segment_time >= segment.duration {
                // Segment complete
                // println!(
                //     "Completed move to [{:.3}, {:.3}, {:.3}, {:.3}]",
                //     segment.target[0], segment.target[1], segment.target[2], segment.target[3]
                // );
                self.current_position = segment.target; // Update position

                // Clear current segment
                self.planner_state.current_segment = None; // This is now safe

                // Potentially trigger next segment start immediately in next update call
            } else {
                // Interpolate position within segment (placeholder)
                let progress = self.planner_state.segment_time / segment.duration;
                let current_pos = [
                    self.current_position[0] + (segment.target[0] - self.current_position[0]) * progress,
                    self.current_position[1] + (segment.target[1] - self.current_position[1]) * progress,
                    self.current_position[2] + (segment.target[2] - self.current_position[2]) * progress,
                    self.current_position[3] + (segment.target[3] - self.current_position[3]) * progress,
                ];
                // println!("Interpolated position: {:?}", current_pos);
                // In a real implementation, generate steps here based on current_pos
            }
        }
        Ok(())
    }

    // Placeholder for step generation
    async fn generate_steps(&self, _position: &[f64; 4], _segment: &MotionSegment) -> Result<(), Box<dyn std::error::Error>> {
        // This function would calculate the steps needed to reach _position
        // and send them to the hardware.
        // println!("Generating steps for position {:?}", _position);
        Ok(())
    }

    pub fn clear_queue(&mut self) {
        self.motion_queue.clear();
        self.planner_state.current_segment = None;
        self.planner_state.segment_time = 0.0;
        // println!("Motion queue cleared");
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
}


// Implement Clone manually if needed and direct fields support it
impl Clone for MotionPlanner {
    fn clone(&self) -> Self {
        Self {
            // config: self.config.clone(),
            // kinematics: self.kinematics.box_clone(), // Requires box_clone on Kinematics trait
            // junction_deviation: self.junction_deviation.clone(),
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
