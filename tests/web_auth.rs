//! Integration tests for web API authentication endpoints

use axum::body::Body;
use axum::http::{Request, StatusCode};
use krusty_rs::web::api::app_with_state;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use krusty_rs::web::auth::InMemoryAuthBackend;

fn test_state() -> krusty_rs::web::api::AppState {
    let users = Arc::new(Mutex::new(HashMap::from([
        ("admin".to_string(), "password".to_string()),
    ])));
    let (printer_tx, _printer_rx) = tokio::sync::mpsc::channel(8);
    let auth_backend = Box::new(InMemoryAuthBackend::new(users));
    let token_blacklist = krusty_rs::web::token_blacklist::TokenBlacklist::new();
    let rate_limiter = krusty_rs::web::rate_limiter::RateLimiter::new(5, std::time::Duration::from_secs(60));
    Arc::new(krusty_rs::web::api::AppStateInner {
        printer_tx,
        auth_backend,
        token_blacklist,
        rate_limiter,
    })
}
use serde_json::json;
use tower::util::ServiceExt; // for `oneshot`
use http_body_util::BodyExt; // for .collect().await

#[tokio::test]
async fn test_auth_login_success() {
    let app = app_with_state(test_state());
    let payload = json!({
        "username": "admin",
        "password": "password"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("token").is_some());
}

#[tokio::test]
async fn test_auth_login_failure() {
    let app = app_with_state(test_state());
    let payload = json!({
        "username": "admin",
        "password": "wrongpassword"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_check_valid_token() {
    let app = app_with_state(test_state());
    // First, login to get a token
    let payload = json!({
        "username": "admin",
        "password": "password"
    });
    let login_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let login_response = app.clone().oneshot(login_request).await.unwrap();
    let login_body = login_response.into_body().collect().await.unwrap().to_bytes();
    let login_json: serde_json::Value = serde_json::from_slice(&login_body).unwrap();
    let token = login_json.get("token").unwrap().as_str().unwrap();

    // Now, check the token
    let check_request = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/check")
        .header("Authorization", &format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let check_response = app.oneshot(check_request).await.unwrap();
    assert_eq!(check_response.status(), StatusCode::OK);
    let check_body = check_response.into_body().collect().await.unwrap().to_bytes();
    let check_json: serde_json::Value = serde_json::from_slice(&check_body).unwrap();
    assert_eq!(check_json.get("valid"), Some(&serde_json::Value::Bool(true)));
}

#[tokio::test]
async fn test_auth_check_invalid_token() {
    let app = app_with_state(test_state());
    let check_request = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/check")
        .header("Authorization", "Bearer invalidtoken")
        .body(Body::empty())
        .unwrap();
    let check_response = app.oneshot(check_request).await.unwrap();
    assert_eq!(check_response.status(), StatusCode::UNAUTHORIZED);
}
