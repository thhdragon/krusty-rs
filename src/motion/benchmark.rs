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
            motion_config,
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
}