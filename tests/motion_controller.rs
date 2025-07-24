// Integration tests for motion controller (moved from src/motion/test.rs)

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use krusty_rs::{PrinterState, HardwareManager, Config, MotionConfig, MotionController};

    #[tokio::test]
    async fn test_motion_controller_creation() {
        let config = create_test_config();
        let state = Arc::new(RwLock::new(PrinterState {
            ready: false,
            position: [0.0, 0.0, 0.0],
            temperature: 0.0,
            bed_temperature: 0.0,
            print_progress: 0.0,
            paused: false,
            printing: false,
        }));
        let hardware_manager = create_mock_hardware_manager(&config);
        let controller = MotionController::new(
            state,
            hardware_manager,
            &config,
        );
        // If new() returns the controller directly, just assert type
        let _controller: MotionController = controller;
    }

    #[tokio::test]
    async fn test_linear_move_queueing() {
        let config = create_test_config();
        let state = Arc::new(RwLock::new(PrinterState {
            ready: false,
            position: [0.0, 0.0, 0.0],
            temperature: 0.0,
            bed_temperature: 0.0,
            print_progress: 0.0,
            paused: false,
            printing: false,
        }));
        let hardware_manager = create_mock_hardware_manager(&config);
        let mut controller = MotionController::new(
            state,
            hardware_manager,
            &config,
        );
        let result = controller.queue_linear_move(
            [10.0, 10.0, 10.0],
            Some(100.0),
            Some(5.0),
        ).await;
        assert!(result.is_ok());
        assert_eq!(controller.get_queue_stats().length, 1);
    }

    #[tokio::test]
    async fn test_emergency_stop() {
        let config = create_test_config();
        let state = Arc::new(RwLock::new(PrinterState {
            ready: false,
            position: [0.0, 0.0, 0.0],
            temperature: 0.0,
            bed_temperature: 0.0,
            print_progress: 0.0,
            paused: false,
            printing: false,
        }));
        let hardware_manager = create_mock_hardware_manager(&config);
        let mut controller = MotionController::new(
            state,
            hardware_manager,
            &config,
        );
        controller.queue_linear_move([10.0, 0.0, 0.0], Some(100.0), None).await.unwrap();
        controller.queue_linear_move([20.0, 0.0, 0.0], Some(100.0), None).await.unwrap();
        assert_eq!(controller.get_queue_stats().length, 2);
        controller.emergency_stop();
        assert_eq!(controller.get_queue_stats().length, 0);
    }

    fn create_test_config() -> Config {
        Config {
            printer: krusty_rs::config::PrinterConfig {
                kinematics: "cartesian".to_string(),
                max_velocity: 300.0,
                max_accel: 3000.0,
                max_z_velocity: 25.0,
                max_z_accel: 100.0,
                printer_name: None,
            },
            mcu: krusty_rs::config::McuConfig {
                serial: "/dev/null".to_string(),
                baud: 250000,
            },
            extruder: krusty_rs::config::ExtruderConfig {
                step_pin: "PA0".to_string(),
                dir_pin: "PA1".to_string(),
                enable_pin: "PA2".to_string(),
                rotation_distance: 22.67895,
                gear_ratio: Some((50.0, 10.0)),
                microsteps: 16,
                nozzle_diameter: 0.4,
                filament_diameter: 1.75,
            },
            heater_bed: krusty_rs::config::HeaterBedConfig {
                heater_pin: "PA3".to_string(),
                sensor_type: "EPCOS 100K B57560G104F".to_string(),
                sensor_pin: "PA4".to_string(),
                min_temp: 0.0,
                max_temp: 130.0,
            },
            steppers: std::collections::HashMap::new(),
            motion: None, // Add default or test motion config if needed
        }
    }

    fn create_mock_hardware_manager(config: &Config) -> HardwareManager {
        HardwareManager::new(config.clone())
    }
}
