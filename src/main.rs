// src/main.rs - Enhanced main with motion mode selection
mod printer;
mod gcode;
mod motion;
mod hardware;
mod config;
mod file_manager;

use printer::Printer;
use std::env;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    tracing::info!("Starting Krusty-RS 3D Printer OS");
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
    let config = config::load_config(config_path)
        .map_err(|e| {
            tracing::error!("Failed to load config from '{}': {}", config_path, e);
            tracing::error!("Please ensure the configuration file exists and is properly formatted");
            Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>
        })?;
    
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
            return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>);
        }
    };
    if let Err(e) = printer.start().await {
        tracing::error!("Failed to start printer: {}", e);
        return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>);
    }
    
    // Test different motion modes
    tracing::info!("Testing different motion modes...");
    
    // Test basic motion
    test_motion_modes(&mut printer).await?;
    
    tracing::info!("Motion mode testing complete");
    tracing::info!("Printer OS is running. Press Ctrl+C to shutdown...");
    
    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => tracing::info!("\nShutdown signal received..."),
        Err(e) => tracing::warn!("Failed to wait for shutdown signal: {}", e),
    }
    
    // Graceful shutdown
    if let Err(e) = printer.shutdown().await {
        tracing::error!("Error during shutdown: {}", e);
        return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>);
    }
    
    Ok(())
}

async fn test_motion_modes(printer: &mut printer::Printer) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
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
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    Ok(())
}