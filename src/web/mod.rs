//! The web module for handling the Axum API.
//! This file declares the other files in this directory as sub-modules.

pub mod api;
pub mod models;
pub mod printer_channel;

use std::sync::Arc;
use tokio::sync::RwLock;

// Placeholder for the shared state type; replace with actual type as needed
pub struct WebInterface {
    pub state: Arc<RwLock<crate::printer::PrinterState>>, // Adjust type if needed
}

impl WebInterface {
    pub fn new(state: Arc<RwLock<crate::printer::PrinterState>>) -> Self {
        Self { state }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}