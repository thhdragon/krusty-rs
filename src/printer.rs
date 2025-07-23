// src/printer.rs (updated with proper hardware integration)
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::config::Config;
use crate::gcode::GCodeProcessor;
use crate::motion::MotionController;
use crate::hardware::HardwareManager;

/// Main printer orchestrator
/// 
/// This struct coordinates all printer subsystems:
/// - Hardware communication
/// - Motion control
/// - G-code processing
/// - State management
pub struct Printer {
    /// Printer configuration
    config: Config,
    
    /// Shared printer state (position, temperature, etc.)
    state: Arc<RwLock<PrinterState>>,
    
    /// G-code command processor
    gcode_processor: GCodeProcessor,
    
    /// Motion planning and control
    motion_controller: MotionController,
    
    /// Hardware communication manager
    hardware_manager: HardwareManager,
    
    /// Shutdown signal broadcaster
    shutdown_tx: broadcast::Sender<()>,
}

/// Shared printer state accessible by all subsystems
#[derive(Debug, Clone)]
pub struct PrinterState {
    /// Whether printer is fully initialized and ready
    pub ready: bool,
    
    /// Current toolhead position [X, Y, Z] in mm
    pub position: [f64; 3],
    
    /// Current hotend temperature in Celsius
    pub temperature: f64,
    
    /// Current print progress (0.0 to 1.0)
    pub print_progress: f64,
}

impl Printer {
    /// Create a new printer instance
    /// 
    /// # Arguments
    /// * `config` - Loaded printer configuration
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize shared state with default values
        let state = Arc::new(RwLock::new(PrinterState {
            ready: false,
            position: [0.0, 0.0, 0.0],
            temperature: 0.0,
            print_progress: 0.0,
        }));
        
        // Create shutdown signal channel
        let (shutdown_tx, _) = broadcast::channel(1);
        
        // Initialize subsystems
        let hardware_manager = HardwareManager::new(&config).await?;
        let motion_controller = MotionController::new(state.clone());
        let gcode_processor = GCodeProcessor::new(
            state.clone(),
            motion_controller.clone(),
            hardware_manager.clone(),
        );
        
        Ok(Self {
            config,
            state,
            gcode_processor,
            motion_controller,
            hardware_manager,
            shutdown_tx,
        })
    }
    
    /// Start all printer subsystems
    /// 
    /// This method:
    /// 1. Initializes hardware
    /// 2. Starts background processing tasks
    /// 3. Marks printer as ready
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Starting printer OS");
        
        // Initialize hardware (connect to MCU, configure components)
        self.hardware_manager.initialize().await?;
        
        // Start background processing tasks
        self.start_hardware_processing_loop().await?;
        self.start_gcode_processing_loop().await?;
        self.start_motion_control_loop().await?;
        
        // Mark printer as ready for operations
        {
            let mut state = self.state.write().await;
            state.ready = true;
        }
        
        tracing::info!("Printer OS ready and operational");
        Ok(())
    }
    
    /// Start hardware response processing loop
    /// 
    /// This background task continuously processes MCU responses
    /// to prevent blocking and ensure timely handling of messages
    async fn start_hardware_processing_loop(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Subscribe to shutdown signal
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
        // Clone hardware manager for the background task
        let hardware_manager = self.hardware_manager.clone();
        
        // Spawn the processing task
        tokio::spawn(async move {
            // Create timer for regular processing intervals
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(10));
            
            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Hardware processing loop shutting down");
                        break;
                    }
                    
                    // Process hardware responses at regular intervals
                    _ = interval.tick() => {
                        if let Err(e) = hardware_manager.process_responses().await {
                            tracing::error!("Hardware processing error: {}", e);
                            
                            // Depending on error severity, might want to shut down
                            // For now, we continue processing
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Start G-code processing loop
    /// 
    /// This background task processes queued G-code commands
    /// as they arrive from various sources (serial, file, network)
    async fn start_gcode_processing_loop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let gcode_processor = self.gcode_processor.clone();
        
        tokio::spawn(async move {
            // Very fast processing loop since G-code parsing is lightweight
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1));
            
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
    
    /// Start motion control loop
    /// 
    /// This high-priority background task handles:
    /// - Motion planning and trajectory generation
    /// - Real-time step generation for motors
    /// - Position tracking and kinematics
    async fn start_motion_control_loop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let motion_controller = self.motion_controller.clone();
        
        tokio::spawn(async move {
            // Very high frequency loop for precise timing
            // 100μs intervals = 10kHz update rate
            let mut interval = tokio::time::interval(tokio::time::Duration::from_micros(100));
            
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Motion control loop shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        if let Err(e) = motion_controller.update().await {
                            tracing::error!("Motion control error: {}", e);
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Graceful shutdown of all printer systems
    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Initiating printer OS shutdown");
        
        // Broadcast shutdown signal to all background tasks
        let _ = self.shutdown_tx.send(());
        
        // Shutdown hardware systems
        self.hardware_manager.shutdown().await?;
        
        tracing::info!("Printer OS shutdown complete");
        Ok(())
    }
}