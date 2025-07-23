// src/main.rs - Fixed main application
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
    tracing_subscriber::fmt::init();
    
    println!("Starting Krusty-RS 3D Printer OS...");
    
    // Get configuration file path
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "printer.toml"
    };
    
    println!("Loading configuration from: {}", config_path);
    
    // Load configuration
    let config = match config::load_config(config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            return Err(e);
        }
    };
    
    println!("Configuration loaded successfully");
    
    // Create and start printer
    let mut printer = Printer::new(config).await?;
    printer.start().await?;
    
    // Test some G-code commands
    println!("Testing G-code commands...");
    printer.process_gcode("G28").await?;
    printer.process_gcode("G1 X100 Y100 Z10 F3000").await?;
    printer.process_gcode("G1 E10 F200").await?;
    
    println!("Printer OS is running. Press Ctrl+C to shutdown...");
    
    // Wait for shutdown signal
    signal::ctrl_c().await?;
    println!("\nShutting down...");
    printer.shutdown().await?;
    
    println!("Shutdown complete");
    Ok(())
}