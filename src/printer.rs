// src/printer.rs - Use all fields properly
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use thiserror::Error;

use crate::config::Config;
use crate::gcode::GCodeProcessor;
use crate::motion::MotionController;
use crate::hardware::HardwareManager;

#[derive(Debug, Error)]
pub enum PrinterError {
    #[error("Hardware error: {0}")]
    Hardware(#[from] crate::hardware::HardwareError),
    #[error("Motion error: {0}")]
    Motion(#[from] crate::motion::MotionError),
    #[error("GCode error: {0}")]
    GCode(#[from] crate::gcode::GCodeError),
    #[error("Other: {0}")]
    Other(String),
}

pub struct Printer {
    config: Config,
    state: Arc<RwLock<PrinterState>>,
    gcode_processor: GCodeProcessor,
    motion_controller: Arc<RwLock<MotionController>>,
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
    pub async fn new(config: Config) -> Result<Self, PrinterError> {
        let state = Arc::new(RwLock::new(PrinterState::new()));
        let (shutdown_tx, _) = broadcast::channel(1);
        let hardware_manager = HardwareManager::new(config.clone());
        let motion_controller = Arc::new(RwLock::new(MotionController::new(state.clone(), hardware_manager.clone(), &config)));
        let gcode_processor = GCodeProcessor::new(state.clone(), motion_controller.clone());

        if config.printer.printer_name.as_deref().unwrap_or("").is_empty() {
            return Err(PrinterError::Other("Printer name cannot be empty".to_string()));
        }

        Ok(Self {
            config,
            state,
            gcode_processor,
            motion_controller,
            hardware_manager,
            shutdown_tx,
        })
    }
    pub async fn start(&mut self) -> Result<(), PrinterError> {
        tracing::info!("Starting printer OS");
        self.hardware_manager.initialize().await?;
        self.start_gcode_processing_loop().await?;
        self.start_motion_control_loop().await?;
        {
            let mut state = self.state.write().await;
            state.ready = true;
        }
        tracing::info!("Printer OS ready");
        Ok(())
    }
    async fn start_motion_control_loop(&self) -> Result<(), PrinterError> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let motion_controller = self.motion_controller.clone();
        tokio::task::spawn_local(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_micros(1000));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Motion control loop shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        let mut mc = motion_controller.write().await;
                        if let Err(e) = mc.update().await {
                            tracing::error!("Motion controller update error: {}", e);
                        }
                    }
                }
            }
        });
        Ok(())
    }
    async fn start_gcode_processing_loop(&self) -> Result<(), PrinterError> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let gcode_processor = self.gcode_processor.clone();
        tokio::task::spawn_local(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(10));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::info!("G-code processing loop shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        if let Err(e) = gcode_processor.process_next_command().await {
                            tracing::error!("G-code processing error: {}", e);
                        }
                    }
                }
            }
        });
        Ok(())
    }
    pub async fn shutdown(&mut self) -> Result<(), PrinterError> {
        tracing::info!("Shutting down printer OS");
        let _ = self.shutdown_tx.send(());
        self.hardware_manager.shutdown().await?;
        Ok(())
    }
    pub async fn process_gcode(&mut self, gcode: &str) -> Result<(), PrinterError> {
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
    pub fn get_motion_controller(&self) -> Arc<RwLock<MotionController>> {
        self.motion_controller.clone()
    }
}