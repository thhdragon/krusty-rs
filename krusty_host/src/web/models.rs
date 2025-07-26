//! Contains the data models for API requests and responses.

use serde::{Deserialize, Serialize};

/// Represents the response for the /api/v1/status endpoint.
#[derive(Serialize, Debug)]
pub struct PrinterStatusResponse {
    /// Printer state: "printing", "paused", "idle", or "error".
    pub state: String,
    /// Job details (can be None if no job is active).
    pub job: Option<JobStatus>,
    /// Printer details (position, temps, etc).
    pub printer: PrinterDetails,
}

/// Represents job status details.
#[derive(Serialize, Debug)]
pub struct JobStatus {
    // Extend with job fields as needed (e.g., file, progress, elapsed, etc)
    pub id: Option<String>,
    pub progress: Option<f32>,
    pub file: Option<String>,
}

/// Represents printer details (position, temps, etc).
#[derive(Serialize, Debug)]
pub struct PrinterDetails {
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