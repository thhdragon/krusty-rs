// src/main.rs - Fixed error handling
mod printer;
mod gcode;
mod motion;
mod hardware;
mod config;
mod file_manager;

use printer::Printer;
use tokio::signal;
use std::env;

// Create a unified error type or convert properly
#[derive(Debug)]
pub enum AppError {
    Config(config::ConfigError),
    Io(std::io::Error),
    Other(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Config(e) => write!(f, "Config error: {}", e),
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

impl std::error::Error for AppError {}

impl From<config::ConfigError> for AppError {
    fn from(error: config::ConfigError) -> Self {
        AppError::Config(error)
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::Io(error)
    }
}

impl From<Box<dyn std::error::Error>> for AppError {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        AppError::Other(error.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    tracing::info!("Starting Krusty-RS 3D Printer OS");
    tracing::info!("Version: {}", env!("CARGO_PKG_VERSION"));
    
    // Get configuration file path
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "printer.toml"
    };
    
    tracing::info!("Loading configuration from: {}", config_path);
    
    // Load configuration
    let config = match config::load_config(config_path) {
        Ok(cfg) => {
            tracing::info!("Configuration loaded successfully");
            cfg
        },
        Err(e) => {
            tracing::error!("Failed to load config from '{}': {}", config_path, e);
            tracing::error!("Please ensure the configuration file exists and is properly formatted");
            return Err(AppError::Config(e));
        }
    };
    
    // Display printer information
    tracing::info!("Printer: {} ({})", 
                  config.printer.printer_name.as_ref().unwrap_or(&"Unknown".to_string()), 
                  config.printer.kinematics);
    tracing::info!("MCU: {} @ {} baud", config.mcu.serial, config.mcu.baud);
    tracing::info!("Max velocity: {} mm/s", config.printer.max_velocity);
    tracing::info!("Max acceleration: {} mm/s²", config.printer.max_accel);
    
    // Create and start printer
    let mut printer = match Printer::new(config).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to initialize printer: {}", e);
            return Err(AppError::Other(e.to_string()));
        }
    };
    
    match printer.start().await {
        Ok(()) => tracing::info!("Printer OS started successfully"),
        Err(e) => {
            tracing::error!("Failed to start printer: {}", e);
            return Err(AppError::Other(e.to_string()));
        }
    }
    
    // Test some advanced G-code commands
    tracing::info!("Testing advanced G-code commands...");
    let test_commands = vec![
        "G28 ; Home all axes",
        "G1 X100 Y100 Z10 F3000 ; Move to position",
        "G1 E10 F200 ; Extrude 10mm",
        "M104 S200 ; Set hotend temperature",
        "M140 S60 ; Set bed temperature",
        "M106 S255 ; Fan on full speed",
        "M117 Testing Krusty-RS ; Display message",
    ];
    
    for command in test_commands {
        match printer.process_gcode(command).await {
            Ok(()) => tracing::debug!("Processed: {}", command.split(';').next().unwrap_or(command).trim()),
            Err(e) => tracing::warn!("Failed to process '{}': {}", command, e),
        }
        // Small delay between commands
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    tracing::info!("Printer OS is running. Press Ctrl+C to shutdown...");
    
    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => tracing::info!("\nShutdown signal received..."),
        Err(e) => tracing::warn!("Failed to wait for shutdown signal: {}", e),
    }
    
    // Graceful shutdown
    match printer.shutdown().await {
        Ok(()) => tracing::info!("Printer shutdown complete"),
        Err(e) => {
            tracing::error!("Error during shutdown: {}", e);
            return Err(AppError::Other(e.to_string()));
        }
    }
    
    Ok(())
}