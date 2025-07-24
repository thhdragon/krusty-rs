// src/main.rs - Enhanced main with motion mode selection
mod printer;
mod gcode;
mod motion;
mod hardware;
mod config;
mod file_manager;
mod web;

use printer::Printer;
use std::env;
use std::sync::{Arc, Mutex};
use web::printer_channel::{PrinterRequest};
use tokio::sync::mpsc;
use tokio::task::LocalSet;

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
    
    // Create the main Printer object.
    let mut printer = match Printer::new(config).await { // The `mut` is needed for the printer task loop
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to initialize printer: {}", e);
            return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>);
        }
    };

    // Set up a channel for communication between Axum handlers and the printer task.
    let (printer_tx, mut printer_rx) = mpsc::channel::<PrinterRequest>(16);

    // Create a LocalSet for !Send tasks.
    let local = LocalSet::new();

    // Spawn the background printer task on the local set.
    local.spawn_local(async move {
        while let Some(request) = printer_rx.recv().await {
            match request {
                PrinterRequest::GetStatus { respond_to } => {
                    let state = printer.get_state().await;
                    // Map to API response type
                    let response = web::models::PrinterStatusResponse {
                        status: if state.ready { "Idle".to_string() } else { "Not Ready".to_string() },
                        position: (
                            state.position[0] as f32,
                            state.position[1] as f32,
                            state.position[2] as f32
                        ),
                        hotend_temp: state.temperature as f32,
                        target_hotend_temp: state.bed_temperature as f32,
                    };
                    let _ = respond_to.send(response);
                }
                PrinterRequest::ExecuteGcode { command, respond_to } => {
                    let result = printer.process_gcode(&command).await.map_err(|e| e.to_string());
                    let _ = respond_to.send(result);
                }
            }
        }
    });

    // Create the Axum router, passing it the channel sender.
    let app = web::api::create_router(printer_tx);

    // Start the web server and the local set.
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Web API listening on http://{}", listener.local_addr()?);
    local.spawn_local(async move {
        axum::serve(listener, app).await.unwrap();
    });
    local.await;

    Ok(())
}