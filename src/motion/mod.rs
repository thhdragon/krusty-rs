// src/motion/mod.rs - Fixed motion controller
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::PrinterState;
use crate::hardware::HardwareManager;

pub struct MotionController {
    state: Arc<RwLock<PrinterState>>,
    hardware_manager: HardwareManager,
    current_position: [f64; 4], // X, Y, Z, E
    move_queue: std::collections::VecDeque<MotionCommand>,
}

#[derive(Debug, Clone)]
pub struct MotionCommand {
    pub target: [f64; 4],
    pub feedrate: f64,
    pub command_type: CommandType,
}

#[derive(Debug, Clone)]
pub enum CommandType {
    LinearMove,
    Home,
    ExtruderMove,
}

impl MotionController {
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        hardware_manager: HardwareManager,
    ) -> Self {
        Self {
            state,
            hardware_manager,
            current_position: [0.0, 0.0, 0.0, 0.0],
            move_queue: std::collections::VecDeque::new(),
        }
    }

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
        
        let command = MotionCommand {
            target: target_4d,
            feedrate,
            command_type: CommandType::LinearMove,
        };
        
        self.move_queue.push_back(command);
        println!("Queued linear move to [{:.3}, {:.3}, {:.3}, {:.3}] at {:.1}mm/s",
                target_4d[0], target_4d[1], target_4d[2], target_4d[3], feedrate);
        
        Ok(())
    }

    pub async fn queue_home(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let command = MotionCommand {
            target: [0.0, 0.0, 0.0, self.current_position[3]],
            feedrate: 50.0, // Slow homing speed
            command_type: CommandType::Home,
        };
        
        self.move_queue.push_back(command);
        println!("Queued home command");
        
        Ok(())
    }

    pub async fn queue_extruder_move(
        &mut self,
        amount: f64,
        feedrate: Option<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let target_e = self.current_position[3] + amount;
        let feedrate = feedrate.unwrap_or(20.0);
        
        let command = MotionCommand {
            target: [self.current_position[0], self.current_position[1], self.current_position[2], target_e],
            feedrate,
            command_type: CommandType::ExtruderMove,
        };
        
        self.move_queue.push_back(command);
        println!("Queued extruder move: {:.3}mm at {:.1}mm/s", amount, feedrate);
        
        Ok(())
    }

    pub async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Process queued moves
        if let Some(command) = self.move_queue.pop_front() {
            self.execute_move(command).await?;
        }
        Ok(())
    }

    async fn execute_move(&mut self, command: MotionCommand) -> Result<(), Box<dyn std::error::Error>> {
        match command.command_type {
            CommandType::LinearMove => {
                // Update position
                self.current_position = command.target;
                
                // Update printer state
                {
                    let mut state = self.state.write().await;
                    state.position = [
                        self.current_position[0],
                        self.current_position[1],
                        self.current_position[2],
                    ];
                }
                
                println!("Executed move to [{:.3}, {:.3}, {:.3}, {:.3}]",
                        command.target[0], command.target[1], command.target[2], command.target[3]);
            }
            CommandType::Home => {
                self.current_position = command.target;
                {
                    let mut state = self.state.write().await;
                    state.position = [0.0, 0.0, 0.0];
                }
                println!("Executed homing");
            }
            CommandType::ExtruderMove => {
                self.current_position[3] = command.target[3];
                println!("Executed extruder move");
            }
        }
        Ok(())
    }

    pub fn emergency_stop(&mut self) {
        self.move_queue.clear();
        println!("Emergency stop - cleared motion queue");
    }

    pub fn get_current_position(&self) -> [f64; 4] {
        self.current_position
    }

    pub fn get_queue_length(&self) -> usize {
        self.move_queue.len()
    }
}

impl Clone for MotionController {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            hardware_manager: self.hardware_manager.clone(),
            current_position: self.current_position,
            move_queue: std::collections::VecDeque::new(), // Start with empty queue
        }
    }
}