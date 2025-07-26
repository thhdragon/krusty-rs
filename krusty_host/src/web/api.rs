/// For tests: create a router with a persistent user store
pub fn app_with_state(state: AppState) -> axum::Router {
    create_router_with_state(state)
}
/// Defines the Axum API routes and handlers.

use crate::web::models::{GcodeCommandRequest, AuthRequest, AuthResponse, TokenCheckResponse};
use crate::web::printer_channel::PrinterRequest;
use krusty_shared::{AuthBackend, InMemoryAuthBackend};
use crate::web::token_blacklist::TokenBlacklist;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
    response::IntoResponse,
};
use crate::web::login_rate_limit::login_rate_limit_middleware;
use axum_extra::extract::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use tokio::sync::mpsc::Sender;

use jsonwebtoken::{encode, decode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use std::env;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Helper to create a JSON error response with a message and status code
fn json_error(message: &str, status: StatusCode) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}


/// Loads the JWT secret from the environment or uses a default for development.
fn jwt_secret() -> Vec<u8> {
    env::var("KRUSTY_JWT_SECRET")
        .map(|s| s.into_bytes())
        .unwrap_or_else(|_| b"super_secret_key_change_me".to_vec())
}

/// Loads the JWT expiration (seconds) from the environment or uses 3600 (1 hour) as default.
fn jwt_expiration() -> usize {
    env::var("KRUSTY_JWT_EXPIRATION")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600)
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub struct AppStateInner {
    pub printer_tx: Sender<PrinterRequest>,
    pub auth_backend: Box<dyn AuthBackend>,
    pub token_blacklist: TokenBlacklist,
    pub rate_limiter: crate::web::rate_limiter::RateLimiter,
}
pub type AppState = Arc<AppStateInner>;

/// Creates the Axum router with all the API endpoints.
pub fn create_router(printer_tx: Sender<PrinterRequest>) -> Router {
    let users = Arc::new(Mutex::new(HashMap::from([
        ("admin".to_string(), "password".to_string()),
    ])));
    let auth_backend = Box::new(InMemoryAuthBackend::new(users.clone()));
    let token_blacklist = TokenBlacklist::new();
    let rate_limiter = crate::web::rate_limiter::RateLimiter::new(5, std::time::Duration::from_secs(60));
    let state = Arc::new(AppStateInner { printer_tx, auth_backend, token_blacklist, rate_limiter });
    Router::new()
        .route("/api/v1/status", get(get_status))
        .route("/api/v1/gcode", post(execute_gcode))
        .route("/api/v1/pause", post(pause_handler))
        .route("/api/v1/resume", post(resume_handler))
        .route("/api/v1/cancel", post(cancel_handler))
        .route(
            "/api/v1/auth/login",
            post(auth_login).route_layer(axum::middleware::from_fn_with_state(state.clone(), login_rate_limit_middleware)),
        )
        .route("/api/v1/auth/check", get(auth_check))
        .route("/api/v1/auth/logout", post(auth_logout))
        .with_state(state)
}

pub fn create_router_with_state(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/status", get(get_status))
        .route("/api/v1/gcode", post(execute_gcode))
        .route("/api/v1/pause", post(pause_handler))
        .route("/api/v1/resume", post(resume_handler))
        .route("/api/v1/cancel", post(cancel_handler))
        .route(
            "/api/v1/auth/login",
            post(auth_login).route_layer(axum::middleware::from_fn_with_state(state.clone(), login_rate_limit_middleware)),
        )
        .route("/api/v1/auth/check", get(auth_check))
        .route("/api/v1/auth/logout", post(auth_logout))
        .with_state(state)
}

pub fn app() -> axum::Router {
    // Provide a dummy channel for tests
    let (printer_tx, _printer_rx) = tokio::sync::mpsc::channel(8);
    create_router(printer_tx)
}

/// Handler to get the current status of the printer.
async fn get_status(State(state): State<AppState>) -> axum::response::Response {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    if state.printer_tx.send(PrinterRequest::GetStatus { respond_to: resp_tx }).await.is_err() {
        return json_error("Internal error", StatusCode::INTERNAL_SERVER_ERROR);
    }
    match resp_rx.await {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(_) => json_error("Internal error", StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Handler to execute a single G-code command.
async fn execute_gcode(
    State(state): State<AppState>,
    Json(payload): Json<GcodeCommandRequest>,
) -> axum::response::Response {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    if state.printer_tx
        .send(PrinterRequest::ExecuteGcode {
            command: payload.command,
            respond_to: resp_tx,
        })
        .await
        .is_err()
    {
        return json_error("Internal error", StatusCode::INTERNAL_SERVER_ERROR);
    }
    match resp_rx.await {
        Ok(Ok(_)) => (StatusCode::OK, Json(serde_json::json!({ "result": "ok" }))).into_response(),
        Ok(Err(e)) => json_error(&e, StatusCode::CONFLICT),
        Err(_) => json_error("Internal error", StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Pause the current print job.
async fn pause_handler(State(state): State<AppState>) -> axum::response::Response {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    if state.printer_tx.send(PrinterRequest::PauseJob { respond_to: resp_tx }).await.is_err() {
        return json_error("Internal error", StatusCode::INTERNAL_SERVER_ERROR);
    }
    match resp_rx.await {
        Ok(Ok(_)) => (StatusCode::OK, Json(serde_json::json!({ "result": "ok" }))).into_response(),
        Ok(Err(e)) => json_error(&e, StatusCode::CONFLICT),
        Err(_) => json_error("Internal error", StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Resume a paused print job.
async fn resume_handler(State(state): State<AppState>) -> axum::response::Response {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    if state.printer_tx.send(PrinterRequest::ResumeJob { respond_to: resp_tx }).await.is_err() {
        return json_error("Internal error", StatusCode::INTERNAL_SERVER_ERROR);
    }
    match resp_rx.await {
        Ok(Ok(_)) => (StatusCode::OK, Json(serde_json::json!({ "result": "ok" }))).into_response(),
        Ok(Err(e)) => json_error(&e, StatusCode::CONFLICT),
        Err(_) => json_error("Internal error", StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Cancel the current print job.
async fn cancel_handler(State(state): State<AppState>) -> axum::response::Response {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
    if state.printer_tx.send(PrinterRequest::CancelJob { respond_to: resp_tx }).await.is_err() {
        return json_error("Internal error", StatusCode::INTERNAL_SERVER_ERROR);
    }
    match resp_rx.await {
        Ok(Ok(_)) => (StatusCode::OK, Json(serde_json::json!({ "result": "ok" }))).into_response(),
        Ok(Err(e)) => json_error(&e, StatusCode::CONFLICT),
        Err(_) => json_error("Internal error", StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/v1/auth/login
async fn auth_login(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> axum::response::Response {
    if state.auth_backend.validate(&payload.username, &payload.password).await {
        // Create JWT
        let expiration = chrono::Utc::now().timestamp() as usize + jwt_expiration();
        let claims = Claims { sub: payload.username.clone(), exp: expiration };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(&jwt_secret())).unwrap();
        return (StatusCode::OK, Json(AuthResponse { token })).into_response();
    }
    json_error("Invalid credentials", StatusCode::UNAUTHORIZED)
}

/// GET /api/v1/auth/check
async fn auth_check(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> axum::response::Response {
    let token = auth.token();
    // Check blacklist first
    if state.token_blacklist.contains(token) {
        return json_error("Invalid token", StatusCode::UNAUTHORIZED);
    }
    let validation = Validation::new(Algorithm::HS256);
    let result = decode::<Claims>(token, &DecodingKey::from_secret(&jwt_secret()), &validation);
    match result {
        Ok(_) => (StatusCode::OK, Json(TokenCheckResponse { valid: true })).into_response(),
        Err(_) => json_error("Invalid token", StatusCode::UNAUTHORIZED),
    }
}

/// POST /api/v1/auth/logout -- blacklist the current token
async fn auth_logout(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> axum::response::Response {
    let token = auth.token().to_string();
    state.token_blacklist.insert(token);
    (StatusCode::OK, Json(serde_json::json!({ "result": "ok" }))).into_response()
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