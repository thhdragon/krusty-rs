// src/web.rs - Simple web API
use axum::{
    routing::{get, post},
    Router, Json, Extension,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::printer::Printer;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebError {
    #[error("Axum error: {0}")]
    Axum(#[from] axum::Error),
    #[error("Printer error: {0}")]
    Printer(#[from] crate::printer::PrinterError),
    #[error("Other: {0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionRequest {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub e: Option<f64>,
    pub feedrate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureRequest {
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterStatus {
    pub ready: bool,
    pub position: [f64; 3],
    pub temperature: f64,
    pub print_progress: f64,
}

pub struct WebServer {
    printer: Arc<RwLock<Printer>>,
}

impl WebServer {
    pub fn new(printer: Arc<RwLock<Printer>>) -> Self {
        Self { printer }
    }

    pub async fn start(&self, port: u16) -> Result<(), WebError> {
        let app = Router::new()
            .route("/", get(root))
            .route("/status", get(get_status))
            .route("/move", post(move_to_position))
            .route("/home", post(home_axes))
            .route("/temperature/hotend", post(set_hotend_temperature))
            .route("/temperature/bed", post(set_bed_temperature))
            .layer(Extension(self.printer.clone()));

        let addr = format!("0.0.0.0:{}", port).parse()?;
        tracing::info!("Starting web server on http://{}", addr);
        
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;
        
        Ok(())
    }
}

async fn root() -> &'static str {
    "Krusty-RS 3D Printer API"
}

async fn get_status(
    Extension(printer): Extension<Arc<RwLock<Printer>>>,
) -> Result<Json<PrinterStatus>, (axum::http::StatusCode, String)> {
    let printer_guard = printer.read().await;
    let state = printer_guard.get_state().await;
    
    let status = PrinterStatus {
        ready: state.ready,
        position: state.position,
        temperature: state.temperature,
        print_progress: state.print_progress,
    };
    
    Ok(Json(status))
}

async fn move_to_position(
    Extension(printer): Extension<Arc<RwLock<Printer>>>,
    Json(request): Json<PositionRequest>,
) -> Result<Json<&'static str>, (axum::http::StatusCode, String)> {
    let mut printer_guard = printer.write().await;
    
    // Get current position
    let current_pos = printer_guard.get_current_position();
    
    let target_x = request.x.unwrap_or(current_pos[0]);
    let target_y = request.y.unwrap_or(current_pos[1]);
    let target_z = request.z.unwrap_or(current_pos[2]);
    let target_e = request.e.unwrap_or(current_pos[3]);
    let feedrate = request.feedrate;
    
    // Move to target position
    match printer_guard.queue_linear_move(
        [target_x, target_y, target_z],
        feedrate,
        Some(target_e - current_pos[3]), // Relative E move
    ).await {
        Ok(()) => Ok(Json("Move queued successfully")),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn home_axes(
    Extension(printer): Extension<Arc<RwLock<Printer>>>,
) -> Result<Json<&'static str>, (axum::http::StatusCode, String)> {
    let mut printer_guard = printer.write().await;
    
    match printer_guard.queue_home().await {
        Ok(()) => Ok(Json("Homing queued successfully")),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn set_hotend_temperature(
    Extension(printer): Extension<Arc<RwLock<Printer>>>,
    Json(request): Json<TemperatureRequest>,
) -> Result<Json<&'static str>, (axum::http::StatusCode, String)> {
    let printer_guard = printer.read().await;
    
    match printer_guard.set_hotend_temperature(request.temperature).await {
        Ok(()) => Ok(Json("Hotend temperature set")),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn set_bed_temperature(
    Extension(printer): Extension<Arc<RwLock<Printer>>>,
    Json(request): Json<TemperatureRequest>,
) -> Result<Json<&'static str>, (axum::http::StatusCode, String)> {
    let printer_guard = printer.read().await;
    
    match printer_guard.set_bed_temperature(request.temperature).await {
        Ok(()) => Ok(Json("Bed temperature set")),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}