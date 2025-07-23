// src/motion/mod.rs - Fixed motion controller
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::PrinterState;
use crate::hardware::HardwareManager;

pub struct MotionController {
    state: Arc<RwLock<PrinterState>>,
    hardware_manager: HardwareManager,
    current_position: [f64; 4], // X, Y, Z, E
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
        
        tracing::info!("Queuing linear move to [{:.3}, {:.3}, {:.3}, {:.3}] at {:.1}mm/s",
                      target_4d[0], target_4d[1], target_4d[2], target_4d[3], feedrate);
        
        // Update current position (in real implementation, this would be queued)
        self.current_position = target_4d;
        
        // Update printer state
        {
            let mut state = self.state.write().await;
            state.position = [target_4d[0], target_4d[1], target_4d[2]];
        }
        
        Ok(())
    }

    pub async fn queue_home(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Queuing home command");
        self.current_position = [0.0, 0.0, 0.0, self.current_position[3]];
        
        // Update printer state
        {
            let mut state = self.state.write().await;
            state.position = [0.0, 0.0, 0.0];
        }
        
        Ok(())
    }

    pub async fn queue_extruder_move(
        &mut self,
        amount: f64,
        feedrate: Option<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let target_e = self.current_position[3] + amount;
        let feedrate = feedrate.unwrap_or(20.0);
        
        tracing::info!("Queuing extruder move: {:.3}mm at {:.1}mm/s", amount, feedrate);
        self.current_position[3] = target_e;
        
        Ok(())
    }

    pub async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Process motion updates (in real implementation, this would process queued moves)
        // For now, this is a placeholder
        Ok(())
    }

    pub fn emergency_stop(&mut self) {
        tracing::warn!("Emergency stop activated - clearing motion state");
        // In real implementation, this would clear the motion queue
    }

    pub fn get_current_position(&self) -> [f64; 4] {
        self.current_position
    }
}

impl Clone for MotionController {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            hardware_manager: self.hardware_manager.clone(),
            current_position: self.current_position,
        }
    }
}