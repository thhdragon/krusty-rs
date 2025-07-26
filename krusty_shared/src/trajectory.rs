// src/trajectory.rs - Shared trajectory generation logic
use std::collections::VecDeque;
use thiserror::Error;

/// Motion trajectory generator with proper acceleration control
#[derive(Debug, Clone)]
pub struct TrajectoryGenerator {
    /// Current position
    current_position: [f64; 4],
    
    /// Motion queue
    motion_queue: VecDeque<TrajectorySegment>,
    
    /// Trajectory configuration
    config: TrajectoryConfig,
}

#[derive(Debug, Clone)]
pub struct TrajectoryConfig {
    pub max_velocity: [f64; 4],
    pub max_acceleration: [f64; 4],
    pub max_jerk: [f64; 4],
    pub minimum_segment_time: f64,
}

#[derive(Debug, Clone)]
pub struct TrajectorySegment {
    /// Target position at the end of the segment
    pub target: [f64; 4],
    /// Start velocity for each axis (mm/s)
    pub start_velocity: [f64; 4],
    /// End velocity for each axis (mm/s)
    pub end_velocity: [f64; 4],
    /// Acceleration for each axis (mm/s^2)
    pub acceleration: [f64; 4],
    /// Duration of the segment (seconds)
    pub duration: f64,
    /// Type of motion (Print, Travel, Home, Extruder)
    pub motion_type: MotionType,
    /// Requested feedrate (mm/s)
    pub feedrate: f64,
    /// Feedrate after acceleration/jerk limiting (mm/s)
    pub limited_feedrate: f64,
    /// Euclidean distance of the segment (mm)
    pub distance: f64,
    /// Entry speed at the start of the segment (mm/s)
    pub entry_speed: f64,
    /// Exit speed at the end of the segment (mm/s)
    pub exit_speed: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MotionType {
    Print,
    Travel,
    Home,
    Extruder,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MotionQueueState {
    Idle,
    Running,
    Paused,
    Cancelled,
}

#[derive(Debug, thiserror::Error)]
pub enum MotionError {
    #[error("Junction deviation error: {0}")]
    JunctionDeviation(String),
    #[error("Kinematics error: {0}")]
    Kinematics(String),
    #[error("Other: {0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub struct MotionConfig {
    pub max_velocity: [f64; 4],
    pub max_acceleration: [f64; 4],
    pub max_jerk: [f64; 4],
    pub junction_deviation: f64,
    pub axis_limits: [[f64; 2]; 3],
    pub kinematics_type: crate::KinematicsType,
    pub minimum_step_distance: f64,
    pub lookahead_buffer_size: usize,
}

impl Default for MotionConfig {
    fn default() -> Self {
        Self {
            max_velocity: [100.0, 100.0, 10.0, 50.0],
            max_acceleration: [1000.0, 1000.0, 100.0, 1000.0],
            max_jerk: [20.0, 20.0, 0.5, 2.0],
            junction_deviation: 0.05,
            axis_limits: [[0.0, 200.0], [0.0, 200.0], [0.0, 200.0]],
            kinematics_type: crate::KinematicsType::Cartesian,
            minimum_step_distance: 0.001,
            lookahead_buffer_size: 16,
        }
    }
}

#[derive(Debug, Error)]
pub enum TrajectoryError {
    #[error("Invalid trajectory parameters: {0}")]
    InvalidParameters(String),
    #[error("Other: {0}")]
    Other(String),
}

impl TrajectoryGenerator {
    pub fn new(config: TrajectoryConfig) -> Self {
        Self {
            current_position: [0.0, 0.0, 0.0, 0.0],
            motion_queue: VecDeque::new(),
            config,
        }
    }

    /// Generate trapezoidal velocity profile for a move
    pub fn generate_trapezoidal_move(
        &mut self,
        target: [f64; 4],
        feedrate: f64,
        motion_type: MotionType,
    ) -> Result<(), TrajectoryError> {
        let distance = self.calculate_distance(&self.current_position, &target);
        if distance < 0.001 {
            return Ok(());
        }
        // Calculate unit vector
        let unit_vector = self.calculate_unit_vector(&self.current_position, &target);
        // Calculate limited velocity based on acceleration limits
        let limited_velocity = self.limit_velocity_by_acceleration(&unit_vector, feedrate);
        // Calculate acceleration for each axis
        let acceleration = self.calculate_axis_accelerations(&unit_vector);
        // Calculate trapezoidal profile parameters
        let (accel_time, cruise_time, decel_time) = self.calculate_trapezoidal_times(
            distance,
            limited_velocity,
            &acceleration,
        );
        // Compute entry/exit speeds (for now, assume start at rest, end at rest)
        // In a more advanced planner, these would be set by lookahead or previous segment
        let entry_speed = self.motion_queue.back().map_or(0.0, |prev| prev.exit_speed);
        let exit_speed = 0.0; // For now, always end at rest
        let segment = TrajectorySegment {
            target,
            start_velocity: [entry_speed; 4], // Placeholder: per-axis in future
            end_velocity: [exit_speed; 4],    // Placeholder: per-axis in future
            acceleration,
            duration: accel_time + cruise_time + decel_time,
            motion_type,
            feedrate,
            limited_feedrate: limited_velocity,
            distance,
            entry_speed,
            exit_speed,
        };
        self.motion_queue.push_back(segment);
        self.current_position = target;
        tracing::debug!("Generated trapezoidal move: {:.3}mm @ {:.1}mm/s", distance, limited_velocity);
        Ok(())
    }

    fn calculate_distance(&self, start: &[f64; 4], end: &[f64; 4]) -> f64 {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let de = end[3] - start[3];
        (dx * dx + dy * dy + dz * dz + de * de).sqrt()
    }

    fn calculate_unit_vector(&self, start: &[f64; 4], end: &[f64; 4]) -> [f64; 4] {
        let distance = self.calculate_distance(start, end);
        if distance == 0.0 {
            return [0.0; 4];
        }
        
        [
            (end[0] - start[0]) / distance,
            (end[1] - start[1]) / distance,
            (end[2] - start[2]) / distance,
            (end[3] - start[3]) / distance,
        ]
    }

    fn limit_velocity_by_acceleration(&self, unit_vector: &[f64; 4], requested_velocity: f64) -> f64 {
        let mut max_acceleration = f64::INFINITY;
        
        for i in 0..4 {
            let component = unit_vector[i].abs();
            if component > 0.0 {
                let limit = self.config.max_acceleration[i] / component;
                max_acceleration = max_acceleration.min(limit);
            }
        }
        
        // Calculate maximum velocity based on acceleration and distance
        let distance = 10.0; // Assume reasonable distance for calculation
        let accel_limited_velocity = (2.0 * max_acceleration * distance).sqrt();
        
        requested_velocity.min(accel_limited_velocity).max(0.1)
    }

    fn calculate_axis_accelerations(&self, unit_vector: &[f64; 4]) -> [f64; 4] {
        let mut accelerations = [0.0; 4];
        
        for i in 0..4 {
            accelerations[i] = unit_vector[i].abs() * self.config.max_acceleration[i];
        }
        
        accelerations
    }

    fn calculate_trapezoidal_times(
        &self,
        distance: f64,
        velocity: f64,
        acceleration: &[f64; 4],
    ) -> (f64, f64, f64) {
        // Use average acceleration for time calculations
        let avg_accel = (acceleration[0] + acceleration[1] + acceleration[2] + acceleration[3]) / 4.0;
        
        if avg_accel <= 0.0 {
            return (0.0, distance / velocity, 0.0);
        }
        
        let accel_time = velocity / avg_accel;
        let accel_distance = 0.5 * avg_accel * accel_time * accel_time;
        
        let decel_time = accel_time;
        let decel_distance = accel_distance;
        
        let cruise_distance = distance - accel_distance - decel_distance;
        let cruise_time = if cruise_distance > 0.0 {
            cruise_distance / velocity
        } else {
            0.0
        };
        
        (accel_time, cruise_time, decel_time)
    }

    pub fn get_next_segment(&mut self) -> Option<TrajectorySegment> {
        self.motion_queue.pop_front()
    }

    pub fn clear_queue(&mut self) {
        self.motion_queue.clear();
    }

    pub fn get_queue_length(&self) -> usize {
        self.motion_queue.len()
    }
}
