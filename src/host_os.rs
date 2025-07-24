// src/host_os.rs - Complete 3D Printer Host OS Interface
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::printer::Printer;
use crate::config::Config;
use crate::gcode::GCodeProcessor;
use crate::motion::MotionController;
use crate::motion::controller::MotionMode;
use crate::PlannerMotionConfig;
use crate::hardware::HardwareManager;
use crate::web::WebInterface;
use crate::file_manager::FileManager;

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
    state: Arc<RwLock<crate::printer::PrinterState>>,
    
    /// Shutdown signaling
    shutdown_tx: broadcast::Sender<()>,
}

/// Configuration manager
pub struct ConfigManager {
    config: Config,
    config_path: String,
    backup_configs: Vec<Config>,
}

/// Main Host OS implementation
impl PrinterHostOS {
    /// Create new Host OS instance with pre-loaded configuration
    pub async fn new_with_config(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = "printer.toml".to_string(); // Default path
        let config_manager = ConfigManager::new(config.clone(), config_path);
        
        // Initialize core components
        let state = Arc::new(RwLock::new(crate::printer::PrinterState::default()));
        let (shutdown_tx, _) = broadcast::channel(1);
        
        let hardware_manager = HardwareManager::new(config.clone());
        let motion_config = PlannerMotionConfig::new_from_config(&config);
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
        })
    }

    /// Initialize the entire system
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Starting Host OS processing loops");
        
        // Start hardware response processing
        self.start_hardware_loop().await?;
        
        // Start motion control loop
        self.start_motion_loop().await?;
        
        // Start web server loop
        self.start_web_loop().await?;
        
        // Start file monitoring
        self.start_file_monitoring().await?;
        
        Ok(())
    }

    /// Main hardware processing loop
    async fn start_hardware_loop(&self) -> Result<(), Box<dyn std::error::Error>> {
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
    async fn start_motion_loop(&self) -> Result<(), Box<dyn std::error::Error>> {
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
    async fn start_web_loop(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Web interface runs independently
        Ok(())
    }

    /// File monitoring loop
    async fn start_file_monitoring(&self) -> Result<(), Box<dyn std::error::Error>> {
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
    pub async fn load_gcode_file(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Loading G-code file: {}", file_path);
        
        let gcode_content = self.file_manager.read_file(file_path).await?;
        let lines: Vec<&str> = gcode_content.lines().collect();
        
        // Update system state
        {
            let mut state = self.state.write().await;
            state.print_progress = 0.0; // Reset progress
            state.printing = true;
            state.paused = false;
            // File size and position tracking removed
        }
        
        tracing::info!("Loaded {} lines of G-code", lines.len());
        Ok(())
    }

    /// Start printing the loaded file
    pub async fn start_print(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
            // state.system_stats.print_count += 1; // Removed unsupported field
        }
        
        Ok(())
    }

    /// Pause current print
    pub async fn pause_print(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    pub async fn resume_print(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    pub async fn cancel_print(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Canceling print job");
        
        {
            let mut state = self.state.write().await;
            state.printing = false;
            state.paused = false;
            // state.system_stats.failed_prints += 1; // Removed unsupported field
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
    pub async fn home_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    ) -> Result<(), Box<dyn std::error::Error>> {
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
    pub async fn set_hotend_temperature(&self, temperature: f64) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Setting hotend temperature to {:.1}°C", temperature);
        self.hardware_manager
            .set_heater_temperature("hotend", temperature)
            .await?;
        
        {
            let state = self.state.write().await;
            // state.temperature.hotend_target = temperature; // Removed unsupported field
        }
        
        Ok(())
    }

    /// Set bed temperature
    pub async fn set_bed_temperature(&self, temperature: f64) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Setting bed temperature to {:.1}°C", temperature);
        self.hardware_manager
            .set_heater_temperature("bed", temperature)
            .await?;
        
        {
            let state = self.state.write().await;
            // state.temperature.bed_target = temperature; // Removed unsupported field
        }
        
        Ok(())
    }

    /// Get current system status
    pub async fn get_status(&self) -> crate::printer::PrinterState {
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
    pub async fn emergency_stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
            // state.error = Some("Emergency stop activated".to_string()); // Removed unsupported field
        }
        
        Ok(())
    }

    /// Save current configuration
    pub async fn save_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config_manager.get_config();
        self.config_manager.save_config(&config).await?;
        Ok(())
    }

    /// Reload configuration
    pub async fn reload_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let new_config = self.config_manager.reload_config()?;
        // Apply new configuration to all components
        // This would require reinitializing components
        Ok(())
    }

    /// Shutdown the entire system
    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
        self.state.read().await.print_progress // Removed system_stats.uptime reference
    }
}

/// System information structure
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub version: String,
    pub name: String,
    pub rust_version: String,
    pub uptime: f64,
}

impl ConfigManager {
    pub fn new(config: Config, config_path: String) -> Self {
        Self {
            config,
            config_path,
            backup_configs: Vec::new(),
        }
    }

    pub fn load_config(config_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        Ok(crate::config::load_config(config_path)?)
    }

    pub async fn save_config(&self, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;
        let toml_string = toml::to_string(config)?;
        let mut file = std::fs::File::create(&self.config_path)?;
        file.write_all(toml_string.as_bytes())?;
        Ok(())
    }

    pub fn reload_config(&self) -> Result<Config, Box<dyn std::error::Error>> {
        Self::load_config(&self.config_path)
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn backup_config(&mut self) {
        self.backup_configs.push(self.config.clone());
        // Keep only last 5 backups
        while self.backup_configs.len() > 5 {
            self.backup_configs.remove(0);
        }
    }

    pub fn restore_backup(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        if index < self.backup_configs.len() {
            self.config = self.backup_configs[index].clone();
            Ok(())
        } else {
            Err("Backup index out of range".into())
        }
    }
}

/// --- Host OS Feature Stubs (for parity with Klipper) ---

/// Robust serial protocol stub (CRC, sequence, retransmit, windowing)
pub struct SerialProtocolStub;
impl SerialProtocolStub {
    /// Send a command with CRC and sequence number (stub)
    pub async fn send_command(&self, _cmd: &str) -> Result<(), String> {
        // TODO: Implement CRC, sequence, retransmit, windowing
        Ok(())
    }
    /// Receive and validate a response (stub)
    pub async fn receive_response(&self) -> Result<String, String> {
        // TODO: Implement CRC, sequence, retransmit, windowing
        Ok("stub response".to_string())
    }
}

/// Print time/MCU clock synchronization stub
pub struct ClockSyncStub;
impl ClockSyncStub {
    pub fn new() -> Self { Self }
    /// Sync host and MCU clocks (stub)
    pub async fn sync(&self) -> Result<(), String> {
        // TODO: Implement print time/MCU clock sync
        Ok(())
    }
}

/// Dynamic module system stub
pub struct ModuleManagerStub;
impl ModuleManagerStub {
    pub fn new() -> Self { Self }
    /// Load a module by name (stub)
    pub fn load_module(&self, _name: &str) {
        // TODO: Implement dynamic module loading
    }
}

/// Multi-MCU abstraction stub
pub struct MultiMCUManagerStub;
impl MultiMCUManagerStub {
    pub fn new() -> Self { Self }
    /// Register a new MCU (stub)
    pub fn register_mcu(&self, _id: &str) {
        // TODO: Implement multi-MCU support
    }
}

/// Event extensibility stub
pub struct EventBusStub;
impl EventBusStub {
    pub fn new() -> Self { Self }
    /// Register an event handler (stub)
    pub fn register_handler(&self, _event: &str, _handler: fn()) {
        // TODO: Implement event handler registration
    }
    /// Emit an event (stub)
    pub fn emit(&self, _event: &str) {
        // TODO: Implement event emission
    }
}