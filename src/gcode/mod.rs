// src/gcode/mod.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::PrinterState;
use crate::motion::MotionController;

pub struct GCodeProcessor {
    state: Arc<RwLock<PrinterState>>,
    motion_controller: MotionController,
    // hardware_manager: HardwareManager,
    command_queue: tokio::sync::mpsc::UnboundedReceiver<GCodeCommand>,
    command_sender: tokio::sync::mpsc::UnboundedSender<GCodeCommand>,
}

#[derive(Debug, Clone)]
pub struct GCodeCommand {
    pub command: String,
    pub parameters: std::collections::HashMap<String, String>,
}

impl GCodeProcessor {
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        motion_controller: MotionController,
        // hardware_manager: HardwareManager,
    ) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        
        Self {
            state,
            motion_controller,
            // hardware_manager,
            command_queue: receiver,
            command_sender: sender,
        }
    }
    
    pub fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            motion_controller: self.motion_controller.clone(),
            // hardware_manager: self.hardware_manager.clone(),
            command_queue: {
                let (_, receiver) = tokio::sync::mpsc::unbounded_channel();
                receiver
            },
            command_sender: self.command_sender.clone(),
        }
    }
    
    pub async fn process_next_command(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(command) = self.command_queue.recv().await {
            self.handle_command(command).await?;
        }
        Ok(())
    }
    
    async fn handle_command(&self, command: GCodeCommand) -> Result<(), Box<dyn std::error::Error>> {
        tracing::debug!("Processing G-code: {}", command.command);
        
        match command.command.as_str() {
            "G1" | "G0" => self.handle_linear_move(command).await?,
            "G28" => self.handle_home(command).await?,
            "M104" | "M109" => self.handle_set_temperature(command).await?,
            "M140" | "M190" => self.handle_set_bed_temperature(command).await?,
            _ => tracing::warn!("Unhandled G-code command: {}", command.command),
        }
        
        Ok(())
    }
    
    async fn handle_linear_move(&self, command: GCodeCommand) -> Result<(), Box<dyn std::error::Error>> {
        let mut target = [0.0f64; 3];
        let mut feedrate = None;
        
        for (param, value) in &command.parameters {
            match param.as_str() {
                "X" => target[0] = value.parse()?,
                "Y" => target[1] = value.parse()?,
                "Z" => target[2] = value.parse()?,
                "F" => feedrate = Some(value.parse()?),
                _ => {}
            }
        }
        
        self.motion_controller
            .queue_move(target, feedrate)
            .await?;
        
        Ok(())
    }
    
    async fn handle_home(&self, _command: GCodeCommand) -> Result<(), Box<dyn std::error::Error>> {
        self.motion_controller.queue_home().await?;
        Ok(())
    }
    
    async fn handle_set_temperature(&self, command: GCodeCommand) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(temp_str) = command.parameters.get("S") {
            let temp: f64 = temp_str.parse()?;
            tracing::info!("Setting extruder temperature to {}°C", temp);
            // TODO: Send to hardware manager
        }
        Ok(())
    }
    
    async fn handle_set_bed_temperature(&self, command: GCodeCommand) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(temp_str) = command.parameters.get("S") {
            let temp: f64 = temp_str.parse()?;
            tracing::info!("Setting bed temperature to {}°C", temp);
            // TODO: Send to hardware manager
        }
        Ok(())
    }
}