// src/motion/stepper.rs - Fixed duplicate Clone implementation
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct StepGenerator {
    steps_per_mm: [f64; 4],
    direction_invert: [bool; 4],
    current_steps: [i64; 4],
}

// Remove the manual Clone implementation since we're using #[derive(Clone)]

#[derive(Debug, Clone)]
pub struct StepCommand {
    pub axis: usize,
    pub steps: u32,
    pub direction: bool,
}

impl StepCommand {
    pub fn to_mcu_command(&self) -> String {
        let axis_name = match self.axis {
            0 => "X",
            1 => "Y",
            2 => "Z",
            3 => "E",
            _ => "U",
        };
        
        format!("step {} {} {}", axis_name, self.steps, if self.direction { 1 } else { 0 })
    }
}

impl StepGenerator {
    pub fn new(steps_per_mm: [f64; 4], direction_invert: [bool; 4]) -> Self {
        Self {
            steps_per_mm,
            direction_invert,
            current_steps: [0; 4],
        }
    }

    pub fn generate_steps(&mut self, position: &[f64; 4]) -> Vec<StepCommand> {
        let mut target_steps = [0i64; 4];
        for i in 0..4 {
            target_steps[i] = (position[i] * self.steps_per_mm[i]).round() as i64;
        }
        
        let mut step_deltas = [0i64; 4];
        for i in 0..4 {
            step_deltas[i] = target_steps[i] - self.current_steps[i];
        }
        
        self.current_steps = target_steps;
        
        let mut commands = Vec::new();
        
        for i in 0..4 {
            if step_deltas[i] != 0 {
                let steps = step_deltas[i].abs() as u32;
                let direction = if step_deltas[i] > 0 {
                    !self.direction_invert[i]
                } else {
                    self.direction_invert[i]
                };
                
                commands.push(StepCommand {
                    axis: i,
                    steps,
                    direction,
                });
            }
        }
        
        commands
    }

    pub fn reset_steps(&mut self) {
        self.current_steps = [0; 4];
    }
}

// No manual Clone implementation needed - #[derive(Clone)] handles it