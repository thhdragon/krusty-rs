// src/gcode/macros.rs
use crate::gcode::gcode_executor::GCodeExecutor;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// MacroProcessor manages G-code macros and provides expansion and dispatch utilities.
///
/// # Example
/// ```rust
/// use your_crate::gcode::macros::MacroProcessor;
/// use tokio::runtime::Runtime;
/// let rt = Runtime::new().unwrap();
/// rt.block_on(async {
///     let macro_processor = MacroProcessor::new();
///     macro_processor.define_macro("HOME", vec!["G28".to_string()]).await.unwrap();
///     macro_processor.execute_macro_and_dispatch("HOME", |cmd| {
///         // Send `cmd` to your motion or print job system here
///         println!("Dispatching: {:?}", cmd);
///     }).await.unwrap();
/// });
/// ```
#[derive(Debug, Clone)]
pub struct MacroProcessor {
    macros: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl MacroProcessor {
    pub fn new() -> Self {
        Self {
            macros: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Define a new macro
    pub async fn define_macro(&self, name: &str, commands: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        let mut macros = self.macros.write().await;
        macros.insert(name.to_string(), commands);
        tracing::info!("Defined macro: {}", name);
        Ok(())
    }

    /// Execute a macro
    pub async fn execute_macro(&self, name: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let macros = self.macros.read().await;
        if let Some(commands) = macros.get(name) {
            tracing::info!("Executing macro: {}", name);
            Ok(commands.clone())
        } else {
            Err(format!("Macro '{}' not found", name).into())
        }
    }

    /// List all available macros
    pub async fn list_macros(&self) -> Vec<String> {
        let macros = self.macros.read().await;
        macros.keys().cloned().collect()
    }

    /// Delete a macro
    pub async fn delete_macro(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut macros = self.macros.write().await;
        if macros.remove(name).is_some() {
            tracing::info!("Deleted macro: {}", name);
            Ok(())
        } else {
            Err(format!("Macro '{}' not found", name).into())
        }
    }

    /// Parse and expand a macro asynchronously
    pub async fn parse_and_expand_async(&self, command: &str) -> Vec<Result<crate::gcode::parser::GCodeCommand<'static>, crate::gcode::parser::GCodeError>> {
        use crate::gcode::parser::{GCodeParser, GCodeParserConfig, GCodeCommand, GCodeError, OwnedGCodeCommand};
        let mut results = Vec::new();
        let mut stack = vec![command.to_string()];
        let config = GCodeParserConfig {
            enable_comments: true,
            enable_checksums: true,
            enable_infix: true,
            enable_macros: true,
            enable_vendor_extensions: true,
        };
        while let Some(line) = stack.pop() {
            let mut parser = GCodeParser::new(&line, config.clone());
            while let Some(cmd_result) = parser.next_command() {
                let span = match &cmd_result {
                    Ok(GCodeCommand::Macro { span, .. }) => Some(span.clone()),
                    Ok(GCodeCommand::Word { span, .. }) => Some(span.clone()),
                    Ok(GCodeCommand::Comment(_, span)) => Some(span.clone()),
                    Ok(GCodeCommand::VendorExtension { span, .. }) => Some(span.clone()),
                    Ok(GCodeCommand::Checksum { span, .. }) => Some(span.clone()),
                    Err(e) => Some(e.span.clone()),
                    _ => None,
                };
                match &cmd_result {
                    Ok(GCodeCommand::Macro { name, .. }) => {
                        let macros = self.macros.read().await;
                        if let Some(lines) = macros.get(*name) {
                            // Push macro lines to stack for expansion (reverse order for correct sequence)
                            for macro_line in lines.iter().rev() {
                                stack.push(macro_line.clone());
                            }
                        } else {
                            results.push(Err(GCodeError { message: format!("Macro '{}' not found", name), span: span.unwrap_or(crate::gcode::parser::GCodeSpan { range: 0..line.len() }) }));
                        }
                    }
                    Ok(cmd) => {
                        // Convert to owned, then leak to 'static
                        let owned: OwnedGCodeCommand = cmd.clone().into();
                        let static_cmd = match owned {
                            OwnedGCodeCommand::Word { letter, value, span } => GCodeCommand::Word { letter, value: Box::leak(value.into_boxed_str()), span },
                            OwnedGCodeCommand::Comment(comment, span) => GCodeCommand::Comment(Box::leak(comment.into_boxed_str()), span),
                            OwnedGCodeCommand::Macro { name, args, span } => GCodeCommand::Macro { name: Box::leak(name.into_boxed_str()), args: Box::leak(args.into_boxed_str()), span },
                            OwnedGCodeCommand::VendorExtension { name, args, span } => GCodeCommand::VendorExtension { name: Box::leak(name.into_boxed_str()), args: Box::leak(args.into_boxed_str()), span },
                            OwnedGCodeCommand::Checksum { command, checksum, span } => GCodeCommand::Checksum { command: Box::new(GCodeCommand::Word { letter: 'N', value: "0", span: span.clone() }), checksum, span },
                        };
                        results.push(Ok(static_cmd));
                    }
                    Err(e) => {
                        results.push(Err(e.clone()));
                    }
                }
            }
        }
        results
    }

    /// Integration point: Connect this to the print job and motion system
    /// Call this function from your print job or motion pipeline to execute a macro by name
    ///
    /// # Arguments
    /// * `name` - The name of the macro to execute.
    /// * `dispatch` - A closure that will be called for each parsed GCodeCommand.
    ///
    /// # Errors
    /// Returns an error if the macro is not found or expansion fails
    pub async fn execute_macro_and_dispatch<F>(&self, name: &str, mut dispatch: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(crate::gcode::parser::GCodeCommand<'static>) + Send,
    {
        let expanded = self.parse_and_expand_async(&format!("{{{}}}", name)).await;
        for cmd in expanded {
            if let Ok(cmd) = cmd {
                // Here, dispatch the command to the print job or motion system
                dispatch(cmd);
            }
        }
        Ok(())
    }

    /// Execute a macro and send each parsed command to the provided GCodeExecutor.
    pub async fn execute_macro_with_executor<E: crate::gcode::gcode_executor::GCodeExecutor + Send>(
        &self,
        name: &str,
        executor: &mut E,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let expanded = self.parse_and_expand_async(&format!("{{{}}}", name)).await;
        for cmd in expanded {
            if let Ok(cmd) = cmd {
                executor.execute(cmd);
            }
        }
        Ok(())
    }
}

// All test code moved to tests/gcode_macros.rs