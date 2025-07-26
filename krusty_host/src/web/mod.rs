//! The web module for handling the Axum API.
//! This file declares the other files in this directory as sub-modules.

pub mod api;
pub mod auth;
pub mod models;
pub mod printer_channel;
pub mod token_blacklist;
pub mod rate_limiter;
pub mod login_rate_limit;

use std::sync::Arc;
use tokio::sync::RwLock;

// Placeholder for the shared state type; replace with actual type as needed
pub struct WebInterface {
    pub state: Arc<RwLock<crate::host_os::PrinterState>>, // Adjusted type
}

impl WebInterface {
    pub fn new(state: Arc<RwLock<crate::host_os::PrinterState>>) -> Self {
        Self { state }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}