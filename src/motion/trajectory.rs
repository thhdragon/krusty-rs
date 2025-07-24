// src/motion/trajectory.rs - Add trajectory generation
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
    pub target: [f64; 4],
    pub start_velocity: [f64; 4],
    pub end_velocity: [f64; 4],
    pub acceleration: [f64; 4],
    pub duration: f64,
    pub motion_type: MotionType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MotionType {
    Print,
    Travel,
    Home,
    Extruder,
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
        
        let segment = TrajectorySegment {
            target,
            start_velocity: [0.0; 4],
            end_velocity: [0.0; 4],
            acceleration,
            duration: accel_time + cruise_time + decel_time,
            motion_type,
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