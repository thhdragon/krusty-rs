// src/gcode/macro_processor.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
}