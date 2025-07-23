// src/printer.rs - Fixed printer system
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::config::Config;
use crate::gcode::GCodeProcessor;
use crate::motion::MotionController;
use crate::hardware::HardwareManager;

pub struct Printer {
    config: Config,
    state: Arc<RwLock<PrinterState>>,
    gcode_processor: GCodeProcessor,
    motion_controller: MotionController,
    hardware_manager: HardwareManager,
    shutdown_tx: broadcast::Sender<()>,
}

#[derive(Debug, Clone)]
pub struct PrinterState {
    pub ready: bool,
    pub position: [f64; 3], // X, Y, Z
    pub temperature: f64,
    pub print_progress: f64,
}

impl PrinterState {
    pub fn new() -> Self {
        Self {
            ready: false,
            position: [0.0, 0.0, 0.0],
            temperature: 0.0,
            print_progress: 0.0,
        }
    }
}

impl Printer {
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let state = Arc::new(RwLock::new(PrinterState::new()));
        let (shutdown_tx, _) = broadcast::channel(1);
        
        let hardware_manager = HardwareManager::new(config.clone());
        let motion_controller = MotionController::new(state.clone(), hardware_manager.clone());
        let gcode_processor = GCodeProcessor::new(state.clone(), motion_controller.clone());
        
        Ok(Self {
            config,
            state,
            gcode_processor,
            motion_controller,
            hardware_manager,
            shutdown_tx,
        })
    }
    
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting printer OS");
        
        // Initialize hardware
        self.hardware_manager.initialize().await?;
        
        // Mark as ready
        {
            let mut state = self.state.write().await;
            state.ready = true;
        }
        
        println!("Printer OS ready");
        Ok(())
    }
    
    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Shutting down printer OS");
        let _ = self.shutdown_tx.send(());
        self.hardware_manager.shutdown().await?;
        Ok(())
    }
    
    pub async fn process_gcode(&mut self, gcode: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.gcode_processor.process_command(gcode).await?;
        Ok(())
    }
}