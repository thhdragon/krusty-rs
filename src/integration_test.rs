// src/integration_test.rs - Integration test for complete system
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    /// Helper: Parse G-code and queue commands into the manager
    async fn queue_gcode(manager: &crate::print_job::PrintJobManager, gcode: &str) {
        use crate::gcode::parser::{GCodeParser, GCodeParserConfig};
        let mut parser = GCodeParser::new(gcode, GCodeParserConfig::default());
        let mut commands = Vec::new();
        while let Some(cmd) = parser.next_command() {
            commands.push(cmd.map(|c| c.into()));
        }
        manager.queue_commands(commands).await;
    }

    /// Helper: Define and expand a macro, then queue the result
    async fn define_and_queue_macro(manager: &crate::print_job::PrintJobManager, macro_processor: &crate::gcode::macros::MacroProcessor, name: &str, body: Vec<String>, call: &str) {
        macro_processor.define_macro(name, body).await.unwrap();
        let expanded = macro_processor.parse_and_expand_async(call).await;
        manager.queue_commands(expanded).await;
    }

    /// Stub: Complete system initialization (to be implemented)
    #[tokio::test]
    async fn test_complete_system_initialization() {
        // TODO: Implement complete system initialization test
    }

    /// Stub: G-code processing (to be implemented)
    #[tokio::test]
    async fn test_gcode_processing() {
        // TODO: Implement G-code command processing test
    }

    /// Stub: Motion planning (to be implemented)
    #[tokio::test]
    async fn test_motion_planning() {
        // TODO: Implement motion planning and execution test
    }

    /// Stub: Hardware communication (to be implemented)
    #[tokio::test]
    async fn test_hardware_communication() {
        // TODO: Implement hardware communication test
    }

    /// File operations integration test
    #[tokio::test]
    async fn test_file_operations() {
        let file_manager = file::FileManager::new();
        // Test file listing
        let files = file_manager.list_files(".").await.unwrap();
        assert!(!files.is_empty());
        // Test file info
        let info = file_manager.get_file_info("Cargo.toml").await.unwrap();
        assert_eq!(info.name, "Cargo.toml");
        assert!(!info.is_directory);
    }

    /// System state management test
    #[tokio::test]
    async fn test_system_state_management() {
        use std::sync::Arc;
        use tokio::sync::RwLock;
        let state = Arc::new(RwLock::new(host_os::SystemState::default()));
        // Test state updates
        {
            let mut state_write = state.write().await;
            state_write.ready = true;
            state_write.printing = true;
        }
        // Test state reading
        {
            let state_read = state.read().await;
            assert!(state_read.ready);
            assert!(state_read.printing);
        }
    }

    /// End-to-end print job G-code queueing and popping
    #[tokio::test]
    async fn test_print_job_gcode_flow() {
        use crate::print_job::PrintJobManager;
        let manager = PrintJobManager::new();
        queue_gcode(&manager, "G1 X10 Y10 F1500\nG28\n").await;
        // Should be queued
        assert!(manager.state.queued);
        // Should be able to pop commands
        let cmd1 = manager.next_command().await;
        assert!(cmd1.is_some());
        let cmd2 = manager.next_command().await;
        assert!(cmd2.is_some());
        let cmd3 = manager.next_command().await;
        assert!(cmd3.is_none());
    }

    /// Macro expansion and queueing in print job
    #[tokio::test]
    async fn test_macro_expansion_in_print_job() {
        use crate::print_job::PrintJobManager;
        use crate::gcode::macros::MacroProcessor;
        use crate::gcode::parser::GCodeCommand;
        let manager = PrintJobManager::new();
        let macro_processor = MacroProcessor::new();
        define_and_queue_macro(&manager, &macro_processor, "HOME", vec!["G28".to_string()], "{HOME}").await;
        let cmd = manager.next_command().await;
        assert!(cmd.is_some());
        if let Some(Ok(GCodeCommand::Word { letter, value, .. })) = cmd {
            assert_eq!(letter, 'G');
            assert_eq!(value, "28");
        } else {
            panic!("Expected expanded G28 command");
        }
    }

    /// Real-world macro with multiple commands
    #[tokio::test]
    async fn test_real_world_macro_examples() {
        use crate::print_job::PrintJobManager;
        use crate::gcode::macros::MacroProcessor;
        use crate::gcode::parser::GCodeCommand;
        let manager = PrintJobManager::new();
        let macro_processor = MacroProcessor::new();
        define_and_queue_macro(
            &manager,
            &macro_processor,
            "STARTUP",
            vec![
                "G28".to_string(),
                "G1 X0 Y0 Z0 F3000".to_string(),
                "M104 S200".to_string(),
            ],
            "{STARTUP}"
        ).await;
        // Should expand to 3 commands
        let mut count = 0;
        while let Some(cmd) = manager.next_command().await {
            if let Some(Ok(GCodeCommand::Word { .. })) = Some(cmd) {
                count += 1;
            }
        }
        assert_eq!(count, 3);
    }

    /// Stress test: queue and pop 1000 G-code commands
    #[tokio::test]
    async fn test_stress_large_gcode_file() {
        use crate::print_job::PrintJobManager;
        let manager = PrintJobManager::new();
        // Simulate a large G-code file (1000 moves)
        let mut gcode = String::new();
        for i in 0..1000 {
            gcode.push_str(&format!("G1 X{} Y{} F1500\n", i, i));
        }
        queue_gcode(&manager, &gcode).await;
        // Should be able to pop 1000 commands
        let mut count = 0;
        while let Some(cmd) = manager.next_command().await {
            if cmd.is_some() { count += 1; }
        }
        assert_eq!(count, 1000);
    }
}