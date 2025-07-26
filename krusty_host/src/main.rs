use std::env;
use tokio::sync::mpsc;
use tokio::task::LocalSet;
use krusty_shared::board_config::BoardConfig;
#[cfg(feature = "sim-in-host")]
use krusty_shared::event_queue::{SimEventQueue, SimClock};
use hardware::HardwareManager;
use motion::MotionSystem;
#[cfg(feature = "sim-in-host")]
use krusty_simulator::simulator::Simulator;
use krusty_host::*;

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
    
    // Instantiate board config from loaded config
    let board_name = config.printer.printer_name.clone().unwrap_or("DefaultBoard".to_string());
    let board = BoardConfig::new(&board_name);
    // Create event queue and simulation clock
    #[cfg(feature = "sim-in-host")]
    let event_queue = Arc::new(Mutex::new(SimEventQueue::new()));
    #[cfg(feature = "sim-in-host")]
    let clock = SimClock::new();
    // Pass to hardware manager, motion system, and simulator
    let _hardware_manager = HardwareManager::new(config.clone(), board.clone());
    let _motion_system = {
        #[cfg(feature = "sim-in-host")]
        { MotionSystem::new(event_queue.clone(), board.clone()) }
        #[cfg(not(feature = "sim-in-host"))]
        { MotionSystem::new(board.clone()) }
    };
    #[cfg(feature = "sim-in-host")]
    let simulator = Simulator::new(event_queue, clock);

    // Set up a channel for communication between Axum handlers and the (now-removed) printer task.
    let (printer_tx, _printer_rx) = mpsc::channel::<web::printer_channel::PrinterRequest>(16);

    // Create a LocalSet for !Send tasks.
    let local = LocalSet::new();

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