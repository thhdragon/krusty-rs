// src/gcode.rs - Add Debug to MotionController
use std::collections::VecDeque;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::motion::MotionController;
use crate::printer::PrinterState;

#[derive(Debug, Error)]
pub enum GCodeError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Motion error: {0}")]
    MotionError(String),
    #[error("State error: {0}")]
    StateError(String),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Clone)] // This should work now
pub struct GCodeProcessor {
    state: Arc<RwLock<PrinterState>>,
    motion_controller: Arc<RwLock<MotionController>>,
    queue: Arc<tokio::sync::Mutex<VecDeque<String>>>,
}

impl GCodeProcessor {
    /// Create a new GCodeProcessor with shared printer state and a motion controller.
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        motion_controller: Arc<RwLock<MotionController>>,
    ) -> Self {
        Self {
            state,
            motion_controller,
            queue: Arc::new(tokio::sync::Mutex::new(VecDeque::new())),
        }
    }

    pub async fn enqueue_command(&self, command: String) {
        let mut queue = self.queue.lock().await;
        queue.push_back(command);
    }

    pub async fn process_next_command(&self) -> Result<(), GCodeError> {
        let mut queue = self.queue.lock().await;
        if let Some(command) = queue.pop_front() {
            // Call process_command on self, not on motion_controller
            drop(queue); // Release lock before calling async fn
            let mut this = self.clone();
            this.process_command(&command).await?;
        }
        Ok(())
    }

    async fn handle_linear_move(&mut self, parts: &[&str]) -> Result<(), GCodeError> {
        let mut x = None;
        let mut y = None;
        let mut z = None;
        let mut e = None;
        let mut f = None;

        for part in parts.iter().skip(1) {
            if part.len() < 2 {
                tracing::warn!(
                    "Parameter '{}' is missing a value and will be ignored",
                    part
                );
                continue;
            }
            let param = part.chars().next().unwrap_or(' ').to_ascii_uppercase();
            let value_str = &part[1..];
            if value_str.is_empty() {
                tracing::warn!(
                    "Parameter '{}' is missing a value and will be ignored",
                    part
                );
                continue;
            }
            let value_res = value_str.parse::<f64>();
            match value_res {
                Ok(value) => match param {
                    'X' => x = Some(value),
                    'Y' => y = Some(value),
                    'Z' => z = Some(value),
                    'E' => e = Some(value),
                    'F' => f = Some(value),
                    _ => {}
                },
                Err(e) => tracing::warn!("Failed to parse parameter '{}': {}", part, e),
            }
        }

        // Get current position for relative moves (simplified - assuming absolute)
        let current_pos = self.get_current_position().await;
        let target_x = x.unwrap_or(current_pos[0]);
        let target_y = y.unwrap_or(current_pos[1]);
        let target_z = z.unwrap_or(current_pos[2]);

        let mut mc = self.motion_controller.write().await;
        mc.queue_linear_move([target_x, target_y, target_z], f, e)
            .await
            .map_err(|e| GCodeError::MotionError(e.to_string()))?;

        Ok(())
    }

    async fn handle_home(&mut self, _parts: &[&str]) -> Result<(), GCodeError> {
        let mut mc = self.motion_controller.write().await;
        mc.queue_home().await.map_err(|e| GCodeError::MotionError(e.to_string()))?;
        Ok(())
    }

    async fn handle_set_position(&mut self, parts: &[&str]) -> Result<(), GCodeError> {
        let mut x = None;
        let mut y = None;
        let mut z = None;
        let mut e = None;

        for part in parts.iter().skip(1) {
            if part.len() < 2 {
                tracing::warn!(
                    "Parameter '{}' is missing a value and will be ignored",
                    part
                );
                continue;
            }
            let param = part.chars().next().unwrap_or(' ').to_ascii_uppercase();
            let value_str = &part[1..];
            if value_str.is_empty() {
                tracing::warn!(
                    "Parameter '{}' is missing a value and will be ignored",
                    part
                );
                continue;
            }
            let value_res = value_str.parse::<f64>();
            match value_res {
                Ok(value) => match param {
                    'X' => x = Some(value),
                    'Y' => y = Some(value),
                    'Z' => z = Some(value),
                    'E' => e = Some(value),
                    _ => {}
                },
                Err(e) => tracing::warn!("Failed to parse parameter '{}': {}", part, e),
            }
        }
        tracing::info!(
            "Setting position - X:{:?} Y:{:?} Z:{:?} E:{:?}",
            x,
            y,
            z,
            e
        );
        Ok(())
    }

    async fn handle_set_hotend_temp(&mut self, parts: &[&str]) -> Result<(), GCodeError> {
        for part in parts.iter().skip(1) {
            if part.len() < 2 || !part[0..1].eq_ignore_ascii_case("S") {
                continue;
            }
            let value_str = &part[1..];
            if value_str.is_empty() {
                tracing::warn!(
                    "Parameter '{}' is missing a value and will be ignored",
                    part
                );
                continue;
            }
            match value_str.parse::<f64>() {
                Ok(temp) => {
                    tracing::info!("Setting hotend temperature to {:.1}°C", temp);
                    // Update state
                    {
                        let mut state = self.state.write().await;
                        state.temperature = temp;
                    }
                }
                Err(e) => tracing::warn!("Failed to parse hotend temp '{}': {}", part, e),
            }
            break;
        }
        Ok(())
    }

    async fn handle_set_bed_temp(&mut self, parts: &[&str]) -> Result<(), GCodeError> {
        for part in parts.iter().skip(1) {
            if part.len() < 2 || !part[0..1].eq_ignore_ascii_case("S") {
                continue;
            }
            let value_str = &part[1..];
            if value_str.is_empty() {
                tracing::warn!(
                    "Parameter '{}' is missing a value and will be ignored",
                    part
                );
                continue;
            }
            match value_str.parse::<f64>() {
                Ok(temp) => {
                    tracing::info!("Setting bed temperature to {:.1}°C", temp);
                    // Update bed temperature in state
                    {
                        let mut state = self.state.write().await;
                        state.bed_temperature = temp;
                    }
                },
                Err(e) => tracing::warn!("Failed to parse bed temp '{}': {}", part, e),
            }
            break;
        }
        Ok(())
    }

    async fn handle_fan_on(&mut self, parts: &[&str]) -> Result<(), GCodeError> {
        let mut speed = 255; // Full speed default
        for part in parts.iter().skip(1) {
            if part.len() < 2 || !part[0..1].eq_ignore_ascii_case("S") {
                continue;
            }
            let value_str = &part[1..];
            if value_str.is_empty() {
                tracing::warn!(
                    "Parameter '{}' is missing a value and will be ignored",
                    part
                );
                continue;
            }
            match value_str.parse::<i32>() {
                Ok(val) => speed = val,
                Err(e) => tracing::warn!("Failed to parse fan speed '{}': {}", part, e),
            }
            break;
        }
        tracing::info!("Setting fan speed to {}", speed);
        Ok(())
    }

    async fn handle_set_hotend_temp_wait(&mut self, parts: &[&str]) -> Result<(), GCodeError> {
        self.handle_set_hotend_temp(parts).await?;
        tracing::info!("Waiting for hotend temperature...");
        Ok(())
    }

    async fn handle_set_bed_temp_wait(&mut self, parts: &[&str]) -> Result<(), GCodeError> {
        self.handle_set_bed_temp(parts).await?;
        tracing::info!("Waiting for bed temperature...");
        Ok(())
    }

    /// Process a single G-code command string.
    pub async fn process_command(&mut self, command: &str) -> Result<(), GCodeError> {
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
            "M82" => tracing::info!("Extruder set to absolute mode"),
            "M84" => tracing::info!("Motors disabled"),
            "M106" => self.handle_fan_on(&parts).await?,
            "M107" => tracing::info!("Fan turned off"),
            _ => tracing::warn!("Unhandled G-code: {}", command),
        }
        Ok(())
    }

    async fn get_current_position(&self) -> [f64; 3] {
        let mc = self.motion_controller.read().await;
        let pos4 = mc.get_current_position();
        [pos4[0], pos4[1], pos4[2]]
    }

    /// Get a copy of the current printer state.
    pub async fn get_state(&self) -> PrinterState {
        self.state.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::MotionController;
    use crate::printer::PrinterState;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn dummy_motion_controller() -> Arc<RwLock<MotionController>> {
        Arc::new(RwLock::new(MotionController::default()))
    }

    #[tokio::test]
    async fn test_handle_linear_move_valid() {
        let state = Arc::new(RwLock::new(PrinterState::new()));
        let mut gcode = GCodeProcessor::new(state, dummy_motion_controller());
        let result = gcode.process_command("G1 X10 Y20 Z30 F1500").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_linear_move_invalid_param() {
        let state = Arc::new(RwLock::new(PrinterState::new()));
        let mut gcode = GCodeProcessor::new(state, dummy_motion_controller());
        let result = gcode.process_command("G1 Xbad Y20").await;
        assert!(result.is_ok()); // Should not panic or error fatally
    }

    #[tokio::test]
    async fn test_handle_set_hotend_temp() {
        let state = Arc::new(RwLock::new(PrinterState::new()));
        let mut gcode = GCodeProcessor::new(state.clone(), dummy_motion_controller());
        let result = gcode.process_command("M104 S200").await;
        assert!(result.is_ok());
        let s = state.read().await;
        assert_eq!(s.temperature, 200.0);
    }

    #[tokio::test]
    async fn test_handle_set_bed_temp() {
        let state = Arc::new(RwLock::new(PrinterState::new()));
        let mut gcode = GCodeProcessor::new(state.clone(), dummy_motion_controller());
        let result = gcode.process_command("M140 S60").await;
        assert!(result.is_ok());
        let s = state.read().await;
        assert_eq!(s.bed_temperature, 60.0);
    }
}