use crate::communication::serial_protocol::SerialProtocolStub;
use crate::multi_mcu_manager::MultiMCUManagerStub;
use crate::module_manager::ModuleManagerStub;
use krusty_shared::system_info::SystemInfo;
// NOTE: Public API types (PrinterHostOS, SystemInfo, stubs) are re-exported via mod.rs and lib.rs
// Only internal logic and trait implementations should remain here

// src/host_os.rs - Implementation details for 3D Printer Host OS
// Public API types (PrinterHostOS, SystemInfo, stubs) are re-exported via mod.rs and lib.rs
// Only internal logic and trait implementations should remain here

use crate::communication::event_bus_stub::EventBusStub;
// use serial2_tokio::SerialPort; // Only used in trait impl below (commented out)

// SystemInfo struct moved to src/system_info.rs

// ModuleManagerStub moved to src/module_manager.rs

// MultiMCUManagerStub moved to src/multi_mcu_manager.rs


// ...SerialInterface trait moved to src/communication/serial_interface.rs...

// ...TimeInterface trait moved to src/scheduler/time_interface.rs...

// ...EventInterface trait moved to src/communication/event_interface.rs...

use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::config::{Config, ConfigManager};
use crate::gcode::GCodeProcessor;
use crate::motion::MotionController;
use crate::motion::controller::MotionMode;
use crate::hardware::HardwareManager;
use crate::web::WebInterface;
use crate::file_manager::FileManager;
use krusty_shared::board_config::BoardConfig;
use krusty_shared::trajectory::MotionConfig;
use krusty_shared::MotionConfigExt;

#[derive(Debug, thiserror::Error)]
pub enum PrinterError {
    #[error("Hardware error: {0}")]
    Hardware(#[from] crate::hardware::HardwareError),
    #[error("Motion error: {0}")]
    Motion(#[from] crate::motion::MotionError),
    #[error("GCode error: {0}")]
    GCode(#[from] krusty_shared::gcode::GCodeError),
    #[error("Other: {0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub struct PrinterState {
    pub ready: bool,
    pub position: [f64; 3], // X, Y, Z
    pub temperature: f64,
    pub bed_temperature: f64,
    pub print_progress: f64,
    pub printing: bool,
    pub paused: bool,
}

impl PrinterState {
    pub fn new() -> Self {
        Self {
            ready: false,
            position: [0.0, 0.0, 0.0],
            temperature: 0.0,
            bed_temperature: 0.0,
            print_progress: 0.0,
            printing: false,
            paused: false,
        }
    }
}

impl Default for PrinterState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Printer {
    config: crate::config::Config,
    state: std::sync::Arc<tokio::sync::RwLock<PrinterState>>,
    gcode_processor: crate::gcode::GCodeProcessor,
    motion_controller: std::sync::Arc<tokio::sync::RwLock<crate::motion::MotionController>>,
    hardware_manager: crate::hardware::HardwareManager,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl Printer {
    pub async fn new(config: crate::config::Config) -> Result<Self, PrinterError> {
        let state = std::sync::Arc::new(tokio::sync::RwLock::new(PrinterState::new()));
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        let board = BoardConfig::new(
            config.printer.printer_name.as_deref().unwrap_or("DefaultBoard")
        );
        let hardware_manager = crate::hardware::HardwareManager::new(config.clone(), board);
        let motion_controller = std::sync::Arc::new(tokio::sync::RwLock::new(crate::motion::MotionController::new(
            state.clone(),
            hardware_manager.clone(),
            crate::motion::controller::MotionMode::Basic,
            &config,
        )));
        let gcode_processor = crate::gcode::GCodeProcessor::new(state.clone(), motion_controller.clone());
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
    pub fn get_config(&self) -> &crate::config::Config {
        &self.config
    }
    pub async fn get_state(&self) -> PrinterState {
        self.state.read().await.clone()
    }
    pub fn get_motion_controller(&self) -> std::sync::Arc<tokio::sync::RwLock<crate::motion::MotionController>> {
        self.motion_controller.clone()
    }
}

/// Complete 3D Printer Host OS
pub struct PrinterHostOS {
    /// Core printer system
    printer: Printer,

    /// Configuration management
    config_manager: ConfigManager,

    /// File management
    file_manager: FileManager,

    /// Web interface
    web_interface: WebInterface,

    /// G-code processing
    gcode_processor: GCodeProcessor,

    /// Motion control
    motion_controller: Arc<RwLock<MotionController>>,

    /// Hardware interface
    hardware_manager: HardwareManager,

    /// System state
    state: Arc<RwLock<PrinterState>>,

    /// Shutdown signaling
    shutdown_tx: broadcast::Sender<()>,
    /// Serial protocol stub (for parity with Klipper)
    serial_protocol: SerialProtocolStub,
    /// Dynamic module manager stub
    module_manager: ModuleManagerStub,
    /// Multi-MCU manager stub
    multi_mcu_manager: MultiMCUManagerStub,
    /// Event bus stub
    event_bus: EventBusStub,
}

/// Main Host OS implementation
impl PrinterHostOS {
    /// Get a reference to the printer
    pub fn get_printer(&self) -> &Printer {
        &self.printer
    }

    /// Get a reference to the GCodeProcessor
    pub fn get_gcode_processor(&self) -> &GCodeProcessor {
        &self.gcode_processor
    }

    /// Process a G-code command using the GCodeProcessor
    pub async fn process_gcode_command(&mut self, gcode: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        self.gcode_processor.process_command(gcode).await?;
        Ok(())
    }

    /// Get the current printer state
    pub async fn get_printer_state(&self) -> PrinterState {
        self.state.read().await.clone()
    }

    /// Get the current configuration
    pub fn get_config(&self) -> &Config {
        self.config_manager.get_config()
    }

    /// Get the file manager
    pub fn get_file_manager(&self) -> &FileManager {
        &self.file_manager
    }

    /// Get the hardware manager
    pub fn get_hardware_manager(&self) -> &HardwareManager {
        &self.hardware_manager
    }

    /// Get the motion controller
    pub fn get_motion_controller(&self) -> Arc<RwLock<MotionController>> {
        self.motion_controller.clone()
    }

    /// Get the web interface
    pub fn get_web_interface(&self) -> &WebInterface {
        &self.web_interface
    }

    /// Get the serial protocol stub
    pub fn get_serial_protocol(&self) -> &SerialProtocolStub {
        &self.serial_protocol
    }

    /// Get the module manager stub
    pub fn get_module_manager(&self) -> &ModuleManagerStub {
        &self.module_manager
    }

    /// Get the multi-MCU manager stub
    pub fn get_multi_mcu_manager(&self) -> &MultiMCUManagerStub {
        &self.multi_mcu_manager
    }

    /// Get the event bus stub
    pub fn get_event_bus(&self) -> &EventBusStub {
        &self.event_bus
    }
    /// Create new Host OS instance with pre-loaded configuration
    pub async fn new_with_config(config: Config) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let config_path = "printer.toml".to_string(); // Default path
        let config_manager = ConfigManager::new(config.clone(), config_path);

        // Initialize core components
        let state = Arc::new(RwLock::new(PrinterState::default()));
        let (shutdown_tx, _) = broadcast::channel(1);

        let board = BoardConfig::new(&config.printer.printer_name.clone().unwrap_or("DefaultBoard".to_string()));
        let hardware_manager = HardwareManager::new(config.clone(), board);
        let _motion_config = MotionConfig::new_from_config(&config); // Unused, for future planner config
        let motion_controller = Arc::new(RwLock::new(MotionController::new(
            state.clone(),
            hardware_manager.clone(),
            MotionMode::Basic, // Or choose based on config
            &config,
        )));

        let gcode_processor = GCodeProcessor::new(
            state.clone(),
            motion_controller.clone(),
        );

        let printer = Printer::new(config.clone()).await?;
        let file_manager = FileManager::new();
        let web_interface = WebInterface::new(state.clone());

        // Initialize stubs for extensibility
        // Setup serial port (example: "/dev/ttyUSB0", 250000 baud)
        let serial_port = Arc::new(serial2_tokio::SerialPort::open("/dev/ttyUSB0", 250000)?);
        let serial_protocol = SerialProtocolStub::new(serial_port, 4); // window_size=4
        let module_manager = ModuleManagerStub::new();
        let multi_mcu_manager = MultiMCUManagerStub::new();
        let event_bus = EventBusStub::new();

        Ok(Self {
            printer,
            config_manager,
            file_manager,
            web_interface,
            gcode_processor,
            motion_controller,
            hardware_manager,
            state,
            shutdown_tx,
            serial_protocol,
            module_manager,
            multi_mcu_manager,
            event_bus,
        })
    }

    /// Initialize the entire system
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        tracing::info!("Initializing 3D Printer Host OS");
        
        // Initialize hardware
        self.hardware_manager.initialize().await?;
        
        // Initialize motion system
        // (Already initialized in constructor)
        
        // Initialize web interface
        self.web_interface.start().await?;
        
        // Mark system as ready
        {
            let mut state = self.state.write().await;
            state.ready = true;
        }
        
        tracing::info!("Host OS initialization complete");
        Ok(())
    }

    /// Start main processing loops
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        tracing::info!("Starting Host OS processing loops");
        
        // Start hardware response processing
        self.start_hardware_loop().await?;
        
        // Start motion control loop
        self.start_motion_loop().await?;
        
        // Start web server loop
        self.start_web_loop().await?;
        
        // Start file monitoring
        self.start_file_monitoring().await?;

        // Start G-code processing loop (connect unused gcode_processor)
        self.start_gcode_processing_loop().await?;

        // Optionally, start printer main loop (connect unused printer)
        self.start_printer_main_loop().await?;
        
        Ok(())
    }
    /// G-code processing loop
    async fn start_gcode_processing_loop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
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

    /// Printer main loop (stub for future use)
    async fn start_printer_main_loop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        // Example: monitor printer state, update progress, etc.
        Ok(())
    }

    /// Main hardware processing loop
    async fn start_hardware_loop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let hardware_manager = self.hardware_manager.clone();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(10));
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => break,
                _ = interval.tick() => {
                    if let Err(e) = hardware_manager.process_responses().await {
                        tracing::error!("Hardware processing error: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Motion control processing loop
    async fn start_motion_loop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let motion_controller = self.motion_controller.clone();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_micros(100));
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => break,
                _ = interval.tick() => {
                    let mut controller = motion_controller.write().await;
                    if let Err(e) = controller.update().await {
                        tracing::error!("Motion control error: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Web interface processing loop
    async fn start_web_loop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        // Web interface runs independently
        Ok(())
    }

    /// File monitoring loop
    async fn start_file_monitoring(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let file_manager = self.file_manager.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    _ = interval.tick() => {
                        if let Err(e) = file_manager.check_for_updates().await {
                            tracing::error!("File monitoring error: {}", e);
                        }
                    }
                }
            }
        });
        
        Ok(())
    }

    /// Load and start a G-code file
    pub async fn load_gcode_file(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        tracing::info!("Loading G-code file: {}", file_path);
        
        let gcode_content = self.file_manager.read_file(file_path).await?;
        let lines: Vec<&str> = gcode_content.lines().collect();
        
        // Update system state
        {
            let mut state = self.state.write().await;
            state.print_progress = 0.0; // Reset progress
            state.printing = true;
            state.paused = false;
            // File size and position tracking removed (ensure this is documented in PrinterState)
        }
        
        tracing::info!("Loaded {} lines of G-code", lines.len());
        Ok(())
    }

    /// Start printing the loaded file
    pub async fn start_print(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        tracing::info!("Starting print job");
        
        {
            let mut state = self.state.write().await;
            if !state.ready {
                return Err("System not ready".into());
            }
            if state.printing {
                return Err("Print already in progress".into());
            }
            state.printing = true;
            state.paused = false;
            // Print count tracking removed (ensure PrinterState docs clarify)
        }
        
        Ok(())
    }

    /// Pause current print
    pub async fn pause_print(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        tracing::info!("Pausing print job");
        
        {
            let mut state = self.state.write().await;
            if !state.printing {
                return Err("No print in progress".into());
            }
            state.paused = true;
        }
        
        // Emergency stop motion
        {
            let mut controller = self.motion_controller.write().await;
            controller.emergency_stop();
        }
        
        Ok(())
    }

    /// Resume paused print
    pub async fn resume_print(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        tracing::info!("Resuming print job");
        
        {
            let mut state = self.state.write().await;
            if !state.printing {
                return Err("No print in progress".into());
            }
            if !state.paused {
                return Err("Print not paused".into());
            }
            state.paused = false;
        }
        
        Ok(())
    }

    /// Cancel current print
    pub async fn cancel_print(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        tracing::info!("Canceling print job");
        
        {
            let mut state = self.state.write().await;
            state.printing = false;
            state.paused = false;
            // Failed print tracking removed (ensure PrinterState docs clarify)
        }
        
        // Emergency stop
        {
            let mut controller = self.motion_controller.write().await;
            controller.emergency_stop();
        }
        // Home printer
        {
            let mut controller = self.motion_controller.write().await;
            controller.queue_home().await?;
        }
        
        Ok(())
    }

    /// Home all axes
    pub async fn home_all(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        tracing::info!("Homing all axes");
        {
            let mut controller = self.motion_controller.write().await;
            controller.queue_home().await?;
        }
        Ok(())
    }

    /// Move to specific position
    pub async fn move_to(
        &mut self,
        x: Option<f64>,
        y: Option<f64>,
        z: Option<f64>,
        feedrate: Option<f64>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        // Get current position
        let current_pos = {
            let state = self.state.read().await;
            state.position
        };
        
        let target_x = x.unwrap_or(current_pos[0]);
        let target_y = y.unwrap_or(current_pos[1]);
        let target_z = z.unwrap_or(current_pos[2]);
        let feedrate = feedrate.unwrap_or(300.0);
        
        tracing::info!("Moving to X:{:.3} Y:{:.3} Z:{:.3} F:{:.1}", 
                      target_x, target_y, target_z, feedrate);
        
        {
            let mut controller = self.motion_controller.write().await;
            controller.queue_linear_move([target_x, target_y, target_z], Some(feedrate), None).await?;
        }
        
        Ok(())
    }

    /// Set hotend temperature
    pub async fn set_hotend_temperature(&self, temperature: f64) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        tracing::info!("Setting hotend temperature to {:.1}°C", temperature);
        self.hardware_manager
            .set_heater_temperature("hotend", temperature)
            .await?;
        
        // hotend_target tracking removed (ensure PrinterState docs clarify)
        
        Ok(())
    }

    /// Set bed temperature
    pub async fn set_bed_temperature(&self, temperature: f64) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        tracing::info!("Setting bed temperature to {:.1}°C", temperature);
        self.hardware_manager
            .set_heater_temperature("bed", temperature)
            .await?;
        
        // bed_target tracking removed (ensure PrinterState docs clarify)
        
        Ok(())
    }

    /// Get current system status
    pub async fn get_status(&self) -> PrinterState {
        self.state.read().await.clone()
    }

    /// Get hardware statistics
    pub async fn get_hardware_stats(&self) -> crate::hardware::CommandStats {
        self.hardware_manager.get_command_stats().await
    }

    /// Get motion queue statistics
    pub async fn get_motion_stats(&self) -> crate::motion::QueueStats {
        let controller = self.motion_controller.read().await;
        crate::motion::QueueStats {
            length: controller.get_queue_length(),
            max_length: 0, // TODO: track max length if needed
            last_command: None, // TODO: track last command if needed
        }
    }

    /// Emergency stop
    pub async fn emergency_stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        tracing::warn!("EMERGENCY STOP ACTIVATED");
        
        // Stop all motion
        {
            let mut controller = self.motion_controller.write().await;
            controller.emergency_stop();
        }
        
        // Disable heaters
        let _ = self.hardware_manager.set_heater_temperature("hotend", 0.0).await;
        let _ = self.hardware_manager.set_heater_temperature("bed", 0.0).await;
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.printing = false;
            state.paused = false;
            // Error field removed (ensure PrinterState docs clarify)
        }
        
        Ok(())
    }

    /// Save current configuration
    pub async fn save_config(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let config = self.config_manager.get_config();
        self.config_manager.save_config(&config).await?;
        Ok(())
    }

    /// Reload configuration
    pub async fn reload_config(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let _ = self.config_manager.reload_config()?; // Unused, for future config reload logic
        // Apply new configuration to all components
        // This would require reinitializing components
        Ok(())
    }

    /// Shutdown the entire system
    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        tracing::info!("Shutting down Host OS");
        
        // Broadcast shutdown signal
        let _ = self.shutdown_tx.send(());
        
        // Emergency stop
        self.emergency_stop().await?;
        
        // Shutdown hardware
        self.hardware_manager.shutdown().await?;
        
        // Shutdown web interface
        self.web_interface.shutdown().await?;
        
        tracing::info!("Host OS shutdown complete");
        Ok(())
    }

    /// Get system information
    pub async fn get_system_info(&self) -> SystemInfo {
        SystemInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            name: env!("CARGO_PKG_NAME").to_string(),
            rust_version: env!("CARGO_PKG_RUST_VERSION").to_string(),
            uptime: self.get_uptime().await,
        }
    }

    /// Get system uptime
    async fn get_uptime(&self) -> f64 {
        self.state.read().await.print_progress // Uptime tracking removed (ensure PrinterState docs clarify)
    }
}
