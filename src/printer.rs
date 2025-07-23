// src/printer.rs - Use all fields properly
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::config::Config;
use crate::gcode::GCodeProcessor;
use crate::motion::MotionController;
use crate::hardware::{HardwareManager, TemperatureController};

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
    pub bed_temperature: f64,
    pub print_progress: f64,
}

impl PrinterState {
    pub fn new() -> Self {
        Self {
            ready: false,
            position: [0.0, 0.0, 0.0],
            temperature: 0.0,
            bed_temperature: 0.0,
            print_progress: 0.0,
        }
    }
}

impl Default for PrinterState {
    fn default() -> Self {
        Self::new()
    }
}

impl Printer {
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let state = Arc::new(RwLock::new(PrinterState::new()));
        let (shutdown_tx, _) = broadcast::channel(1);
        
        let hardware_manager = HardwareManager::new(config.clone());
        let motion_controller = MotionController::new(state.clone(), hardware_manager.clone(), &config);
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
        tracing::info!("Starting printer OS");
        
        // Initialize hardware
        self.hardware_manager.initialize().await?;
        
        // Mark as ready
        {
            let mut state = self.state.write().await;
            state.ready = true;
        }
        
        tracing::info!("Printer OS ready");
        Ok(())
    }
    
    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Shutting down printer OS");
        let _ = self.shutdown_tx.send(());
        self.hardware_manager.shutdown().await?;
        Ok(())
    }
    
    pub async fn process_gcode(&mut self, gcode: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.gcode_processor.process_command(gcode).await?;
        Ok(())
    }
    
    // Add methods to use the fields
    pub fn get_config(&self) -> &Config {
        &self.config
    }
    
    pub async fn get_state(&self) -> PrinterState {
        self.state.read().await.clone()
    }
    
    pub fn get_motion_controller(&self) -> &MotionController {
        &self.motion_controller
    }
}