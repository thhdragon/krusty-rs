// src/motion/advanced_planner.rs
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::PrinterState;
use crate::hardware::HardwareManager;
use crate::motion::kinematics::{Kinematics, KinematicsType, create_kinematics};
use crate::motion::junction::JunctionDeviation;
use crate::motion::shaper::ShaperConfig;

/// Advanced motion planner with junction deviation and input shaping
pub struct AdvancedMotionPlanner {
    /// Shared printer state
    state: Arc<RwLock<PrinterState>>,
    
    /// Hardware interface
    hardware_manager: HardwareManager,
    
    /// Motion configuration
    config: MotionConfig,
    
    /// Current Cartesian position
    current_position: [f64; 4],
    
    /// Planned motion blocks
    motion_queue: VecDeque<MotionBlock>,
    
    /// Kinematics handler
    kinematics: Box<dyn Kinematics>,
    
    /// Junction deviation calculator
    junction_deviation: JunctionDeviation,
    
    /// Input shaper configuration
    shaper_config: Option<ShaperConfig>,
    
    /// Previous unit vector for junction calculations
    previous_unit_vector: Option<[f64; 4]>,
    
    /// Planner state
    planner_state: PlannerState,
}

/// A motion block with full planning information
#[derive(Debug, Clone)]
pub struct MotionBlock {
    /// Target Cartesian position [X, Y, Z, E]
    pub target: [f64; 4],
    
    /// Target motor positions
    pub motor_target: [f64; 4],
    
    /// Requested feedrate (mm/s)
    pub requested_feedrate: f64,
    
    /// Limited feedrate after acceleration and junction limits
    pub limited_feedrate: f64,
    
    /// Distance of move (mm)
    pub distance: f64,
    
    /// Duration of move (seconds)
    pub duration: f64,
    
    /// Acceleration for this move (mm/s²)
    pub acceleration: f64,
    
    /// Entry speed into this block (mm/s)
    pub entry_speed: f64,
    
    /// Exit speed from this block (mm/s)
    pub exit_speed: f64,
    
    /// Type of motion
    pub motion_type: MotionType,
    
    /// Whether this block has been optimized
    pub optimized: bool,
}

/// Motion configuration with advanced parameters
#[derive(Debug, Clone)]
pub struct MotionConfig {
    /// Maximum velocity for each axis [X, Y, Z, E] (mm/s)
    pub max_velocity: [f64; 4],
    
    /// Maximum acceleration for each axis [X, Y, Z, E] (mm/s²)
    pub max_acceleration: [f64; 4],
    
    /// Maximum jerk for each axis [X, Y, Z, E] (mm/s³)
    pub max_jerk: [f64; 4],
    
    /// Junction deviation (mm)
    pub junction_deviation: f64,
    
    /// Axis limits [min, max] for X, Y, Z
    pub axis_limits: [[f64; 2]; 3],
    
    /// Printer kinematics type
    pub kinematics_type: KinematicsType,
    
    /// Minimum step distance (mm)
    pub minimum_step_distance: f64,
    
    /// Lookahead buffer size
    pub lookahead_buffer_size: usize,
}

impl MotionConfig {
    pub fn new_from_config(config: &crate::config::Config) -> Self {
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
            max_jerk: [10.0, 10.0, 0.4, 2.0],
            junction_deviation: 0.05, // 50 microns
            axis_limits: [[0.0, 200.0], [0.0, 200.0], [0.0, 200.0]], // Default 200mm
            kinematics_type: KinematicsType::Cartesian,
            minimum_step_distance: 0.001,
            lookahead_buffer_size: 32,
        }
    }
}

impl Default for MotionConfig {
    fn default() -> Self {
        Self {
            max_velocity: [300.0, 300.0, 20.0, 50.0],
            max_acceleration: [3000.0, 3000.0, 100.0, 1000.0],
            max_jerk: [10.0, 10.0, 0.4, 2.0],
            junction_deviation: 0.05,
            axis_limits: [[0.0, 200.0], [0.0, 200.0], [0.0, 200.0]],
            kinematics_type: KinematicsType::Cartesian,
            minimum_step_distance: 0.001,
            lookahead_buffer_size: 32,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MotionType {
    Print,
    Travel,
    Home,
    Extruder,
}

#[derive(Debug, Clone)]
struct PlannerState {
    active: bool,
    current_block: Option<MotionBlock>,
    block_time: f64,
    last_update: std::time::Instant,
}

impl AdvancedMotionPlanner {
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        hardware_manager: HardwareManager,
        config: MotionConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let kinematics = create_kinematics(
            config.kinematics_type,
            config.axis_limits,
        );
        
        let junction_deviation = JunctionDeviation::new(config.junction_deviation);
        
        Ok(Self {
            state,
            hardware_manager,
            config,
            current_position: [0.0, 0.0, 0.0, 0.0],
            motion_queue: VecDeque::new(),
            kinematics,
            junction_deviation,
            shaper_config: None,
            previous_unit_vector: None,
            planner_state: PlannerState {
                active: false,
                current_block: None,
                block_time: 0.0,
                last_update: std::time::Instant::now(),
            },
        })
    }

    /// Plan a linear move with advanced features
    pub async fn plan_advanced_move(
        &mut self,
        target: [f64; 4],
        feedrate: f64,
        motion_type: MotionType,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Validate target position
        let cartesian_target = [target[0], target[1], target[2]];
        if !self.kinematics.is_valid_position(&cartesian_target) {
            return Err("Target position outside axis limits".into());
        }
        
        // Convert to motor coordinates
        let motor_target = self.kinematics.cartesian_to_motors(&cartesian_target)?;
        
        // Calculate move parameters
        let distance = self.calculate_distance(&self.current_position, &target);
        
        if distance < self.config.minimum_step_distance {
            return Ok(());
        }
        
        // Create initial motion block
        let mut block = MotionBlock {
            target,
            motor_target,
            requested_feedrate: feedrate,
            limited_feedrate: feedrate,
            distance,
            duration: 0.0,
            acceleration: self.calculate_block_acceleration(&target),
            entry_speed: 0.0,
            exit_speed: 0.0,
            motion_type,
            optimized: false,
        };
        
        // Apply acceleration limits
        block.limited_feedrate = self.limit_feedrate_by_acceleration(&block);
        
        // Add to queue
        self.motion_queue.push_back(block);
        
        // Trigger optimization when queue is full enough
        if self.motion_queue.len() >= self.config.lookahead_buffer_size / 2 {
            self.optimize_queue().await?;
        }
        
        Ok(())
    }

    /// Optimize motion queue with junction deviation and lookahead
    async fn optimize_queue(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let queue_len = self.motion_queue.len();
        if queue_len < 2 {
            return Ok(());
        }
        
        tracing::debug!("Optimizing {} motion blocks", queue_len);
        
        // Convert to vector for easier manipulation
        let mut blocks: Vec<MotionBlock> = self.motion_queue.drain(..).collect();
        
        // Forward pass: calculate entry/exit speeds
        self.forward_pass(&mut blocks)?;
        
        // Backward pass: optimize speeds
        self.backward_pass(&mut blocks)?;
        
        // Recalculate durations with optimized speeds
        self.recalculate_durations(&mut blocks)?;
        
        // Put optimized blocks back in queue
        for block in blocks {
            self.motion_queue.push_back(block);
        }
        
        Ok(())
    }

    /// Forward pass through motion blocks
    fn forward_pass(&mut self, blocks: &mut [MotionBlock]) -> Result<(), Box<dyn std::error::Error>> {
        let mut previous_unit_vector = self.previous_unit_vector;
        
        for i in 0..blocks.len() {
            // Calculate unit vector for this move
            let unit_vector = if i == 0 {
                // First block - use current to target
                JunctionDeviation::calculate_unit_vector(&self.current_position, &blocks[i].target)
            } else {
                // Subsequent blocks - previous target to current target
                JunctionDeviation::calculate_unit_vector(&blocks[i-1].target, &blocks[i].target)
            };
            
            // Calculate junction speed if we have previous vector
            if let Some(prev_unit) = previous_unit_vector {
                let junction_speed = self.junction_deviation.calculate_junction_speed(
                    &prev_unit,
                    &unit_vector,
                    blocks[i].acceleration,
                );
                
                // Limit entry speed by junction deviation
                blocks[i].entry_speed = blocks[i].entry_speed.min(junction_speed);
            }
            
            // Calculate maximum exit speed based on acceleration and distance
            let max_exit_speed = ((blocks[i].entry_speed * blocks[i].entry_speed) + 
                                 2.0 * blocks[i].acceleration * blocks[i].distance).sqrt();
            
            blocks[i].exit_speed = blocks[i].limited_feedrate.min(max_exit_speed);
            
            // Update for next iteration
            previous_unit_vector = Some(unit_vector);
        }
        
        // Store last unit vector for next optimization
        self.previous_unit_vector = previous_unit_vector;
        
        Ok(())
    }

    /// Backward pass through motion blocks
    fn backward_pass(&mut self, blocks: &mut [MotionBlock]) -> Result<(), Box<dyn std::error::Error>> {
        for i in (0..blocks.len()).rev() {
            let exit_speed = if i == blocks.len() - 1 {
                // Last block - assume it can decelerate to zero
                0.0
            } else {
                // Previous block's entry speed
                blocks[i + 1].entry_speed
            };
            
            // Calculate maximum entry speed based on exit speed and deceleration
            let max_entry_speed = ((exit_speed * exit_speed) + 
                                  2.0 * blocks[i].acceleration * blocks[i].distance).sqrt();
            
            // Limit entry speed
            blocks[i].entry_speed = blocks[i].entry_speed.min(max_entry_speed);
            
            // Recalculate exit speed based on limited entry speed
            blocks[i].exit_speed = ((blocks[i].entry_speed * blocks[i].entry_speed) + 
                                   2.0 * blocks[i].acceleration * blocks[i].distance).sqrt()
                                   .min(blocks[i].limited_feedrate);
        }
        
        Ok(())
    }

    /// Recalculate block durations with optimized speeds
    fn recalculate_durations(&mut self, blocks: &mut [MotionBlock]) -> Result<(), Box<dyn std::error::Error>> {
        for block in blocks {
            // For trapezoidal profile: t = (v_end - v_start + sqrt((v_end - v_start)² + 2*a*d)) / a
            let delta_v = block.exit_speed - block.entry_speed;
            let discriminant = delta_v * delta_v + 2.0 * block.acceleration * block.distance;
            
            if discriminant >= 0.0 {
                let time = (delta_v + discriminant.sqrt()) / block.acceleration;
                block.duration = time.max(0.0);
            } else {
                // Fallback - should not happen with proper optimization
                block.duration = block.distance / block.limited_feedrate;
            }
            
            block.optimized = true;
        }
        
        Ok(())
    }

    /// Limit feedrate by acceleration capabilities
    fn limit_feedrate_by_acceleration(&self, block: &MotionBlock) -> f64 {
        // Calculate unit vector for this move
        let unit_vector = JunctionDeviation::calculate_unit_vector(&self.current_position, &block.target);
        
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
        let acceleration_limited_feedrate = (2.0 * max_acceleration * block.distance).sqrt();
        
        block.requested_feedrate.min(acceleration_limited_feedrate)
    }

    /// Calculate appropriate acceleration for a move
    fn calculate_block_acceleration(&self, target: &[f64; 4]) -> f64 {
        let distance = self.calculate_distance(&self.current_position, target);
        if distance == 0.0 {
            return self.config.max_acceleration[0];
        }
        
        let unit_vector = JunctionDeviation::calculate_unit_vector(&self.current_position, target);
        
        // Weighted average based on axis movement
        let weighted_accel = 
            unit_vector[0].abs() * self.config.max_acceleration[0] +
            unit_vector[1].abs() * self.config.max_acceleration[1] +
            unit_vector[2].abs() * self.config.max_acceleration[2] +
            unit_vector[3].abs() * self.config.max_acceleration[3];
        
        weighted_accel
    }

    /// Calculate 3D Euclidean distance
    fn calculate_distance(&self, start: &[f64; 4], end: &[f64; 4]) -> f64 {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let de = end[3] - start[3];
        
        (dx * dx + dy * dy + dz * dz + de * de).sqrt()
    }

    /// Main update loop for motion execution
    pub async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = std::time::Instant::now();
        let dt = (now - self.planner_state.last_update).as_secs_f64();
        self.planner_state.last_update = now;

        // Check if we need to start a new block
        if self.planner_state.current_block.is_none() {
            if let Some(block) = self.motion_queue.pop_front() {
                self.planner_state.current_block = Some(block);
                self.planner_state.block_time = 0.0;
                self.planner_state.active = true;
            } else {
                self.planner_state.active = false;
                return Ok(());
            }
        }

        // Instead of borrowing and then assigning, check existence first
        let has_current_block = self.planner_state.current_block.is_some();
        if has_current_block {
            // Take ownership of the block temporarily
            if let Some(block) = self.planner_state.current_block.take() {
                self.planner_state.block_time += dt;

                // Check if block is complete
                if self.planner_state.block_time >= block.duration {
                    // Block complete - update position
                    self.current_position = block.target;

                    // Update printer state
                    {
                        let mut state = self.state.write().await;
                        state.position = [
                            self.current_position[0],
                            self.current_position[1],
                            self.current_position[2],
                        ];
                    }

                    // Clear current block and reset block time
                    self.planner_state.current_block = None;
                    self.planner_state.block_time = 0.0;

                    tracing::debug!(
                        "Completed move to [{:.3}, {:.3}, {:.3}, {:.3}]",
                        self.current_position[0], self.current_position[1], self.current_position[2], self.current_position[3]
                    );
                } else {
                    // Interpolate position within block
                    let progress = self.planner_state.block_time / block.duration;
                    let current_pos = [
                        self.current_position[0] + (block.target[0] - self.current_position[0]) * progress,
                        self.current_position[1] + (block.target[1] - self.current_position[1]) * progress,
                        self.current_position[2] + (block.target[2] - self.current_position[2]) * progress,
                        self.current_position[3] + (block.target[3] - self.current_position[3]) * progress,
                    ];
                    self.generate_steps(&current_pos, &block).await?;
                    // Put the block back for the next update
                    self.planner_state.current_block = Some(block);
                }
            }
        }

        Ok(())
    }

    /// Generate step commands for current position
    async fn generate_steps(
        &self,
        position: &[f64; 4],
        block: &MotionBlock,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Convert Cartesian position to motor positions
        let cartesian = [position[0], position[1], position[2]];
        let motor_positions = self.kinematics.cartesian_to_motors(&cartesian)?;
        
        // In real implementation, this would:
        // 1. Convert motor positions to step counts
        // 2. Apply input shaping if configured
        // 3. Send step commands to MCU
        
        tracing::trace!(
            "Position: [{:.3}, {:.3}, {:.3}, {:.3}] Motors: [{:.3}, {:.3}, {:.3}, {:.3}]",
            position[0], position[1], position[2], position[3],
            motor_positions[0], motor_positions[1], motor_positions[2], motor_positions[3]
        );
        
        Ok(())
    }

    /// Set input shaper configuration
    pub fn set_input_shaper(&mut self, shaper_config: Option<ShaperConfig>) {
        self.shaper_config = shaper_config;
    }

    /// Clear motion queue (emergency stop)
    pub fn clear_queue(&mut self) {
        self.motion_queue.clear();
        self.planner_state.current_block = None;
        self.planner_state.block_time = 0.0;
    }

    /// Set current position (after homing)
    pub fn set_position(&mut self, position: [f64; 4]) {
        self.current_position = position;
    }

    /// Get queue length
    pub fn queue_length(&self) -> usize {
        self.motion_queue.len()
    }
}