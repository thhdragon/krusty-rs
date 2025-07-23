// src/main.rs - Updated to test motion planning
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
    
    // Get configuration file path
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "printer.toml"
    };
    
    // Load configuration
    let config = match config::load_config(config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!("Failed to load config: {}", e);
            return Err(e);
        }
    };
    
    // Create and start printer
    let mut printer = Printer::new(config).await?;
    printer.start().await?;
    
    tracing::info!("Printer OS is running. Press Ctrl+C to shutdown...");
    
    // Wait for shutdown signal
    signal::ctrl_c().await?;
    tracing::info!("\nShutting down...");
    printer.shutdown().await?;
    
    Ok(())
}