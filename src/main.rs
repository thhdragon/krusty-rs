// src/main.rs - Updated main with enhanced configuration
mod printer;
mod gcode;
mod motion;
mod hardware;
mod config;
mod host_os;
mod file;
mod web;

use host_os::PrinterHostOS;
use tokio::signal;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    tracing::info!("Starting Rust 3D Printer Host OS");
    tracing::info!("Version: {}", env!("CARGO_PKG_VERSION"));
    
    // Get configuration file path from command line or use default
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "printer.toml" // Default configuration file
    };
    
    tracing::info!("Loading configuration from: {}", config_path);
    
    // Load and validate configuration
    let config = match config::Config::load_config(config_path) {
        Ok(cfg) => {
            tracing::info!("Configuration loaded successfully");
            match cfg.validate() {
                Ok(()) => {
                    tracing::info!("Configuration validated successfully");
                    cfg
                }
                Err(e) => {
                    tracing::error!("Configuration validation failed: {}", e);
                    return Err(e);
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to load configuration: {}", e);
            tracing::error!("Please ensure {} exists and is properly formatted", config_path);
            return Err(e);
        }
    };
    
    // Display some configuration info
    tracing::info!("Printer: {} ({})", config.printer.printer_name, config.printer.kinematics);
    tracing::info!("Max velocity: {} mm/s", config.printer.max_velocity);
    tracing::info!("Max acceleration: {} mm/s²", config.printer.max_accel);
    tracing::info!("MCU: {} @ {} baud", config.mcu.serial, config.mcu.baud);
    
    // Show configured steppers
    for (name, stepper) in &config.steppers {
        let steps_per_mm = config.get_steps_per_mm(name);
        tracing::info!("Stepper {}: {} steps/mm", name, steps_per_mm);
    }
    
    // Create and initialize the host OS with loaded configuration
    let mut host_os = PrinterHostOS::new_with_config(config).await?;
    
    // Initialize all systems
    host_os.initialize().await?;
    
    // Start processing loops
    host_os.start().await?;
    
    // Print system information
    let system_info = host_os.get_system_info().await;
    tracing::info!("System ready: {} v{} (Rust {})", 
                  system_info.name, 
                  system_info.version, 
                  system_info.rust_version);
    
    tracing::info!("Host OS is running. Press Ctrl+C to shutdown...");
    
    // Wait for shutdown signal
    signal::ctrl_c().await?;
    tracing::info!("\nShutdown signal received...");
    
    // Graceful shutdown
    host_os.shutdown().await?;
    
    tracing::info!("Host OS shutdown complete");
    Ok(())
}