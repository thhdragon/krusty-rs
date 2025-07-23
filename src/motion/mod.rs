// src/motion/mod.rs - Complete integrated motion system
pub mod kinematics;
pub mod junction;
pub mod shaper;
pub mod trajectory;
pub mod stepper;
pub mod snap_crackle;

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::PrinterState;
use crate::hardware::HardwareManager;
use crate::motion::kinematics::{Kinematics, KinematicsType, create_kinematics};
use crate::motion::junction::JunctionDeviation;
use crate::motion::shaper::ShaperConfig;
use crate::motion::snap_crackle::SnapCrackleMotion;

/// Complete motion controller integrating all systems
pub struct MotionController {
    /// Shared printer state
    state: Arc<RwLock<PrinterState>>,
    
    /// Hardware interface
    hardware_manager: HardwareManager,
    
    /// Motion configuration
    config: MotionConfig,
    
    /// Current position [X, Y, Z, E]
    current_position: [f64; 4],
    
    /// Motion queue
    motion_queue: MotionQueue,
    
    /// Kinematics handler
    kinematics: Box<dyn Kinematics>,
    
    /// Junction deviation calculator
    junction_deviation: JunctionDeviation,
    
    /// Input shaper
    input_shaper: Option<ShaperConfig>,
    
    /// Snap/Crackle motion system
    snap_crackle: SnapCrackleMotion,
    
    /// Motion planner
    planner: MotionPlanner,
    
    /// Step generator
    step_generator: StepGenerator,
}

/// Motion configuration
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
    pub snap_crackle_enabled: bool,
    pub max_snap: f64,
    pub max_crackle: f64,
}

impl MotionConfig {
    pub fn new_from_host_config(config: &crate::config::Config) -> Self {
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
            max_jerk: [
                config.motion.max_jerk,
                config.motion.max_jerk,
                config.motion.max_jerk * 0.025, // Z jerk typically much lower
                2.0, // Extruder jerk
            ],
            junction_deviation: config.motion.junction_deviation,
            axis_limits: [
                [0.0, config.printer.bed_size[0]], // X limits
                [0.0, config.printer.bed_size[1]], // Y limits
                [0.0, 200.0], // Z limits (would come from config)
            ],
            kinematics_type: match config.printer.kinematics.as_str() {
                "corexy" => KinematicsType::CoreXY,
                "delta" => KinematicsType::Delta,
                _ => KinematicsType::Cartesian,
            },
            minimum_step_distance: 0.001,
            lookahead_buffer_size: 32,
            snap_crackle_enabled: config.motion.snap_crackle_enabled,
            max_snap: config.motion.max_snap,
            max_crackle: config.motion.max_crackle,
        }
    }
}

/// Motion queue managing all pending moves
pub struct MotionQueue {
    /// Queued motion blocks
    blocks: std::collections::VecDeque<MotionBlock>,
    
    /// Currently executing block
    current_block: Option<ExecutingBlock>,
    
    /// Queue statistics
    stats: QueueStats,
}

/// A planned motion block
#[derive(Debug, Clone)]
pub struct MotionBlock {
    pub target: [f64; 4],
    pub motor_target: [f64; 4],
    pub requested_feedrate: f64,
    pub limited_feedrate: f64,
    pub distance: f64,
    pub duration: f64,
    pub acceleration: f64,
    pub entry_speed: f64,
    pub exit_speed: f64,
    pub motion_type: MotionType,
    pub optimized: bool,
    pub timestamp: std::time::Instant,
}

/// Currently executing motion block with real-time state
pub struct ExecutingBlock {
    pub block: MotionBlock,
    pub start_time: std::time::Instant,
    pub current_time: f64,
    pub current_position: [f64; 4],
    pub steps_generated: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MotionType {
    Print,
    Travel,
    Home,
    Extruder,
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub length: usize,
    pub total_distance: f64,
    pub estimated_completion_time: f64,
    pub average_feedrate: f64,
}

impl MotionQueue {
    pub fn new() -> Self {
        Self {
            blocks: std::collections::VecDeque::new(),
            current_block: None,
            stats: QueueStats {
                length: 0,
                total_distance: 0.0,
                estimated_completion_time: 0.0,
                average_feedrate: 0.0,
            },
        }
    }

    pub fn push(&mut self, block: MotionBlock) {
        self.blocks.push_back(block);
        self.update_stats();
    }

    pub fn pop(&mut self) -> Option<MotionBlock> {
        let block = self.blocks.pop_front();
        self.update_stats();
        block
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn clear(&mut self) {
        self.blocks.clear();
        self.current_block = None;
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.length = self.blocks.len();
        self.stats.total_distance = self.blocks.iter().map(|b| b.distance).sum();
        self.stats.estimated_completion_time = self.blocks.iter().map(|b| b.duration).sum();
        if self.stats.total_distance > 0.0 && self.stats.estimated_completion_time > 0.0 {
            self.stats.average_feedrate = self.stats.total_distance / self.stats.estimated_completion_time;
        }
    }

    pub fn get_stats(&self) -> &QueueStats {
        &self.stats
    }
}

impl MotionController {
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
        
        let snap_crackle = SnapCrackleMotion::new(
            config.max_snap,
            config.max_crackle,
        );
        
        let planner = MotionPlanner::new(&config);
        let step_generator = StepGenerator::new([80.0, 80.0, 400.0, 100.0], [false, false, false, false]);
        
        Ok(Self {
            state,
            hardware_manager,
            config,
            current_position: [0.0, 0.0, 0.0, 0.0],
            motion_queue: MotionQueue::new(),
            kinematics,
            junction_deviation,
            input_shaper: None,
            snap_crackle,
            planner,
            step_generator,
        })
    }

    /// Queue a linear move
    pub async fn queue_linear_move(
        &mut self,
        target: [f64; 3],
        feedrate: Option<f64>,
        extrude: Option<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let current_e = self.current_position[3];
        let target_e = if let Some(e) = extrude {
            current_e + e
        } else {
            current_e
        };
        
        let feedrate = feedrate.unwrap_or(300.0);
        let target_4d = [target[0], target[1], target[2], target_e];
        
        let motion_type = if extrude.is_some() && extrude.unwrap() > 0.0 {
            MotionType::Print
        } else {
            MotionType::Travel
        };
        
        // Plan the move
        let block = self.planner.plan_move(
            self.current_position,
            target_4d,
            feedrate,
            motion_type,
            &self.config,
        )?;
        
        self.motion_queue.push(block);
        self.optimize_queue().await?;
        
        Ok(())
    }

    /// Queue a homing operation
    pub async fn queue_home(&mut self, axes: Option<[bool; 3]>) -> Result<(), Box<dyn std::error::Error>> {
        let axes = axes.unwrap_or([true, true, true]);
        let target = [0.0, 0.0, 0.0, self.current_position[3]];
        
        for (i, &home_axis) in axes.iter().enumerate() {
            if home_axis {
                let mut home_target = self.current_position;
                home_target[i] = 0.0;
                
                let block = self.planner.plan_move(
                    self.current_position,
                    home_target,
                    50.0,
                    MotionType::Home,
                    &self.config,
                )?;
                
                self.motion_queue.push(block);
            }
        }
        
        Ok(())
    }

    /// Optimize motion queue
    async fn optimize_queue(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.motion_queue.len() < 2 {
            return Ok(());
        }
        
        // Convert to vector for optimization
        let mut blocks: Vec<MotionBlock> = std::collections::VecDeque::new()
            .extend(self.motion_queue.blocks.drain(..)).into_iter().collect();
        
        // Apply junction deviation optimization
        self.optimize_junctions(&mut blocks)?;
        
        // Clear and rebuild queue
        self.motion_queue.blocks.clear();
        for block in blocks {
            self.motion_queue.blocks.push_back(block);
        }
        
        Ok(())
    }

    /// Apply junction deviation optimization
    fn optimize_junctions(&self, blocks: &mut [MotionBlock]) -> Result<(), Box<dyn std::error::Error>> {
        let mut previous_unit_vector = None;
        
        for i in 0..blocks.len() {
            // Calculate unit vector for this move
            let unit_vector = if i == 0 {
                self.calculate_unit_vector(&self.current_position, &blocks[i].target)
            } else {
                self.calculate_unit_vector(&blocks[i-1].target, &blocks[i].target)
            };
            
            // Apply junction deviation if we have previous vector
            if let Some(prev_unit) = previous_unit_vector {
                let junction_speed = self.junction_deviation.calculate_junction_speed(
                    &prev_unit,
                    &unit_vector,
                    blocks[i].acceleration,
                );
                
                blocks[i].entry_speed = blocks[i].entry_speed.min(junction_speed);
            }
            
            previous_unit_vector = Some(unit_vector);
        }
        
        Ok(())
    }

    /// Calculate unit vector between two points
    fn calculate_unit_vector(&self, start: &[f64; 4], end: &[f64; 4]) -> [f64; 4] {
        let mut delta = [
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

    /// Main update function - called at high frequency
    pub async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Start new block if needed
        if self.motion_queue.current_block.is_none() {
            if let Some(block) = self.motion_queue.pop() {
                let executing_block = ExecutingBlock {
                    block,
                    start_time: std::time::Instant::now(),
                    current_time: 0.0,
                    current_position: self.current_position,
                    steps_generated: 0,
                };
                self.motion_queue.current_block = Some(executing_block);
            }
        }
        
        // Process current block
        if let Some(ref mut executing) = self.motion_queue.current_block {
            let now = std::time::Instant::now();
            let dt = (now - executing.start_time).as_secs_f64() - executing.current_time;
            executing.current_time += dt;
            
            // Check if block is complete
            if executing.current_time >= executing.block.duration {
                // Block complete
                self.current_position = executing.block.target;
                
                // Update printer state
                {
                    let mut state = self.state.write().await;
                    state.position = [
                        self.current_position[0],
                        self.current_position[1],
                        self.current_position[2],
                    ];
                }
                
                self.motion_queue.current_block = None;
            } else {
                // Interpolate position and generate steps
                let progress = executing.current_time / executing.block.duration;
                let current_pos = self.interpolate_position(&executing.block, progress);
                
                // Generate and send steps
                self.generate_and_send_steps(&current_pos).await?;
                
                executing.current_position = current_pos;
            }
        }
        
        Ok(())
    }

    /// Interpolate position within a motion block
    fn interpolate_position(&self, block: &MotionBlock, progress: f64) -> [f64; 4] {
        [
            self.current_position[0] + (block.target[0] - self.current_position[0]) * progress,
            self.current_position[1] + (block.target[1] - self.current_position[1]) * progress,
            self.current_position[2] + (block.target[2] - self.current_position[2]) * progress,
            self.current_position[3] + (block.target[3] - self.current_position[3]) * progress,
        ]
    }

    /// Generate and send step commands
    async fn generate_and_send_steps(&mut self, position: &[f64; 4]) -> Result<(), Box<dyn std::error::Error>> {
        // Convert to motor positions
        let cartesian = [position[0], position[1], position[2]];
        let motor_positions = self.kinematics.cartesian_to_motors(&cartesian)?;
        
        // Generate step commands
        let steps = self.step_generator.generate_steps(position);
        
        // Send to hardware (in real implementation)
        for step_cmd in steps {
            let mcu_cmd = step_cmd.to_mcu_command();
            // self.hardware_manager.send_mcu_command(&mcu_cmd).await?;
            tracing::trace!("Step command: {}", mcu_cmd);
        }
        
        Ok(())
    }

    /// Emergency stop
    pub fn emergency_stop(&mut self) {
        self.motion_queue.clear();
        tracing::warn!("Emergency stop - motion queue cleared");
    }

    /// Get queue statistics
    pub fn get_queue_stats(&self) -> QueueStats {
        self.motion_queue.stats.clone()
    }

    /// Set current position (after homing)
    pub fn set_position(&mut self, x: f64, y: f64, z: f64, e: f64) {
        self.current_position = [x, y, z, e];
        self.step_generator.reset_steps();
    }
}

/// Motion planner for individual moves
pub struct MotionPlanner {
    config: MotionConfig,
}

impl MotionPlanner {
    pub fn new(config: &MotionConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub fn plan_move(
        &self,
        start: [f64; 4],
        target: [f64; 4],
        feedrate: f64,
        motion_type: MotionType,
        config: &MotionConfig,
    ) -> Result<MotionBlock, Box<dyn std::error::Error>> {
        let distance = self.calculate_distance(&start, &target);
        
        if distance < config.minimum_step_distance {
            return Err("Move distance too small".into());
        }
        
        // Convert to motor coordinates
        // let motor_target = self.kinematics.cartesian_to_motors(&[target[0], target[1], target[2]])?;
        
        let limited_feedrate = self.limit_feedrate(start, target, feedrate, config);
        
        let block = MotionBlock {
            target,
            motor_target: target, // Simplified
            requested_feedrate: feedrate,
            limited_feedrate,
            distance,
            duration: distance / limited_feedrate,
            acceleration: self.calculate_acceleration(start, target, config),
            entry_speed: 0.0,
            exit_speed: 0.0,
            motion_type,
            optimized: false,
            timestamp: std::time::Instant::now(),
        };
        
        Ok(block)
    }

    fn calculate_distance(&self, start: &[f64; 4], end: &[f64; 4]) -> f64 {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let de = end[3] - start[3];
        
        (dx * dx + dy * dy + dz * dz + de * de).sqrt()
    }

    fn limit_feedrate(
        &self,
        start: [f64; 4],
        target: [f64; 4],
        requested_feedrate: f64,
        config: &MotionConfig,
    ) -> f64 {
        let distance = self.calculate_distance(&start, &target);
        if distance == 0.0 {
            return requested_feedrate;
        }
        
        let unit_vector = [
            (target[0] - start[0]) / distance,
            (target[1] - start[1]) / distance,
            (target[2] - start[2]) / distance,
            (target[3] - start[3]) / distance,
        ];
        
        let mut max_acceleration = f64::INFINITY;
        for i in 0..4 {
            let axis_component = unit_vector[i].abs();
            if axis_component > 0.0 {
                let axis_accel_limit = config.max_acceleration[i] / axis_component;
                max_acceleration = max_acceleration.min(axis_accel_limit);
            }
        }
        
        let acceleration_limited_feedrate = (2.0 * max_acceleration * distance).sqrt();
        
        requested_feedrate.min(acceleration_limited_feedrate)
    }

    fn calculate_acceleration(
        &self,
        start: [f64; 4],
        target: [f64; 4],
        config: &MotionConfig,
    ) -> f64 {
        let distance = self.calculate_distance(&start, &target);
        if distance == 0.0 {
            return config.max_acceleration[0];
        }
        
        let unit_vector = [
            (target[0] - start[0]) / distance,
            (target[1] - start[1]) / distance,
            (target[2] - start[2]) / distance,
            (target[3] - start[3]) / distance,
        ];
        
        let weighted_accel = 
            unit_vector[0].abs() * config.max_acceleration[0] +
            unit_vector[1].abs() * config.max_acceleration[1] +
            unit_vector[2].abs() * config.max_acceleration[2] +
            unit_vector[3].abs() * config.max_acceleration[3];
        
        weighted_accel
    }
}

// Re-export key components
pub use self::stepper::StepGenerator;
pub use self::stepper::StepCommand;