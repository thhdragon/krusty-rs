//! CLI entry point for the simulation harness: batch parameter sweeps, config-driven runs, and CSV output.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Simulation Harness CLI
#[derive(Parser, Debug)]
#[command(name = "sim-harness", about = "Motion simulation harness for parameter sweeps and benchmarking.")]
pub struct Cli {
    /// Path to a TOML config file (overrides defaults)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Output directory for CSV/logs
    #[arg(short, long, default_value = "./sim_output")] 
    output: PathBuf,

    /// Scenario to run (built-in or custom)
    #[arg(long)]
    scenario: Option<String>,

    /// Parameter override (e.g. --param motion.shaper.x.frequency=40.0)
    #[arg(long, value_parser = parse_key_val, number_of_values = 1)]
    param: Vec<(String, String)>,

    /// Parameter sweep (e.g. --sweep motion.shaper.x.frequency=30:5:60)
    #[arg(long, value_parser = parse_key_val, number_of_values = 1)]
    sweep: Vec<(String, String)>,

    /// Show progress bar
    #[arg(long)]
    progress: bool,

    /// Enable colored output
    #[arg(long)]
    color: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List available built-in scenarios
    ListScenarios,
    /// Run a single scenario (default)
    Run,
    /// Run a parameter sweep
    Sweep,
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s.find('=');
    match pos {
        Some(pos) => Ok((s[..pos].to_string(), s[pos + 1..].to_string())),
        None => Err(format!("Invalid KEY=VAL: no `=` found in '{}'.", s)),
    }
}

fn main() {
    // Bring the Parser trait into scope for .parse()
    use clap::Parser as _;
    let cli = Cli::parse();

    // 1. Load config (default or from file)
    let config = if let Some(ref path) = cli.config {
        match krusty_rs::config::load_config(path.to_str().expect("Invalid config path")) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Failed to load config: {e}");
                std::process::exit(1);
            }
        }
    } else {
        krusty_rs::config::Config::default()
    };

    // 2. Apply parameter overrides (flat key=value, e.g. motion.shaper.x.frequency=40.0)
    for (key, val) in &cli.param {
        // TODO: Implement robust key-path override (parse key, update config struct)
        println!("Override param: {}={}", key, val);
        // For now, just print; will implement actual override logic next
    }

    // 3. Ensure output directory exists
    if let Err(e) = std::fs::create_dir_all(&cli.output) {
        eprintln!("Failed to create output directory {}: {e}", cli.output.display());
        std::process::exit(1);
    }

    // 4. Dispatch command
    match &cli.command {
        Some(Commands::ListScenarios) => {
            // TODO: List built-in scenarios
            println!("Available scenarios: baseline, blending_only, zvd_x, zvd_x_highfreq, zvd_x_highdamping, sine_y, sine_y_highfreq, multi_axis, aggressive_blending");
        }
        Some(Commands::Sweep) => {
            // TODO: Implement parameter sweep logic
            println!("Parameter sweep not yet implemented.");
        }
        _ => {
            // Default: run single scenario
            println!("Running scenario: {:?}", cli.scenario.as_deref().unwrap_or("baseline"));
            // --- Begin simulation setup ---
            use std::sync::{Arc, Mutex};
            use krusty_rs::simulator::event_queue::{SimEventQueue, SimClock, SimEvent, SimEventType};
            use std::time::Duration;
            // 1. Create event queue and clock
            let event_queue = Arc::new(Mutex::new(SimEventQueue::new()));
            let clock = SimClock::new();
            // 2. Schedule initial HeaterUpdate event
            {
                let mut queue = event_queue.lock().unwrap();
                queue.push(SimEvent {
                    timestamp: Duration::from_secs(0),
                    event_type: SimEventType::HeaterUpdate,
                    payload: None,
                });
            }
            // 3. Instantiate Simulator and run event loop
            let mut sim = krusty_rs::simulator::Simulator::new(event_queue.clone(), clock);
            let sim_timeout = std::time::Duration::from_secs(90); // 90s simulated time
            sim.run_event_loop_with_timeout(sim_timeout);
            // --- End simulation setup ---
        }
    }
}
