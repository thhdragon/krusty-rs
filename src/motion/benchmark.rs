//! # Motion Simulation and Benchmark Harness
//!
//! This module provides tools for benchmarking and simulating advanced motion planning, shaper, and blending effects.
//!
//! ## How to Use
//!
//! - To benchmark queueing and update performance, run with the `benchmark` feature enabled:
//!   `cargo run --features benchmark --bin printer-host`
//!
//! - To simulate shaper/blending effects and output CSV for analysis:
//!   Call `shaper_blending_sim::run_shaper_blending_scenarios()` (see code for a wide range of example scenarios).
//!   The output CSV files can be plotted to visualize the effect of different shaper/blending configs.
//!
//! ## Example Output
//!
//! Each CSV row: `t,input,shaped` (time, raw input, shaper output)
//!
//! ## Scenario Coverage
//!
//! - Baseline (no shaper/blending)
//! - Blending only
//! - ZVD shaper (default, high frequency, high damping)
//! - Sine shaper (default, high frequency)
//! - Multi-axis shaping (X: ZVD, Y: Sine)
//! - Aggressive blending
//!
//! ## Best Practices
//!
//! - Start with simulation: Try different shaper types and parameters for each axis, plot the results, and see which settings best suppress vibration and overshoot.
//! - Use the provided scenarios as a starting point for parameter sweeps and tuning.
//! - Compare baseline, blending-only, and advanced configs to understand the impact of each feature.
//! - Tune parameters for your hardware: Use the simulation to guide parameter selection, then validate on your printer.
//! - Troubleshooting:
//!   - If motion feels sluggish, reduce damping or max_deviation.
//!   - If vibration persists, try a different shaper type or adjust frequency.
//!   - If path accuracy is poor, lower `max_deviation` or disable blending.
//!
//! ## Running the Harness
//!
//! 1. Build with the `benchmark` feature:
//!    `cargo run --features benchmark --bin printer-host`
//! 2. Call `shaper_blending_sim::run_shaper_blending_scenarios()` from main or a test.
//! 3. Analyze the generated CSV files (one per scenario) in your favorite plotting tool.
//!
//! See also: `src/config.rs` and `src/motion/planner/mod.rs` for config and planner integration.

// src/motion/benchmark.rs - Performance benchmarking
#[cfg(feature = "benchmark")]
mod benchmark {
    use super::*;
    use std::time::Instant;

    pub async fn benchmark_motion_planning() -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting motion planning benchmark...");
        
        let config = create_benchmark_config();
        let state = Arc::new(RwLock::new(PrinterState {
            ready: false,
            position: [0.0, 0.0, 0.0],
            temperature: 0.0,
            print_progress: 0.0,
        }));
        
        let hardware_manager = create_mock_hardware_manager();
        let motion_config = MotionConfig::new_from_config(&config);
        
        let mut controller = MotionController::new(
            state,
            hardware_manager,
            &config,
        )?;
        
        // Benchmark queueing performance
        let start_time = Instant::now();
        let num_moves = 10000;
        
        for i in 0..num_moves {
            let x = (i as f64) * 0.1;
            let y = (i as f64) * 0.05;
            let z = (i as f64) * 0.01;
            
            controller.queue_linear_move(
                [x, y, z],
                Some(200.0),
                Some(0.1),
            ).await?;
        }
        
        let queue_time = start_time.elapsed();
        println!("Queued {} moves in {:?}", num_moves, queue_time);
        println!("Rate: {:.2} moves/second", num_moves as f64 / queue_time.as_secs_f64());
        
        // Benchmark optimization
        let opt_start = Instant::now();
        controller.optimize_queue().await?;
        let opt_time = opt_start.elapsed();
        
        println!("Optimized queue in {:?}", opt_time);
        
        // Benchmark update loop
        let update_start = Instant::now();
        let num_updates = 100000;
        
        for _ in 0..num_updates {
            controller.update().await?;
        }
        
        let update_time = update_start.elapsed();
        println!("Processed {} updates in {:?}", num_updates, update_time);
        println!("Update rate: {:.2} Hz", num_updates as f64 / update_time.as_secs_f64());
        
        Ok(())
    }

    fn create_benchmark_config() -> Config {
        // Create a config optimized for benchmarking
        Config {
            printer: crate::config::PrinterConfig {
                kinematics: "cartesian".to_string(),
                max_velocity: 500.0,
                max_accel: 5000.0,
                max_z_velocity: 50.0,
                max_z_accel: 200.0,
                printer_name: None,
            },
            mcu: crate::config::McuConfig {
                serial: "/dev/null".to_string(),
                baud: 250000,
                restart_method: None,
            },
            extruder: crate::config::ExtruderConfig {
                step_pin: "PA0".to_string(),
                dir_pin: "PA1".to_string(),
                enable_pin: "PA2".to_string(),
                rotation_distance: 22.67895,
                gear_ratio: Some((50.0, 10.0)),
                microsteps: 16,
                nozzle_diameter: 0.4,
                filament_diameter: 1.75,
            },
            heater_bed: crate::config::HeaterBedConfig {
                heater_pin: "PA3".to_string(),
                sensor_type: "EPCOS 100K B57560G104F".to_string(),
                sensor_pin: "PA4".to_string(),
                min_temp: 0.0,
                max_temp: 130.0,
            },
            steppers: std::collections::HashMap::new(),
        }
    }

    #[cfg(feature = "benchmark")]
    pub async fn benchmark_shaper_effect() -> Result<(), Box<dyn std::error::Error>> {
        use crate::config::{Config, ShaperType, AxisShaperConfig, MotionConfig as UserMotionConfig};
        use crate::motion::shaper::{InputShaperType, PerAxisInputShapers, ZVDShaper};
        use std::collections::HashMap;

        // Example config: ZVD shaper on X
        let mut shaper_map = HashMap::new();
        shaper_map.insert(
            "x".to_string(),
            AxisShaperConfig {
                r#type: ShaperType::Zvd,
                frequency: 40.0,
                damping: Some(0.1),
            },
        );
        let user_motion = UserMotionConfig {
            shaper: shaper_map,
            blending: None,
        };
        let mut config = Config::default();
        config.motion = Some(user_motion);

        // Build input shapers from config
        let mut input_shapers = PerAxisInputShapers::new(4);
        if let Some(motion_cfg) = &config.motion {
            for (axis_name, shaper_cfg) in &motion_cfg.shaper {
                let axis_idx = match axis_name.as_str() {
                    "x" | "X" => 0,
                    "y" | "Y" => 1,
                    "z" | "Z" => 2,
                    "e" | "E" => 3,
                    _ => continue,
                };
                let shaper = match shaper_cfg.r#type {
                    ShaperType::Zvd => {
                        let delay = 1;
                        let coeffs = [1.0, 0.0];
                        InputShaperType::ZVD(ZVDShaper::new(delay, coeffs))
                    }
                    ShaperType::Sine => {
                        let magnitude = 1.0;
                        let frequency = shaper_cfg.frequency as f64;
                        let sample_time = 0.01;
                        InputShaperType::SineWave(crate::motion::shaper::SineWaveShaper::new(magnitude, frequency, sample_time))
                    }
                };
                input_shapers.set_shaper(axis_idx, shaper);
            }
        }

        // Simulate a step input on X
        let mut raw_x = 0.0;
        let mut shaped_x = 0.0;
        println!("step,raw_x,shaped_x");
        for step in 0..100 {
            if step == 10 { raw_x = 1.0; } // Step input at t=10
            shaped_x = input_shapers.do_step(0, raw_x);
            println!("{},{},{}", step, raw_x, shaped_x);
        }
        Ok(())
    }

    #[cfg(feature = "benchmark")]
    pub mod shaper_blending_sim {
        use super::*;
        use std::fs::File;
        use std::io::Write;

        /// Simulate and output motion profiles and shaper effects for different configs
        pub fn run_shaper_blending_scenarios() -> Result<(), Box<dyn std::error::Error>> {
            let scenarios = vec![
                // Baseline: No shaper, no blending
                ("baseline", r#""#),
                // Blending only
                ("blending_only", r#"
                    [motion.blending]
                    type = "bezier"
                    max_deviation = 0.15
                "#),
                // ZVD shaper on X, default params
                ("zvd_x", r#"
                    [motion.shaper.x]
                    type = "zvd"
                    frequency = 40.0
                    damping = 0.1
                    [motion.blending]
                    type = "bezier"
                    max_deviation = 0.2
                "#),
                // ZVD shaper on X, higher frequency
                ("zvd_x_highfreq", r#"
                    [motion.shaper.x]
                    type = "zvd"
                    frequency = 60.0
                    damping = 0.1
                    [motion.blending]
                    type = "bezier"
                    max_deviation = 0.2
                "#),
                // ZVD shaper on X, higher damping
                ("zvd_x_highdamping", r#"
                    [motion.shaper.x]
                    type = "zvd"
                    frequency = 40.0
                    damping = 0.5
                    [motion.blending]
                    type = "bezier"
                    max_deviation = 0.2
                "#),
                // Sine shaper on Y, default params
                ("sine_y", r#"
                    [motion.shaper.y]
                    type = "sine"
                    frequency = 35.0
                    [motion.blending]
                    type = "bezier"
                    max_deviation = 0.1
                "#),
                // Sine shaper on Y, higher frequency
                ("sine_y_highfreq", r#"
                    [motion.shaper.y]
                    type = "sine"
                    frequency = 60.0
                    [motion.blending]
                    type = "bezier"
                    max_deviation = 0.1
                "#),
                // Multi-axis: X=ZVD, Y=Sine
                ("multi_axis", r#"
                    [motion.shaper.x]
                    type = "zvd"
                    frequency = 40.0
                    damping = 0.1
                    [motion.shaper.y]
                    type = "sine"
                    frequency = 35.0
                    [motion.blending]
                    type = "bezier"
                    max_deviation = 0.2
                "#),
                // Aggressive blending
                ("aggressive_blending", r#"
                    [motion.blending]
                    type = "bezier"
                    max_deviation = 0.5
                "#),
            ];
            for (name, toml_str) in scenarios {
                let config: crate::config::Config = if toml_str.trim().is_empty() {
                    crate::config::Config::default()
                } else {
                    toml::from_str(toml_str)?
                };
                let planner = crate::motion::planner::MotionPlanner::new_from_config(&config);
                // Simulate a simple move for the axis with a shaper, or X by default
                let axis = if planner.input_shapers.shapers[0].is_some() {
                    0
                } else if planner.input_shapers.shapers[1].is_some() {
                    1
                } else {
                    0 // Default to X if no shaper
                };
                let mut file = File::create(format!("shaper_blending_{}.csv", name))?;
                writeln!(file, "t,input,shaped")?;
                let mut input = 0.0;
                for i in 0..1000 {
                    let t = i as f64 * 0.001;
                    input = (t * 10.0).sin(); // Example input signal
                    let shaped = planner.input_shapers.shapers[axis]
                        .as_ref()
                        .map(|s| s.do_step(input))
                        .unwrap_or(input);
                    writeln!(file, "{:.4},{:.4},{:.4}", t, input, shaped)?;
                }
            }
            println!("Shaper/blending simulation complete. See CSV files for results.");
            Ok(())
        }
    }

    /*
    Simulation Harness Usage:
    ------------------------
    - To run the shaper effect simulation, build with the `benchmark` feature and call `benchmark_shaper_effect()`.
    - Example (from main or test):
        tokio::runtime::Runtime::new().unwrap().block_on(benchmark_shaper_effect()).unwrap();
    - The output is CSV: step,raw_x,shaped_x. Plot in your favorite tool (Excel, Python, etc).
    - To test different configs, edit the shaper type/params in the harness or load from a TOML file.
    - Compare shaped vs. unshaped output to analyze vibration reduction and smoothness.

    - To run the shaper/blending simulation, call `shaper_blending_sim::run_shaper_blending_scenarios()`.
    - This will generate CSV files for each scenario in the current directory.
    - Open the CSV files in a spreadsheet program or analyze with a script.
    */
}