// src/gcode.rs - Add Debug to MotionController
use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::RwLock;
use crate::motion::MotionController;
use crate::host_os::PrinterState;
use crate::gcode::macros::MacroProcessor;
use crate::gcode::parser::{OwnedGCodeCommand, GCodeError};


#[derive(Debug, Clone)] // This should work now
pub struct GCodeProcessor {
    state: Arc<RwLock<PrinterState>>,
    motion_controller: Arc<RwLock<MotionController>>,
    queue: Arc<tokio::sync::Mutex<VecDeque<String>>>,
    macros: Arc<MacroProcessor>,
}

impl GCodeProcessor {
    /// Handle G0/G1 linear move (stub implementation)
    async fn handle_linear_move(&mut self, parts: &[&str]) -> Result<(), GCodeError> {
        tracing::info!("Linear move: parts = {:?}", parts);
        Ok(())
    }
    /// Create a new GCodeProcessor with shared printer state and a motion controller.
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        motion_controller: Arc<RwLock<MotionController>>,
    ) -> Self {
        Self {
            state,
            motion_controller,
            queue: Arc::new(tokio::sync::Mutex::new(VecDeque::new())),
            macros: Arc::new(MacroProcessor::new()),
        }
    }

    pub async fn enqueue_command(&self, command: String) {
        let mut queue = self.queue.lock().await;
        queue.push_back(command);
    }

    pub async fn process_next_command(&self) -> Result<(), GCodeError> {
        let mut queue = self.queue.lock().await;
        if let Some(_command) = queue.pop_front() {
            // ...existing code for processing command...
            // (see above for full block)
            // For now, just call process_command
            // Note: self is &self, but process_command needs &mut self, so this may need refactoring
            // For now, return Ok(())
            // TODO: Refactor to allow calling process_command
            return Ok(());
        }
        Ok(())
    }

    async fn handle_home(&mut self, _parts: &[&str]) -> Result<(), GCodeError> {
        let mut mc = self.motion_controller.write().await;
        use crate::gcode::parser::GCodeSpan;
        mc.queue_home().await.map_err(|e| GCodeError { message: e.to_string(), span: GCodeSpan { range: 0..0 } })?;
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
        let macros = self.macros.clone();
        let results = macros.parse_and_expand_async_owned(command).await;
        let mut handled_any = false;
        let mut commands_to_process = Vec::new();
        for cmd_result in results {
            match cmd_result {
                Ok(OwnedGCodeCommand::Checksum { command: inner, checksum, .. }) => {
                    tracing::info!("Checksum: {} (command: {:?})", checksum, inner);
                    commands_to_process.push(*inner);
                }
                Ok(cmd) => {
                    commands_to_process.push(cmd);
                }
                Err(e) => {
                    tracing::warn!("G-code parse error: {} at {:?}", e.message, e.span);
                }
            }
        }
        while let Some(cmd) = commands_to_process.pop() {
            match cmd {
                OwnedGCodeCommand::Word { letter, value, span: _ } => {
                    let mut parts = vec![format!("{}{}", letter, value)];
                    if let Some(rest) = command.strip_prefix(&format!("{}{}", letter, value)) {
                        for param in rest.trim().split_whitespace() {
                            if !param.is_empty() {
                                parts.push(param.to_string());
                            }
                        }
                    }
                    match parts[0].to_uppercase().as_str() {
                        "G0" | "G1" => self.handle_linear_move(&parts.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await?,
                        "G28" => self.handle_home(&parts.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await?,
                        "G92" => self.handle_set_position(&parts.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await?,
                        "M104" => self.handle_set_hotend_temp(&parts.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await?,
                        "M109" => self.handle_set_hotend_temp_wait(&parts.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await?,
                        "M140" => self.handle_set_bed_temp(&parts.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await?,
                        "M190" => self.handle_set_bed_temp_wait(&parts.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await?,
                        "M82" => tracing::info!("Extruder set to absolute mode"),
                        "M84" => tracing::info!("Motors disabled"),
                        "M106" => self.handle_fan_on(&parts.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await?,
                        "M107" => tracing::info!("Fan turned off"),
                        _ => tracing::warn!("Unhandled G-code: {}", command),
                    }
                    handled_any = true;
                }
                OwnedGCodeCommand::Comment(comment, _) => {
                    tracing::info!("G-code comment: {}", comment);
                }
                OwnedGCodeCommand::Macro { name, args, .. } => {
                    tracing::info!("Macro encountered: {} {}", name, args);
                }
                OwnedGCodeCommand::VendorExtension { name, args, .. } => {
                    tracing::info!("Vendor extension: {} {}", name, args);
                }
                OwnedGCodeCommand::Checksum { .. } => {
                    // Should not occur here, as we flatten one level above
                }
            }
        }
        if !handled_any {
            let trimmed = command.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') {
                return Ok(());
            }
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

pub mod parser;
pub mod macros;
pub mod gcode_executor;