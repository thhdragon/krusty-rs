// Tests for gcode macros (moved from src/gcode/macros.rs)

#[cfg(test)]
mod tests {
    use krusty_rs::gcode::macros::*;
    use krusty_rs::gcode::parser::*;
    use tokio::runtime::Runtime;
    use std::io::Cursor;

    // ...existing code from macros.rs tests...
    // (Insert all #[test] functions here, as previously cataloged)

    #[tokio::test]
    async fn test_nested_macro_expansion() {
        let macro_processor = MacroProcessor::new();
        // Define macro B: expands to G1 X10
        macro_processor.define_macro("B", vec!["G1 X10".to_string()]).await.unwrap();
        // Define macro A: expands to {B} and G2 X20
        macro_processor.define_macro("A", vec!["{B}".to_string(), "G2 X20".to_string()]).await.unwrap();
        // Input: {A}\nG3 X30
        let gcode = "{A}\nG3 X30";
        let config = GCodeParserConfig {
            enable_comments: false,
            enable_checksums: false,
            enable_infix: false,
            enable_macros: true,
            enable_vendor_extensions: false,
        };
        use futures_util::io::{BufReader, AllowStdIo};
        let cursor = Cursor::new(gcode.as_bytes());
        let async_reader = AllowStdIo::new(cursor);
        let reader = BufReader::new(async_reader);
        let mut parser = AsyncGCodeParser::new(reader, config).with_macro_expander(Box::new(macro_processor.clone()));
        let mut commands = Vec::new();
        use futures_util::stream::StreamExt;
        while let Some(cmd) = parser.next().await {
            match cmd {
                Ok(OwnedGCodeCommand::Word { letter, value, .. }) => commands.push((letter, value)),
                _ => {}
            }
        }
        // Expect: G1 X10 (from B), G2 X20 (from A), G3 X30 (from source)
        assert_eq!(commands, vec![('G', "1 X10".to_string()), ('G', "2 X20".to_string()), ('G', "3 X30".to_string())]);
    }

    #[tokio::test]
    async fn test_macro_error_recovery() {
        let macro_processor = MacroProcessor::new();
        // Only define macro A
        macro_processor.define_macro("A", vec!["G1 X10".to_string()]).await.unwrap();
        // Input: {B}\nG2 X20 (B is undefined)
        let gcode = "{B}\nG2 X20";
        let config = GCodeParserConfig {
            enable_comments: false,
            enable_checksums: false,
            enable_infix: false,
            enable_macros: true,
            enable_vendor_extensions: false,
        };
        use futures_util::io::{BufReader, AllowStdIo};
        use std::io::Cursor;
        let cursor = Cursor::new(gcode.as_bytes());
        let async_reader = AllowStdIo::new(cursor);
        let reader = BufReader::new(async_reader);
        let mut parser = AsyncGCodeParser::new(reader, config).with_macro_expander(Box::new(macro_processor.clone()));
        let mut errors = Vec::new();
        let mut commands = Vec::new();
        use futures_util::stream::StreamExt;
        while let Some(cmd) = parser.next().await {
            match cmd {
                Ok(OwnedGCodeCommand::Word { letter, value, .. }) => commands.push((letter, value)),
                Err(e) => errors.push(e.message),
                _ => {}
            }
        }
        // Should report error for missing macro B, but still parse G2 X20
        assert!(errors.iter().any(|msg| msg.contains("not found")));
        assert_eq!(commands, vec![('G', "2 X20".to_string())]);
    }

    #[tokio::test]
    async fn test_command_injection_order() {
        let macro_processor = MacroProcessor::new();
        // Define macro A: expands to G1 X10, G2 X20
        macro_processor.define_macro("A", vec!["G1 X10".to_string(), "G2 X20".to_string()]).await.unwrap();
        // Input: {A}\nG3 X30
        let gcode = "{A}\nG3 X30";
        let config = GCodeParserConfig {
            enable_comments: false,
            enable_checksums: false,
            enable_infix: false,
            enable_macros: true,
            enable_vendor_extensions: false,
        };
        use futures_util::io::{BufReader, AllowStdIo};
        use std::io::Cursor;
        let cursor = Cursor::new(gcode.as_bytes());
        let async_reader = AllowStdIo::new(cursor);
        let reader = BufReader::new(async_reader);
        let mut parser = AsyncGCodeParser::new(reader, config).with_macro_expander(Box::new(macro_processor.clone()));
        let mut commands = Vec::new();
        use futures_util::stream::StreamExt;
        while let Some(cmd) = parser.next().await {
            match cmd {
                Ok(OwnedGCodeCommand::Word { letter, value, .. }) => commands.push((letter, value)),
                _ => {}
            }
        }
        // Expect: G1 X10, G2 X20 (from macro), G3 X30 (from source)
        assert_eq!(commands, vec![('G', "1 X10".to_string()), ('G', "2 X20".to_string()), ('G', "3 X30".to_string())]);
    }
}
