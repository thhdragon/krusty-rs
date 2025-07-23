// src/gcode/mod.rs - Fixed G-code processor
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::PrinterState;
use crate::motion::MotionController;

pub struct GCodeProcessor {
    state: Arc<RwLock<PrinterState>>,
    motion_controller: MotionController,
    command_queue: std::collections::VecDeque<String>,
}

impl GCodeProcessor {
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        motion_controller: MotionController,
    ) -> Self {
        Self {
            state,
            motion_controller,
            command_queue: std::collections::VecDeque::new(),
        }
    }

    pub async fn process_command(&mut self, command: &str) -> Result<(), Box<dyn std::error::Error>> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        
        if parts.is_empty() {
            return Ok(());
        }
        
        match parts[0] {
            "G1" | "G0" => self.handle_linear_move(&parts).await?,
            "G28" => self.handle_home().await?,
            "M104" => self.handle_set_hotend_temp(&parts).await?,
            "M140" => self.handle_set_bed_temp(&parts).await?,
            _ => println!("Unhandled G-code: {}", command),
        }
        
        Ok(())
    }

    async fn handle_linear_move(&mut self, parts: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        let mut x = None;
        let mut y = None;
        let mut z = None;
        let mut e = None;
        let mut f = None;
        
        for part in parts.iter().skip(1) {
            let param = part.chars().next().unwrap_or(' ');
            let value: f64 = part[1..].parse().unwrap_or(0.0);
            
            match param {
                'X' => x = Some(value),
                'Y' => y = Some(value),
                'Z' => z = Some(value),
                'E' => e = Some(value),
                'F' => f = Some(value),
                _ => {}
            }
        }
        
        // Get current position for relative moves
        let current_pos = self.get_current_position().await;
        let target_x = x.unwrap_or(current_pos[0]);
        let target_y = y.unwrap_or(current_pos[1]);
        let target_z = z.unwrap_or(current_pos[2]);
        
        self.motion_controller
            .queue_move([target_x, target_y, target_z], f, e)
            .await?;
        
        Ok(())
    }

    async fn handle_home(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.motion_controller.queue_home().await?;
        Ok(())
    }

    async fn handle_set_hotend_temp(&mut self, parts: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(temp_str) = parts.iter().find(|p| p.starts_with('S')) {
            let temp: f64 = temp_str[1..].parse().unwrap_or(0.0);
            println!("Setting hotend temperature to {:.1}°C", temp);
        }
        Ok(())
    }

    async fn handle_set_bed_temp(&mut self, parts: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(temp_str) = parts.iter().find(|p| p.starts_with('S')) {
            let temp: f64 = temp_str[1..].parse().unwrap_or(0.0);
            println!("Setting bed temperature to {:.1}°C", temp);
        }
        Ok(())
    }

    async fn get_current_position(&self) -> [f64; 3] {
        let state = self.state.read().await;
        state.position
    }
}

impl Clone for GCodeProcessor {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            motion_controller: self.motion_controller.clone(),
            command_queue: std::collections::VecDeque::new(),
        }
    }
}