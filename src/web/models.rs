//! Contains the data models for API requests and responses.

use serde::{Deserialize, Serialize};

/// Represents the current status of the printer.
#[derive(Serialize, Debug)]
pub struct PrinterStatusResponse {
    pub status: String,
    pub position: (f32, f32, f32),
    pub hotend_temp: f32,
    pub target_hotend_temp: f32,
}

/// Represents a login request.
#[derive(Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

/// Represents a login response with JWT token.
#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
}

/// Represents a token validation response.
#[derive(Serialize)]
pub struct TokenCheckResponse {
    pub valid: bool,
}

/// Represents a request to execute a G-code command.
#[derive(Deserialize)]
pub struct GcodeCommandRequest {
    pub command: String,
}