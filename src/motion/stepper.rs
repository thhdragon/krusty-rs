// src/motion/stepper.rs - Complete step generator implementation
use std::collections::HashMap;

/// Complete step generator that converts motion positions to motor step commands
pub struct StepGenerator {
    /// Steps per mm for each axis
    steps_per_mm: [f64; 4], // [X, Y, Z, E]
    
    /// Direction pin inversion for each axis
    direction_invert: [bool; 4],
    
    /// Current step counts for each axis
    current_steps: [i64; 4],
    
    /// Last generated steps for delta calculation
    last_steps: [i64; 4],
    
    /// Step timing parameters
    step_timing: StepTiming,
    
    /// Step buffer for batch processing
    step_buffer: StepBuffer,
}

/// Step timing configuration
#[derive(Debug, Clone)]
pub struct StepTiming {
    /// Minimum step pulse width in microseconds
    pub pulse_width: u64,
    
    /// Minimum step interval in microseconds
    pub step_interval: u64,
    
    /// Direction setup time in microseconds
    pub direction_setup: u64,
    
    /// Enable signal timing
    pub enable_timing: EnableTiming,
}

/// Enable signal timing configuration
#[derive(Debug, Clone)]
pub struct EnableTiming {
    pub pre_enable_delay: u64,    // Delay before enabling
    pub post_step_delay: u64,     // Delay after step before disabling
    pub disable_delay: u64,       // Delay before disabling
}

/// Step buffer for efficient batch processing
pub struct StepBuffer {
    /// Buffered step commands
    commands: Vec<StepCommand>,
    
    /// Maximum buffer size
    max_size: usize,
    
    /// Current buffer position
    position: usize,
}

/// A single step command for precise motor control
#[derive(Debug, Clone)]
pub struct StepCommand {
    /// Axis identifier
    pub axis: Axis,
    
    /// Number of steps to take
    pub steps: u32,
    
    /// Direction (true = positive, false = negative)
    pub direction: bool,
    
    /// Timing information for this step
    pub timing: Option<StepTiming>,
    
    /// Step completion callback
    pub callback: Option<Box<dyn Fn() + Send>>,
}

/// Axis identifiers
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    X,
    Y,
    Z,
    E,
    Custom(u8),
}

impl Axis {
    pub fn to_string(&self) -> &'static str {
        match self {
            Axis::X => "X",
            Axis::Y => "Y",
            Axis::Z => "Z",
            Axis::E => "E",
            Axis::Custom(_) => "U",
        }
    }
    
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'X' | 'x' => Some(Axis::X),
            'Y' | 'y' => Some(Axis::Y),
            'Z' | 'z' => Some(Axis::Z),
            'E' | 'e' => Some(Axis::E),
            _ => None,
        }
    }
}

impl StepGenerator {
    /// Create a new step generator with specified parameters
    pub fn new(
        steps_per_mm: [f64; 4],
        direction_invert: [bool; 4],
    ) -> Self {
        Self {
            steps_per_mm,
            direction_invert,
            current_steps: [0; 4],
            last_steps: [0; 4],
            step_timing: StepTiming {
                pulse_width: 2,        // 2 microseconds
                step_interval: 5,      // 5 microseconds
                direction_setup: 1,    // 1 microsecond
                enable_timing: EnableTiming {
                    pre_enable_delay: 1000,   // 1ms
                    post_step_delay: 1000,    // 1ms
                    disable_delay: 5000,      // 5ms
                },
            },
            step_buffer: StepBuffer {
                commands: Vec::new(),
                max_size: 1000,
                position: 0,
            },
        }
    }

    /// Convert position in mm to step counts
    pub fn position_to_steps(&self, position: &[f64; 4]) -> [i64; 4] {
        let mut steps = [0i64; 4];
        
        for i in 0..4 {
            // Convert position to steps with proper rounding
            let step_count = position[i] * self.steps_per_mm[i];
            steps[i] = step_count.round() as i64;
        }
        
        steps
    }

    /// Generate step commands for movement to new position
    pub fn generate_steps(&mut self, new_position: &[f64; 4]) -> Vec<StepCommand> {
        // Convert new position to steps
        let target_steps = self.position_to_steps(new_position);
        
        // Calculate step deltas for each axis
        let mut step_deltas = [0i64; 4];
        for i in 0..4 {
            step_deltas[i] = target_steps[i] - self.current_steps[i];
        }
        
        // Store current steps for next calculation
        self.current_steps = target_steps;
        
        // Generate step commands only for axes that moved
        let mut commands = Vec::new();
        
        for (i, &delta) in step_deltas.iter().enumerate() {
            if delta != 0 {
                let steps = delta.abs() as u32;
                let direction = if delta > 0 {
                    !self.direction_invert[i]
                } else {
                    self.direction_invert[i]
                };
                
                let axis = match i {
                    0 => Axis::X,
                    1 => Axis::Y,
                    2 => Axis::Z,
                    3 => Axis::E,
                    _ => Axis::Custom(i as u8),
                };
                
                commands.push(StepCommand {
                    axis,
                    steps,
                    direction,
                    timing: Some(self.step_timing.clone()),
                    callback: None,
                });
            }
        }
        
        commands
    }

    /// Generate interpolated steps for smooth motion
    pub fn generate_interpolated_steps(
        &mut self,
        start_position: &[f64; 4],
        end_position: &[f64; 4],
        steps_per_segment: u32,
    ) -> Vec<Vec<StepCommand>> {
        let total_distance = self.calculate_distance(start_position, end_position);
        let segments = (total_distance * 1000.0) as u32 / steps_per_segment.max(1);
        let segments = segments.max(1);
        
        let mut all_commands = Vec::new();
        
        for i in 0..segments {
            let progress = (i + 1) as f64 / segments as f64;
            let interpolated_position = [
                start_position[0] + (end_position[0] - start_position[0]) * progress,
                start_position[1] + (end_position[1] - start_position[1]) * progress,
                start_position[2] + (end_position[2] - start_position[2]) * progress,
                start_position[3] + (end_position[3] - start_position[3]) * progress,
            ];
            
            let commands = self.generate_steps(&interpolated_position);
            if !commands.is_empty() {
                all_commands.push(commands);
            }
        }
        
        all_commands
    }

    /// Calculate 3D Euclidean distance between two positions
    fn calculate_distance(&self, start: &[f64; 4], end: &[f64; 4]) -> f64 {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let de = end[3] - start[3];
        
        (dx * dx + dy * dy + dz * dz + de * de).sqrt()
    }

    /// Add step command to buffer
    pub fn buffer_step(&mut self, command: StepCommand) -> Result<(), &'static str> {
        if self.step_buffer.commands.len() >= self.step_buffer.max_size {
            return Err("Step buffer full");
        }
        
        self.step_buffer.commands.push(command);
        Ok(())
    }

    /// Flush step buffer and return all buffered commands
    pub fn flush_buffer(&mut self) -> Vec<StepCommand> {
        let commands = self.step_buffer.commands.clone();
        self.step_buffer.commands.clear();
        self.step_buffer.position = 0;
        commands
    }

    /// Reset step counters (used after homing)
    pub fn reset_steps(&mut self) {
        self.current_steps = [0; 4];
        self.last_steps = [0; 4];
    }

    /// Get current step position
    pub fn get_current_steps(&self) -> [i64; 4] {
        self.current_steps
    }

    /// Set step timing parameters
    pub fn set_step_timing(&mut self, timing: StepTiming) {
        self.step_timing = timing;
    }

    /// Get step timing parameters
    pub fn get_step_timing(&self) -> &StepTiming {
        &self.step_timing
    }

    /// Calculate minimum time required for a set of steps
    pub fn calculate_minimum_time(&self, commands: &[StepCommand]) -> u64 {
        let mut total_time = 0u64;
        
        for command in commands {
            // Time for direction setup
            total_time += self.step_timing.direction_setup;
            
            // Time for steps
            total_time += command.steps as u64 * self.step_timing.step_interval;
            
            // Time for pulse width (overlaps with step interval)
            total_time += command.steps as u64 * self.step_timing.pulse_width;
        }
        
        total_time
    }
}

impl StepCommand {
    /// Convert to MCU command string
    pub fn to_mcu_command(&self) -> String {
        format!(
            "step {} {} {}",
            self.axis.to_string(),
            self.steps,
            if self.direction { "1" } else { "0" }
        )
    }
    
    /// Create step command with callback
    pub fn with_callback<F>(mut self, callback: F) -> Self 
    where 
        F: Fn() + Send + 'static,
    {
        self.callback = Some(Box::new(callback));
        self
    }
    
    /// Execute callback if present
    pub fn execute_callback(&self) {
        if let Some(ref callback) = self.callback {
            callback();
        }
    }
}

impl StepBuffer {
    /// Create new step buffer
    pub fn new(max_size: usize) -> Self {
        Self {
            commands: Vec::with_capacity(max_size),
            max_size,
            position: 0,
        }
    }
    
    /// Add command to buffer
    pub fn push(&mut self, command: StepCommand) -> Result<(), &'static str> {
        if self.commands.len() >= self.max_size {
            return Err("Buffer full");
        }
        
        self.commands.push(command);
        Ok(())
    }
    
    /// Get next command from buffer
    pub fn next(&mut self) -> Option<StepCommand> {
        if self.position < self.commands.len() {
            let command = self.commands[self.position].clone();
            self.position += 1;
            Some(command)
        } else {
            None
        }
    }
    
    /// Reset buffer position
    pub fn reset(&mut self) {
        self.position = 0;
    }
    
    /// Clear buffer
    pub fn clear(&mut self) {
        self.commands.clear();
        self.position = 0;
    }
    
    /// Get buffer length
    pub fn len(&self) -> usize {
        self.commands.len()
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

// Default implementations
impl Default for StepTiming {
    fn default() -> Self {
        Self {
            pulse_width: 2,
            step_interval: 5,
            direction_setup: 1,
            enable_timing: EnableTiming {
                pre_enable_delay: 1000,
                post_step_delay: 1000,
                disable_delay: 5000,
            },
        }
    }
}

impl Default for StepBuffer {
    fn default() -> Self {
        Self::new(1000)
    }
}