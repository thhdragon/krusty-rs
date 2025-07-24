use axum::{body::{Body, to_bytes}, http::{Request, StatusCode}};
use krusty_rs::web::api::app;
use serde_json::json;
use tower::ServiceExt; // for .oneshot()

#[tokio::test]
async fn test_login_rate_limit() {
    let app = app();
    let client_ip = "127.0.0.1";
    let login_body = json!({"username": "admin", "password": "wrong"});
    let req = || {
        Request::builder()
            .method("POST")
            .uri("/api/v1/auth/login")
            .header("content-type", "application/json")
            .header("x-forwarded-for", client_ip)
            .body(Body::from(login_body.to_string()))
            .unwrap()
    };
    // 5 allowed attempts
    for _ in 0..5 {
        let response = app.clone().oneshot(req()).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    // 6th attempt should be rate limited
    let response = app.clone().oneshot(req()).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "Too many login attempts");
}

#[tokio::test]
async fn test_login_rate_limit_reset() {
    let app = app();
    let client_ip = "127.0.0.2";
    let login_body = json!({"username": "admin", "password": "wrong"});
    let req = || {
        Request::builder()
            .method("POST")
            .uri("/api/v1/auth/login")
            .header("content-type", "application/json")
            .header("x-forwarded-for", client_ip)
            .body(Body::from(login_body.to_string()))
            .unwrap()
    };
    // 5 allowed attempts
    for _ in 0..5 {
        let response = app.clone().oneshot(req()).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    // 6th attempt should be rate limited
    let response = app.clone().oneshot(req()).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    // Wait for window to expire (60s)
    tokio::time::sleep(std::time::Duration::from_secs(61)).await;
    // Should be allowed again
    let response = app.clone().oneshot(req()).await.unwrap();
    // Still unauthorized, but not rate limited
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
