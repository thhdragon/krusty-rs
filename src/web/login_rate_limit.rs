use axum::{middleware::Next, extract::{Request, State}, response::Response, http::StatusCode};
use std::net::IpAddr;
use std::str::FromStr;
use crate::web::api::AppState;
use axum::response::IntoResponse;

/// Middleware for rate limiting login attempts per IP.
pub async fn login_rate_limit_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract IP address from request (use remote_addr if available, else fallback)
    let ip = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|info| info.0.ip())
        .or_else(|| {
            req.headers()
                .get("x-forwarded-for")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| IpAddr::from_str(s).ok())
        })
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));

    if !state.rate_limiter.check_and_increment(ip).await {
        let body = axum::Json(serde_json::json!({ "error": "Too many login attempts" }));
        return Ok((StatusCode::TOO_MANY_REQUESTS, body).into_response());
    }
    Ok(next.run(req).await)
}
