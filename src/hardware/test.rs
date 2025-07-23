// src/hardware/test.rs - Integration tests for hardware components
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_step_generator_basic() {
        let mut step_gen = StepGenerator::new(
            [80.0, 80.0, 400.0, 100.0], // Steps per mm
            [false, false, false, false], // No direction inversion
        );
        
        // Test position to steps conversion
        let position = [1.0, 2.0, 3.0, 4.0];
        let steps = step_gen.position_to_steps(&position);
        
        assert_eq!(steps[0], 80);   // 1.0 * 80
        assert_eq!(steps[1], 160);  // 2.0 * 80
        assert_eq!(steps[2], 1200); // 3.0 * 400
        assert_eq!(steps[3], 400);  // 4.0 * 100
        
        // Test step generation
        let commands = step_gen.generate_steps(&position);
        assert_eq!(commands.len(), 4); // All axes moved
        
        // Test step commands
        for (i, command) in commands.iter().enumerate() {
            assert_eq!(command.steps, steps[i] as u32);
            assert_eq!(command.direction, true); // Positive movement
        }
    }

    #[tokio::test]
    async fn test_step_generator_direction_inversion() {
        let mut step_gen = StepGenerator::new(
            [100.0, 100.0, 100.0, 100.0],
            [true, false, true, false], // Invert X and Z
        );
        
        let position = [1.0, 1.0, 1.0, 1.0];
        let commands = step_gen.generate_steps(&position);
        
        // X axis should be inverted (negative direction)
        assert_eq!(commands[0].direction, false);
        // Y axis should not be inverted (positive direction)
        assert_eq!(commands[1].direction, true);
        // Z axis should be inverted (negative direction)
        assert_eq!(commands[2].direction, false);
        // E axis should not be inverted (positive direction)
        assert_eq!(commands[3].direction, true);
    }

    #[tokio::test]
    async fn test_step_generator_no_movement() {
        let mut step_gen = StepGenerator::new(
            [100.0, 100.0, 100.0, 100.0],
            [false, false, false, false],
        );
        
        // Set current position
        step_gen.reset_steps();
        
        // Generate steps to same position
        let commands = step_gen.generate_steps(&[0.0, 0.0, 0.0, 0.0]);
        assert_eq!(commands.len(), 0); // No movement
    }

    #[tokio::test]
    async fn test_step_command_conversion() {
        let command = StepCommand {
            axis: Axis::X,
            steps: 100,
            direction: true,
            timing: None,
            callback: None,
        };
        
        let mcu_command = command.to_mcu_command();
        assert_eq!(mcu_command, "step X 100 1");
        
        let command = StepCommand {
            axis: Axis::E,
            steps: 50,
            direction: false,
            timing: None,
            callback: None,
        };
        
        let mcu_command = command.to_mcu_command();
        assert_eq!(mcu_command, "step E 50 0");
    }

    #[tokio::test]
    async fn test_axis_conversion() {
        assert_eq!(Axis::from_char('X'), Some(Axis::X));
        assert_eq!(Axis::from_char('y'), Some(Axis::Y));
        assert_eq!(Axis::from_char('Z'), Some(Axis::Z));
        assert_eq!(Axis::from_char('e'), Some(Axis::E));
        assert_eq!(Axis::from_char('A'), None);
    }

    #[tokio::test]
    async fn test_step_buffer() {
        let mut buffer = StepBuffer::new(3);
        
        let command1 = StepCommand {
            axis: Axis::X,
            steps: 100,
            direction: true,
            timing: None,
            callback: None,
        };
        
        let command2 = StepCommand {
            axis: Axis::Y,
            steps: 200,
            direction: false,
            timing: None,
            callback: None,
        };
        
        // Add commands to buffer
        assert!(buffer.push(command1.clone()).is_ok());
        assert!(buffer.push(command2.clone()).is_ok());
        assert_eq!(buffer.len(), 2);
        
        // Try to add third command
        let command3 = StepCommand {
            axis: Axis::Z,
            steps: 300,
            direction: true,
            timing: None,
            callback: None,
        };
        assert!(buffer.push(command3.clone()).is_ok());
        assert_eq!(buffer.len(), 3);
        
        // Try to add fourth command (should fail)
        let command4 = StepCommand {
            axis: Axis::E,
            steps: 400,
            direction: false,
            timing: None,
            callback: None,
        };
        assert!(buffer.push(command4).is_err());
        
        // Test buffer iteration
        assert_eq!(buffer.next(), Some(command1));
        assert_eq!(buffer.next(), Some(command2));
        assert_eq!(buffer.next(), Some(command3));
        assert_eq!(buffer.next(), None);
        
        // Reset and clear
        buffer.reset();
        assert_eq!(buffer.next(), Some(command1));
        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[tokio::test]
    async fn test_step_timing_calculation() {
        let step_gen = StepGenerator::new(
            [100.0, 100.0, 100.0, 100.0],
            [false, false, false, false],
        );
        
        let commands = vec![
            StepCommand {
                axis: Axis::X,
                steps: 100,
                direction: true,
                timing: Some(StepTiming {
                    pulse_width: 2,
                    step_interval: 5,
                    direction_setup: 1,
                    enable_timing: EnableTiming {
                        pre_enable_delay: 0,
                        post_step_delay: 0,
                        disable_delay: 0,
                    },
                }),
                callback: None,
            }
        ];
        
        let min_time = step_gen.calculate_minimum_time(&commands);
        // Direction setup (1) + steps * interval (100 * 5) + steps * pulse (100 * 2)
        assert_eq!(min_time, 1 + 500 + 200);
    }
}