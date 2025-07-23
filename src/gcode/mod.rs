// src/gcode/mod.rs - Fixed G-code processor
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::PrinterState;
use crate::motion::MotionController;

pub struct GCodeProcessor {
    state: Arc<RwLock<PrinterState>>,
    motion_controller: MotionController,
}

impl GCodeProcessor {
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        motion_controller: MotionController,
    ) -> Self {
        Self {
            state,
            motion_controller,
        }
    }

    pub async fn process_command(&mut self, command: &str) -> Result<(), Box<dyn std::error::Error>> {
        let command = command.trim();
        if command.is_empty() || command.starts_with(';') {
            return Ok(());
        }
        
        let parts: Vec<&str> = command.split_whitespace().collect();
        
        if parts.is_empty() {
            return Ok(());
        }
        
        match parts[0].to_uppercase().as_str() {
            "G0" | "G1" => self.handle_linear_move(&parts).await?,
            "G28" => self.handle_home(&parts).await?,
            "G92" => self.handle_set_position(&parts).await?,
            "M104" => self.handle_set_hotend_temp(&parts).await?,
            "M109" => self.handle_set_hotend_temp_wait(&parts).await?,
            "M140" => self.handle_set_bed_temp(&parts).await?,
            "M190" => self.handle_set_bed_temp_wait(&parts).await?,
            "M82" => println!("Extruder set to absolute mode"),
            "M84" => println!("Motors disabled"),
            "M106" => self.handle_fan_on(&parts).await?,
            "M107" => println!("Fan turned off"),
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
            if part.len() < 2 { continue; }
            
            let param = part.chars().next().unwrap_or(' ').to_ascii_uppercase();
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
        
        // Get current position for relative moves (simplified - assuming absolute)
        let current_pos = self.get_current_position().await;
        let target_x = x.unwrap_or(current_pos[0]);
        let target_y = y.unwrap_or(current_pos[1]);
        let target_z = z.unwrap_or(current_pos[2]);
        
        self.motion_controller
            .queue_linear_move([target_x, target_y, target_z], f, e)
            .await?;
        
        Ok(())
    }

    async fn handle_home(&mut self, _parts: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        self.motion_controller.queue_home().await?;
        Ok(())
    }

    async fn handle_set_position(&mut self, parts: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        let mut x = None;
        let mut y = None;
        let mut z = None;
        let mut e = None;
        
        for part in parts.iter().skip(1) {
            if part.len() < 2 { continue; }
            
            let param = part.chars().next().unwrap_or(' ').to_ascii_uppercase();
            let value: f64 = part[1..].parse().unwrap_or(0.0);
            
            match param {
                'X' => x = Some(value),
                'Y' => y = Some(value),
                'Z' => z = Some(value),
                'E' => e = Some(value),
                _ => {}
            }
        }
        
        println!("Setting position - X:{:?} Y:{:?} Z:{:?} E:{:?}", x, y, z, e);
        Ok(())
    }

    async fn handle_set_hotend_temp(&mut self, parts: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        for part in parts.iter().skip(1) {
            if part.starts_with('S') {
                let temp: f64 = part[1..].parse().unwrap_or(0.0);
                println!("Setting hotend temperature to {:.1}°C", temp);
                break;
            }
        }
        Ok(())
    }

    async fn handle_set_hotend_temp_wait(&mut self, parts: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        self.handle_set_hotend_temp(parts).await?;
        println!("Waiting for hotend temperature...");
        Ok(())
    }

    async fn handle_set_bed_temp(&mut self, parts: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        for part in parts.iter().skip(1) {
            if part.starts_with('S') {
                let temp: f64 = part[1..].parse().unwrap_or(0.0);
                println!("Setting bed temperature to {:.1}°C", temp);
                break;
            }
        }
        Ok(())
    }

    async fn handle_set_bed_temp_wait(&mut self, parts: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        self.handle_set_bed_temp(parts).await?;
        println!("Waiting for bed temperature...");
        Ok(())
    }

    async fn handle_fan_on(&mut self, parts: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        let mut speed = 255; // Full speed default
        for part in parts.iter().skip(1) {
            if part.starts_with('S') {
                speed = part[1..].parse().unwrap_or(255);
                break;
            }
        }
        println!("Setting fan speed to {}", speed);
        Ok(())
    }

    async fn get_current_position(&self) -> [f64; 3] {
        let pos = self.motion_controller.get_current_position();
        [pos[0], pos[1], pos[2]]
    }
}

impl Clone for GCodeProcessor {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            motion_controller: self.motion_controller.clone(),
        }
    }
}