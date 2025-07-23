// src/motion/planner.rs
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::PrinterState;
use crate::hardware::HardwareManager;

/// A single motion segment in the planned path
#[derive(Debug, Clone)]
pub struct MotionSegment {
    /// Target position [X, Y, Z, E] in mm
    pub target: [f64; 4],
    
    /// Feedrate in mm/s
    pub feedrate: f64,
    
    /// Acceleration in mm/s²
    pub acceleration: f64,
    
    /// Distance of this move in mm
    pub distance: f64,
    
    /// Time to complete this segment in seconds
    pub duration: f64,
    
    /// Type of motion (printing, travel, homing, etc.)
    pub motion_type: MotionType,
}

/// Types of motion segments
#[derive(Debug, Clone, PartialEq)]
pub enum MotionType {
    /// Printing move (extruder moving)
    Print,
    
    /// Travel move (no extrusion)
    Travel,
    
    /// Homing move
    Home,
    
    /// Retract/Prime move
    Extruder,
}

/// Motion planning parameters
#[derive(Debug, Clone)]
pub struct MotionConfig {
    /// Maximum velocity for each axis (mm/s)
    pub max_velocity: [f64; 4], // [X, Y, Z, E]
    
    /// Maximum acceleration for each axis (mm/s²)
    pub max_acceleration: [f64; 4],
    
    /// Maximum jerk for each axis (mm/s)
    pub max_jerk: [f64; 4],
    
    /// Minimum movement distance (moves smaller than this may be skipped)
    pub minimum_step_distance: f64,
    
    /// Lookahead buffer size for motion planning
    pub lookahead_buffer_size: usize,
}

impl MotionConfig {
    pub fn new_from_printer_config(config: &crate::config::Config) -> Self {
        Self {
            max_velocity: [
                config.printer.max_velocity,
                config.printer.max_velocity,
                config.printer.max_z_velocity,
                50.0, // Extruder max velocity
            ],
            max_acceleration: [
                config.printer.max_accel,
                config.printer.max_accel,
                config.printer.max_z_accel,
                1000.0, // Extruder max acceleration
            ],
            max_jerk: [10.0, 10.0, 0.4, 2.0], // Typical jerk values
            minimum_step_distance: 0.001, // 1 micron minimum
            lookahead_buffer_size: 16, // Look ahead at 16 moves
        }
    }
}

/// Motion planner that generates smooth, coordinated movements
pub struct MotionPlanner {
    /// Shared printer state
    state: Arc<RwLock<PrinterState>>,
    
    /// Hardware interface for sending step commands
    hardware_manager: HardwareManager,
    
    /// Motion configuration parameters
    config: MotionConfig,
    
    /// Current position [X, Y, Z, E]
    current_position: [f64; 4],
    
    /// Planned motion segments waiting execution
    motion_queue: VecDeque<MotionSegment>,
    
    /// Current velocity for each axis
    current_velocity: [f64; 4],
    
    /// Planner state
    planner_state: PlannerState,
}

/// Internal state of the motion planner
#[derive(Debug, Clone)]
struct PlannerState {
    /// Whether planner is currently executing moves
    active: bool,
    
    /// Current segment being executed
    current_segment: Option<MotionSegment>,
    
    /// Time into current segment (seconds)
    segment_time: f64,
    
    /// Last update timestamp
    last_update: std::time::Instant,
}

impl MotionPlanner {
    /// Create a new motion planner
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        hardware_manager: HardwareManager,
        config: MotionConfig,
    ) -> Self {
        Self {
            state,
            hardware_manager,
            config,
            current_position: [0.0, 0.0, 0.0, 0.0],
            motion_queue: VecDeque::new(),
            current_velocity: [0.0; 4],
            planner_state: PlannerState {
                active: false,
                current_segment: None,
                segment_time: 0.0,
                last_update: std::time::Instant::now(),
            },
        }
    }

    /// Add a linear move to the motion queue
    /// 
    /// This method:
    /// 1. Calculates move parameters
    /// 2. Plans acceleration profile
    /// 3. Adds to queue for execution
    pub async fn plan_linear_move(
        &mut self,
        target: [f64; 4], // [X, Y, Z, E]
        feedrate: f64,
        motion_type: MotionType,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Calculate move distance
        let distance = self.calculate_distance(&self.current_position, &target);
        
        // Skip very small moves
        if distance < self.config.minimum_step_distance {
            tracing::debug!("Skipping move smaller than minimum: {}mm", distance);
            return Ok(());
        }
        
        // Calculate acceleration-limited feedrate
        let limited_feedrate = self.limit_feedrate_by_acceleration(&target, feedrate);
        
        // Create motion segment
        let segment = MotionSegment {
            target,
            feedrate: limited_feedrate,
            acceleration: self.calculate_acceleration(&target),
            distance,
            duration: distance / limited_feedrate,
            motion_type,
        };
        
        tracing::debug!(
            "Planned {} move: {:.3}mm @ {:.1}mm/s",
            match motion_type {
                MotionType::Print => "print",
                MotionType::Travel => "travel",
                MotionType::Home => "home",
                MotionType::Extruder => "extruder",
            },
            distance,
            limited_feedrate
        );
        
        // Add to queue
        self.motion_queue.push_back(segment);
        
        // Trigger replanning if queue has enough moves
        if self.motion_queue.len() >= self.config.lookahead_buffer_size / 2 {
            self.replan_queue().await?;
        }
        
        Ok(())
    }

    /// Calculate 3D Euclidean distance between two positions
    fn calculate_distance(&self, start: &[f64; 4], end: &[f64; 4]) -> f64 {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let de = end[3] - start[3];
        
        (dx * dx + dy * dy + dz * dz + de * de).sqrt()
    }

    /// Limit feedrate based on acceleration capabilities
    fn limit_feedrate_by_acceleration(&self, target: &[f64; 4], requested_feedrate: f64) -> f64 {
        // Calculate unit vector for this move
        let distance = self.calculate_distance(&self.current_position, target);
        if distance == 0.0 {
            return requested_feedrate;
        }
        
        let dx = (target[0] - self.current_position[0]) / distance;
        let dy = (target[1] - self.current_position[1]) / distance;
        let dz = (target[2] - self.current_position[2]) / distance;
        let de = (target[3] - self.current_position[3]) / distance;
        
        // Find limiting acceleration for each axis
        let mut max_acceleration = f64::INFINITY;
        for i in 0..4 {
            let axis_component = match i {
                0 => dx.abs(),
                1 => dy.abs(),
                2 => dz.abs(),
                3 => de.abs(),
                _ => 0.0,
            };
            
            if axis_component > 0.0 {
                let axis_accel_limit = self.config.max_acceleration[i] / axis_component;
                max_acceleration = max_acceleration.min(axis_accel_limit);
            }
        }
        
        // Convert acceleration limit to velocity limit
        // v = sqrt(2 * a * s) where s is the distance we can accelerate in
        let acceleration_limited_feedrate = (2.0 * max_acceleration * distance).sqrt();
        
        // Return the minimum of requested and acceleration-limited feedrates
        requested_feedrate.min(acceleration_limited_feedrate)
    }

    /// Calculate appropriate acceleration for a move
    fn calculate_acceleration(&self, target: &[f64; 4]) -> f64 {
        // Weighted average based on axis movement
        let distance = self.calculate_distance(&self.current_position, target);
        if distance == 0.0 {
            return self.config.max_acceleration[0];
        }
        
        let dx = (target[0] - self.current_position[0]).abs() / distance;
        let dy = (target[1] - self.current_position[1]).abs() / distance;
        let dz = (target[2] - self.current_position[2]).abs() / distance;
        let de = (target[3] - self.current_position[3]).abs() / distance;
        
        let weighted_accel = 
            dx * self.config.max_acceleration[0] +
            dy * self.config.max_acceleration[1] +
            dz * self.config.max_acceleration[2] +
            de * self.config.max_acceleration[3];
        
        weighted_accel
    }

    /// Replan the motion queue for optimal jerk and acceleration
    /// 
    /// This implements lookahead planning to smooth motion between segments
    async fn replan_queue(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Simple implementation - in production, this would implement
        // junction deviation, S-curve acceleration, and advanced lookahead
        
        let queue_len = self.motion_queue.len();
        if queue_len < 2 {
            return Ok(());
        }
        
        tracing::debug!("Replanning {} motion segments", queue_len);
        
        // For now, we'll just ensure smooth velocity transitions
        // A full implementation would calculate optimal junction speeds
        // based on centripetal acceleration and configured jerk limits
        
        Ok(())
    }

    /// Execute motion planning update
    /// 
    /// This method should be called at high frequency (e.g., 10kHz)
    /// to generate precise step timing for motors
    pub async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = std::time::Instant::now();
        let dt = (now - self.planner_state.last_update).as_secs_f64();
        self.planner_state.last_update = now;
        
        // If no active segment, check if we have queued moves
        if self.planner_state.current_segment.is_none() {
            if let Some(segment) = self.motion_queue.pop_front() {
                self.planner_state.current_segment = Some(segment);
                self.planner_state.segment_time = 0.0;
                self.planner_state.active = true;
            } else {
                self.planner_state.active = false;
                return Ok(());
            }
        }
        
        // Process current segment
        if let Some(ref mut segment) = self.planner_state.current_segment {
            self.planner_state.segment_time += dt;
            
            // Check if segment is complete
            if self.planner_state.segment_time >= segment.duration {
                // Move complete - update current position
                self.current_position = segment.target;
                
                // Update printer state
                {
                    let mut state = self.state.write().await;
                    state.position = [
                        self.current_position[0],
                        self.current_position[1],
                        self.current_position[2],
                    ];
                }
                
                // Clear current segment and prepare for next
                self.planner_state.current_segment = None;
                
                tracing::debug!(
                    "Completed move to [{:.3}, {:.3}, {:.3}, {:.3}]",
                    self.current_position[0],
                    self.current_position[1],
                    self.current_position[2],
                    self.current_position[3]
                );
            } else {
                // Interpolate position within segment
                let progress = self.planner_state.segment_time / segment.duration;
                
                // Simple linear interpolation (in advanced version, this would
                // use trapezoidal or S-curve velocity profiles)
                let current_pos = [
                    self.current_position[0] + (segment.target[0] - self.current_position[0]) * progress,
                    self.current_position[1] + (segment.target[1] - self.current_position[1]) * progress,
                    self.current_position[2] + (segment.target[2] - self.current_position[2]) * progress,
                    self.current_position[3] + (segment.target[3] - self.current_position[3]) * progress,
                ];
                
                // Generate steps for this position
                self.generate_steps(&current_pos, segment).await?;
            }
        }
        
        Ok(())
    }

    /// Generate step commands for current interpolated position
    async fn generate_steps(
        &self,
        position: &[f64; 4],
        segment: &MotionSegment,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // In a real implementation, this would:
        // 1. Convert position to step counts for each motor
        // 2. Calculate timing for step pulses
        // 3. Send step commands to MCU
        
        // For now, we'll just log the position
        tracing::trace!(
            "Position: [{:.3}, {:.3}, {:.3}, {:.3}]",
            position[0], position[1], position[2], position[3]
        );
        
        // In real implementation:
        // self.hardware_manager.send_step_commands(position).await?;
        
        Ok(())
    }

    /// Queue a homing operation
    pub async fn plan_home(&mut self, axes: Option<[bool; 3]>) -> Result<(), Box<dyn std::error::Error>> {
        let target = [0.0, 0.0, 0.0, self.current_position[3]]; // Keep E position
        let axes = axes.unwrap_or([true, true, true]); // Home all by default
        
        // Create home move for each axis
        for (i, &home_axis) in axes.iter().enumerate() {
            if home_axis {
                let mut home_target = self.current_position;
                home_target[i] = 0.0; // Move to home position
                
                self.plan_linear_move(
                    home_target,
                    50.0, // Slow homing speed
                    MotionType::Home,
                ).await?;
            }
        }
        
        Ok(())
    }

    /// Queue an extruder move (retract/prime)
    pub async fn plan_extruder_move(
        &mut self,
        target_e: f64,
        feedrate: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut target = self.current_position;
        target[3] = target_e;
        
        self.plan_linear_move(
            target,
            feedrate,
            MotionType::Extruder,
        ).await?;
        
        Ok(())
    }

    /// Get current motion queue length
    pub fn queue_length(&self) -> usize {
        self.motion_queue.len()
    }

    /// Clear all queued motions (emergency stop)
    pub fn clear_queue(&mut self) {
        self.motion_queue.clear();
        self.planner_state.current_segment = None;
        self.planner_state.segment_time = 0.0;
    }

    /// Set current position (used after homing)
    pub fn set_position(&mut self, position: [f64; 4]) {
        self.current_position = position;
    }
}