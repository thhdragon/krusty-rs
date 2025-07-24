//! Defines the Axum API routes and handlers.

use crate::web::models::{GcodeCommandRequest, AuthRequest, AuthResponse, TokenCheckResponse};
use crate::web::printer_channel::PrinterRequest;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use axum::response::IntoResponse;
use serde_json::json;
use tokio::sync::mpsc::Sender;

use jsonwebtoken::{encode, decode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// For demo: simple in-memory user store and secret key
static SECRET_KEY: &[u8] = b"super_secret_key_change_me";

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

type UserStore = Arc<Mutex<HashMap<String, String>>>;

pub struct AppStateInner {
    pub printer_tx: Sender<PrinterRequest>,
    pub users: UserStore,
}
pub type AppState = Arc<AppStateInner>;

/// Creates the Axum router with all the API endpoints.
pub fn create_router(printer_tx: Sender<PrinterRequest>) -> Router {
    let users = Arc::new(Mutex::new(HashMap::from([
        ("admin".to_string(), "password".to_string()),
    ])));
    let state = Arc::new(AppStateInner { printer_tx, users });
    Router::new()
        .route("/api/v1/status", get(get_status))
        .route("/api/v1/gcode", post(execute_gcode))
        .route("/api/v1/pause", post(pause_handler))
        .route("/api/v1/resume", post(resume_handler))
        .route("/api/v1/cancel", post(cancel_handler))
        .route("/api/v1/auth/login", post(auth_login))
        .route("/api/v1/auth/check", get(auth_check))
        .with_state(state)
}

/// Handler to get the current status of the printer.
async fn get_status(State(state): State<AppState>) -> impl IntoResponse {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    if state.printer_tx.send(PrinterRequest::GetStatus { respond_to: resp_tx }).await.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to send status request" }))).into_response();
    }
    match resp_rx.await {
        Ok(status) => {
            // Compose a richer response (example structure)
            let response = json!({
                "state": status.status,
                "job": {
                    // Placeholder: add job details here if available
                },
                "printer": {
                    "position": status.position,
                    "hotend_temp": status.hotend_temp,
                    "target_hotend_temp": status.target_hotend_temp
                }
            });
            (StatusCode::OK, Json(response)).into_response()
        },
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Status response error" }))).into_response(),
    }
}

/// Handler to execute a single G-code command.
async fn execute_gcode(
    State(state): State<AppState>,
    Json(payload): Json<GcodeCommandRequest>,
) -> Result<StatusCode, StatusCode> {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    if state.printer_tx
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

/// Pause the current print job.
async fn pause_handler(State(state): State<AppState>) -> impl IntoResponse {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    if state.printer_tx.send(PrinterRequest::PauseJob { respond_to: resp_tx }).await.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to send pause request" }))).into_response();
    }
    match resp_rx.await {
        Ok(Ok(_)) => (StatusCode::OK, Json(json!({ "result": "ok" }))).into_response(),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Pause response error" }))).into_response(),
    }
}

/// Resume a paused print job.
async fn resume_handler(State(state): State<AppState>) -> impl IntoResponse {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    if state.printer_tx.send(PrinterRequest::ResumeJob { respond_to: resp_tx }).await.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to send resume request" }))).into_response();
    }
    match resp_rx.await {
        Ok(Ok(_)) => (StatusCode::OK, Json(json!({ "result": "ok" }))).into_response(),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Resume response error" }))).into_response(),
    }
}

/// Cancel the current print job.
async fn cancel_handler(State(state): State<AppState>) -> impl IntoResponse {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    if state.printer_tx.send(PrinterRequest::CancelJob { respond_to: resp_tx }).await.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to send cancel request" }))).into_response();
    }
    match resp_rx.await {
        Ok(Ok(_)) => (StatusCode::OK, Json(json!({ "result": "ok" }))).into_response(),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Cancel response error" }))).into_response(),
    }
}

/// POST /api/v1/auth/login
async fn auth_login(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> impl IntoResponse {
    let users = state.users.lock().unwrap();
    if let Some(stored_pw) = users.get(&payload.username) {
        if stored_pw == &payload.password {
            // Create JWT
            let expiration = chrono::Utc::now().timestamp() as usize + 3600; // 1 hour
            let claims = Claims { sub: payload.username.clone(), exp: expiration };
            let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(SECRET_KEY)).unwrap();
            return (StatusCode::OK, Json(AuthResponse { token })).into_response();
        }
    }
    (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid username or password" }))).into_response()
}

/// GET /api/v1/auth/check
async fn auth_check(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    let token = auth.token();
    let validation = Validation::new(Algorithm::HS256);
    let result = decode::<Claims>(token, &DecodingKey::from_secret(SECRET_KEY), &validation);
    let valid = result.is_ok();
    (StatusCode::OK, Json(TokenCheckResponse { valid })).into_response()
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