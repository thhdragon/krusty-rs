// src/motion/planner.rs - Make sure this compiles
use std::collections::VecDeque;

// Also make sure MotionConfig implements Debug
#[derive(Debug, Clone)]
pub struct MotionConfig {
    pub max_velocity: [f64; 4],
    pub max_acceleration: [f64; 4],
    pub max_jerk: [f64; 4],
    pub junction_deviation: f64,
    pub axis_limits: [[f64; 2]; 3],
    pub kinematics_type: crate::motion::kinematics::KinematicsType,
    pub minimum_step_distance: f64,
    pub lookahead_buffer_size: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MotionType {
    Print,
    Travel,
    Home,
    Extruder,
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
pub struct MotionPlanner {
    config: MotionConfig,
    queue: std::collections::VecDeque<MotionSegment>,
    current_position: [f64; 4],
}

impl MotionPlanner {
    pub fn new(config: MotionConfig) -> Self {
        Self {
            config,
            queue: VecDeque::new(),
            current_position: [0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn plan_move(
        &mut self,
        target: [f64; 4],
        feedrate: f64,
        motion_type: MotionType,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let distance = self.calculate_distance(&self.current_position, &target);
        
        if distance < self.config.minimum_step_distance {
            return Ok(());
        }
        
        let limited_feedrate = self.limit_feedrate(target, feedrate);
        let acceleration = self.calculate_acceleration(target);
        
        let segment = MotionSegment {
            target,
            feedrate,
            limited_feedrate,
            distance,
            duration: distance / limited_feedrate.max(0.1),
            acceleration,
            entry_speed: 0.0,
            exit_speed: 0.0,
            motion_type,
        };
        
        self.queue.push_back(segment);
        self.current_position = target;
        
        Ok(())
    }

    fn calculate_distance(&self, start: &[f64; 4], end: &[f64; 4]) -> f64 {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let de = end[3] - start[3];
        (dx * dx + dy * dy + dz * dz + de * de).sqrt()
    }

    fn limit_feedrate(&self, target: [f64; 4], requested_feedrate: f64) -> f64 {
        let distance = self.calculate_distance(&self.current_position, &target);
        if distance == 0.0 {
            return requested_feedrate;
        }
        
        let dx = target[0] - self.current_position[0];
        let dy = target[1] - self.current_position[1];
        let dz = target[2] - self.current_position[2];
        let de = target[3] - self.current_position[3];
        
        let unit_x = dx / distance;
        let unit_y = dy / distance;
        let unit_z = dz / distance;
        let unit_e = de / distance;
        
        let mut max_accel = f64::INFINITY;
        for i in 0..4 {
            let component = match i {
                0 => unit_x.abs(),
                1 => unit_y.abs(),
                2 => unit_z.abs(),
                3 => unit_e.abs(),
                _ => 0.0,
            };
            
            if component > 0.0 {
                let limit = self.config.max_acceleration[i] / component;
                max_accel = max_accel.min(limit);
            }
        }
        
        let accel_limit_feedrate = (2.0 * max_accel * distance).sqrt();
        requested_feedrate.min(accel_limit_feedrate).max(0.1)
    }

    fn calculate_acceleration(&self, target: [f64; 4]) -> f64 {
        let distance = self.calculate_distance(&self.current_position, &target);
        if distance == 0.0 {
            return self.config.max_acceleration[0];
        }
        
        let dx = target[0] - self.current_position[0];
        let dy = target[1] - self.current_position[1];
        let dz = target[2] - self.current_position[2];
        let de = target[3] - self.current_position[3];
        
        let unit_x = dx / distance;
        let unit_y = dy / distance;
        let unit_z = dz / distance;
        let unit_e = de / distance;
        
        let weighted = 
            unit_x.abs() * self.config.max_acceleration[0] +
            unit_y.abs() * self.config.max_acceleration[1] +
            unit_z.abs() * self.config.max_acceleration[2] +
            unit_e.abs() * self.config.max_acceleration[3];
        
        weighted.max(10.0)
    }

    pub fn clear_queue(&mut self) {
        self.queue.clear();
    }
}

impl Clone for MotionPlanner {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            queue: VecDeque::new(), // Start with empty queue
            current_position: self.current_position,
        }
    }
}