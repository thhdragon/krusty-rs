/// G-code Macro System: Clean, idiomatic, async, and testable foundation.
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Custom error type for macro operations.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MacroError {
    #[error("Macro '{0}' not found")] 
    NotFound(String),
    #[error("Macro recursion detected: {0}")]
    Recursion(String),
    #[error("Invalid macro definition: {0}")]
    InvalidDefinition(String),
    #[error("Other macro error: {0}")]
    Other(String),
}


/// Main macro processor: stores, expands, and manages macros.
#[derive(Debug, Clone)]
pub struct MacroProcessor {
    macros: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl MacroProcessor {
    /// Create a new, empty macro processor.
    pub fn new() -> Self {
        Self {
            macros: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Define or update a macro.
    pub async fn define_macro(&self, name: &str, commands: Vec<String>) -> Result<(), MacroError> {
        let mut macros = self.macros.write().await;
        macros.insert(name.to_string(), commands);
        Ok(())
    }

    /// Delete a macro by name.
    pub async fn delete_macro(&self, name: &str) -> Result<(), MacroError> {
        let mut macros = self.macros.write().await;
        if macros.remove(name).is_some() {
            Ok(())
        } else {
            Err(MacroError::NotFound(name.to_string()))
        }
    }

    /// List all defined macro names.
    pub async fn list_macros(&self) -> Vec<String> {
        let macros = self.macros.read().await;
        macros.keys().cloned().collect()
    }

    /// Expand a macro by name, with cycle detection (stub).
    pub async fn expand_macro(&self, name: &str, call_stack: &[String]) -> Result<Vec<String>, MacroError> {
        let macros = self.macros.read().await;
        if call_stack.contains(&name.to_string()) {
            return Err(MacroError::Recursion(name.to_string()));
        }
        macros.get(name)
            .cloned()
            .ok_or_else(|| MacroError::NotFound(name.to_string()))
    }
}

// Implement MacroExpander for MacroProcessor with correct signature for parser
#[async_trait::async_trait]
impl crate::gcode::parser::MacroExpander for MacroProcessor {
    async fn expand(&self, name: String, _args: String) -> Option<Vec<crate::gcode::parser::OwnedGCodeCommand>> {
        let macros = self.macros.read().await;
        let lines = macros.get(&name)?.clone();
        drop(macros); // Release lock before recursion
        let mut commands = Vec::new();
        let config = crate::gcode::parser::GCodeParserConfig::default();
        for line in lines {
            let mut parser = crate::gcode::parser::GCodeParser::new(&line, config.clone());
            while let Some(cmd_result) = parser.next_command() {
                match cmd_result {
                    Ok(crate::gcode::parser::GCodeCommand::Macro { name: nested_name, args: nested_args, .. }) => {
                        // Recursively expand nested macro
                        if let Some(nested_cmds) = self.expand(nested_name.to_string(), nested_args.to_string()).await {
                            commands.extend(nested_cmds);
                        } else {
                            // If macro not found, skip or optionally yield error
                        }
                    }
                    Ok(cmd) => {
                        commands.push(crate::gcode::parser::OwnedGCodeCommand::from(cmd));
                    }
                    Err(_) => {
                        // Optionally handle parse errors in macro body
                    }
                }
            }
        }
        Some(commands)
    }
}

// Unit tests will be added below.
use crate::gcode::parser::{GCodeParser, GCodeParserConfig, GCodeCommand, GCodeError, OwnedGCodeCommand};

impl MacroProcessor {
    /// Parse a command string, expanding macros recursively, returning owned commands or errors.
    pub async fn parse_and_expand_async_owned(&self, command: &str) -> Vec<Result<OwnedGCodeCommand, GCodeError>> {
        let mut results = Vec::new();
        let mut stack = vec![(command.to_string(), Vec::new())]; // (line, macro call stack)
        let config = GCodeParserConfig::default();
        while let Some((line, call_stack)) = stack.pop() {
            let line_len = line.len();
            let line_ref = line.as_str();
            let mut parser = GCodeParser::new(line_ref, config.clone());
            let mut should_break = false;
            while let Some(cmd_result) = parser.next_command() {
                let span = match &cmd_result {
                    Ok(GCodeCommand::Macro { span, .. }) => Some(span.clone()),
                    Ok(GCodeCommand::Word { span, .. }) => Some(span.clone()),
                    Ok(GCodeCommand::Comment(_, span)) => Some(span.clone()),
                    Ok(GCodeCommand::VendorExtension { span, .. }) => Some(span.clone()),
                    Ok(GCodeCommand::Checksum { span, .. }) => Some(span.clone()),
                    Err(e) => Some(e.span.clone()),
                };
                match &cmd_result {
                    Ok(GCodeCommand::Macro { name, .. }) => {
                        // Cycle detection: if macro is already in call_stack, report error
                        if call_stack.contains(&name.to_string()) {
                            results.push(Err(GCodeError {
                                message: format!("Macro recursion detected: '{}' is already in call stack: {:?}", name, call_stack),
                                span: span.clone().unwrap_or(crate::gcode::parser::GCodeSpan { range: 0..line_len }),
                            }));
                            should_break = true;
                        } else {
                            // Minimize lock scope: only lock for lookup
                            let macro_lines = {
                                let macros = self.macros.read().await;
                                macros.get(&name.to_string()).cloned()
                            };
                            if let Some(lines) = macro_lines {
                                // Push macro lines to stack for expansion (reverse order for correct sequence)
                                let mut new_call_stack = call_stack.clone();
                                new_call_stack.push(name.to_string());
                                for macro_line in lines.iter().rev() {
                                    stack.push((macro_line.clone(), new_call_stack.clone()));
                                }
                            } else {
                                results.push(Err(GCodeError {
                                    message: format!("Macro '{}' not found", name),
                                    span: span.unwrap_or(crate::gcode::parser::GCodeSpan { range: 0..line_len }),
                                }));
                            }
                        }
                    },
                    Ok(cmd) => {
                        // Convert to owned, do not leak
                        results.push(Ok(OwnedGCodeCommand::from(cmd.clone())));
                    },
                    Err(e) => {
                        results.push(Err(e.clone()));
                    },
                }
                if should_break {
                    break;
                }
            }
        }
        results
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gcode::parser::MacroExpander;
    use tokio::runtime::Runtime;

    fn rt() -> Runtime {
        Runtime::new().unwrap()
    }

    #[test]
    fn test_define_and_list_macros() {
        rt().block_on(async {
            let proc = MacroProcessor::new();
            assert_eq!(proc.list_macros().await.len(), 0);
            proc.define_macro("foo", vec!["G1 X1".into()]).await.unwrap();
            proc.define_macro("bar", vec!["G28".into()]).await.unwrap();
            let mut names = proc.list_macros().await;
            names.sort();
            assert_eq!(names, vec!["bar", "foo"]);
        });
    }

    #[test]
    fn test_delete_macro() {
        rt().block_on(async {
            let proc = MacroProcessor::new();
            proc.define_macro("foo", vec!["G1 X1".into()]).await.unwrap();
            assert!(proc.delete_macro("foo").await.is_ok());
            assert!(matches!(proc.delete_macro("foo").await, Err(MacroError::NotFound(_))));
        });
    }

    #[test]
    fn test_expand_macro_success() {
        rt().block_on(async {
            let proc = MacroProcessor::new();
            proc.define_macro("foo", vec!["G1 X1".into(), "G1 X2".into()]).await.unwrap();
            let expanded = proc.expand_macro("foo", &[]).await.unwrap();
            assert_eq!(expanded, vec!["G1 X1", "G1 X2"]);
        });
    }

    #[test]
    fn test_expand_macro_not_found() {
        rt().block_on(async {
            let proc = MacroProcessor::new();
            let err = proc.expand_macro("nope", &[]).await.unwrap_err();
            assert!(matches!(err, MacroError::NotFound(_)));
        });
    }

    #[test]
    fn test_expand_macro_cycle_detection() {
        rt().block_on(async {
            let proc = MacroProcessor::new();
            proc.define_macro("foo", vec!["G1 X1".into()]).await.unwrap();
            let err = proc.expand_macro("foo", &["foo".into()]).await.unwrap_err();
            assert!(matches!(err, MacroError::Recursion(_)));
        });
    }

    fn parse_gcode(s: &str) -> OwnedGCodeCommand {
        let mut parser = GCodeParser::new(s, GCodeParserConfig::default());
        parser.next_command().unwrap().unwrap().into()
    }

    #[test]
    fn test_macro_expander_trait() {
        rt().block_on(async {
            let proc = MacroProcessor::new();
            proc.define_macro("foo", vec!["G1 X1".into()]).await.unwrap();
            let expanded = crate::gcode::parser::MacroExpander::expand(&proc, "foo".to_string(), "".to_string()).await;
            assert_eq!(expanded, Some(vec![parse_gcode("G1 X1")]));
            let none = crate::gcode::parser::MacroExpander::expand(&proc, "bar".to_string(), "".to_string()).await;
            assert_eq!(none, None);
        });
    }
}
