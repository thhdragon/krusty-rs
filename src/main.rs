// src/main.rs - Updated to test motion planning
mod printer;
mod gcode;
mod motion;
mod hardware;
mod config;

use printer::Printer;
use tokio::signal;
use std::env;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("Starting Krusty-RS 3D Printer OS");
    
    // Get configuration file path
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "printer.toml"
    };
    
    // Load configuration
    let config = config::load_config(config_path)?;
    
    // Create and start printer
    let mut printer = Printer::new(config).await.map_err(|e| Box::<dyn std::error::Error>::from(e))?;
    printer.start().await.map_err(|e| Box::<dyn std::error::Error>::from(e))?;
    
    info!("Printer OS is running. Press Ctrl+C to shutdown...");
    
    // Wait for shutdown signal
    signal::ctrl_c().await.map_err(|e| Box::<dyn std::error::Error>::from(e))?;
    info!("\nShutting down...");
    printer.shutdown().await.map_err(|e| Box::<dyn std::error::Error>::from(e))?;
    
    Ok(())
}