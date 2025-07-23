// src/main.rs - Fixed main function
mod printer;
mod gcode;
mod motion;
mod hardware;
mod config;

use printer::Printer;
use tokio::signal;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    tracing::info!("Starting Krusty-RS 3D Printer OS");
    // Remove the build time line or replace with:
    tracing::info!("Version: 0.1.0");
    
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
            return Err(e);
        }
    };
    
    // Display basic config info
    tracing::info!("Printer configuration:");
    tracing::info!("  MCU: {} @ {} baud", config.mcu.serial, config.mcu.baud);
    tracing::info!("  Max velocity: {} mm/s", config.printer.max_velocity);
    tracing::info!("  Max acceleration: {} mm/s²", config.printer.max_accel);
    tracing::info!("  Steppers configured: {}", config.steppers.len());
    
    // Create and start printer
    let mut printer = match Printer::new(config).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to initialize printer: {}", e);
            return Err(e);
        }
    };
    
    match printer.start().await {
        Ok(()) => tracing::info!("Printer OS started successfully"),
        Err(e) => {
            tracing::error!("Failed to start printer: {}", e);
            return Err(e);
        }
    }
    
    // Test some G-code commands
    tracing::info!("Testing G-code commands...");
    let test_commands = vec![
        "G28 ; Home all axes",
        "G1 X100 Y100 Z10 F3000 ; Move to position",
        "G1 E10 F200 ; Extrude 10mm",
        "M104 S200 ; Set hotend temperature",
        "M140 S60 ; Set bed temperature",
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
        Err(e) => tracing::error!("Error during shutdown: {}", e),
    }
    
    Ok(())
}