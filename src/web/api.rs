//! Defines the Axum API routes and handlers.

use crate::web::models::{GcodeCommandRequest, PrinterStatusResponse};
use crate::web::printer_channel::PrinterRequest;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use tokio::sync::mpsc::Sender;

pub type AppState = Sender<PrinterRequest>;

/// Creates the Axum router with all the API endpoints.
pub fn create_router(printer_tx: AppState) -> Router {
    Router::new()
        .route("/api/v1/status", get(get_status))
        .route("/api/v1/gcode", post(execute_gcode))
        .with_state(printer_tx)
}

/// Handler to get the current status of the printer.
async fn get_status(State(printer_tx): State<AppState>) -> Result<Json<PrinterStatusResponse>, StatusCode> {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    if printer_tx.send(PrinterRequest::GetStatus { respond_to: resp_tx }).await.is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    match resp_rx.await {
        Ok(status) => Ok(Json(status)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Handler to execute a single G-code command.
async fn execute_gcode(
    State(printer_tx): State<AppState>,
    Json(payload): Json<GcodeCommandRequest>,
) -> Result<StatusCode, StatusCode> {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    if printer_tx
        .send(PrinterRequest::ExecuteGcode {
            command: payload.command,
            respond_to: resp_tx,
        })
        .await
        .is_err()
    {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    match resp_rx.await {
        Ok(Ok(_)) => Ok(StatusCode::OK),
        Ok(Err(_)) | Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Placeholder for advanced API endpoints
async fn advanced_status_handler() -> String {
    // TODO: Implement advanced status endpoint (authentication, streaming, etc.)
    "Advanced status endpoint not implemented".to_string()
}

async fn printer_control_handler() -> String {
    // TODO: Implement advanced printer control endpoint
    "Printer control endpoint not implemented".to_string()
}

pub fn advanced_api_router() -> Router {
    Router::new()
        .route("/api/v1/advanced_status", get(advanced_status_handler))
        .route("/api/v1/printer_control", get(printer_control_handler))
}

// TODO: Integrate advanced_api_router into main Axum app when implemented.