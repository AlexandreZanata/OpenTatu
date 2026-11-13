//! Actual HTTP transport guard tests with synthetic local credentials; no external service.
use super::*;
use axum::{
    Json, Router,
    response::Redirect,
    routing::{get, post},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::test]
async fn r16_login_redirect_cannot_forward_credentials_to_another_origin() {
    use std::os::unix::fs::PermissionsExt as _;
    let received = Arc::new(AtomicUsize::new(0));
    let capture = received.clone();
    let recipient = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = format!("http://{}/capture", recipient.local_addr().unwrap());
    let receiver = tokio::spawn(async move {
        let r = Router::new().route(
            "/capture",
            post(move || {
                capture.fetch_add(1, Ordering::SeqCst);
                async { Json(serde_json::json!({"token":"synthetic"})) }
            }),
        );
        let _ = axum::serve(recipient, r).await;
    });
    let sent = Arc::new(AtomicUsize::new(0));
    let count = sent.clone();
    let sender = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", sender.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let r = Router::new().route(
            "/v1/watchers/login",
            post(move || {
                count.fetch_add(1, Ordering::SeqCst);
                let location = target.clone();
                async move { Redirect::temporary(&location) }
            }),
        );
        let _ = axum::serve(sender, r).await;
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.json");
    std::fs::write(&path,r#"{"machine_id":"fixture","password":"SYNTHETIC_PRIVATE","bouncer_key":"SYNTHETIC_BOUNCER"}"#).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let mut p = crate::crowdsec_config::tests::profile();
    p.base_url = base;
    p.credentials_file = path;
    assert!(matches!(Api::connect(&p).await, Err(F::Transport)));
    assert_eq!(sent.load(Ordering::SeqCst), 1);
    assert_eq!(received.load(Ordering::SeqCst), 0);
    server.abort();
    receiver.abort();
}
#[tokio::test]
async fn r16_sensor_body_is_bounded_and_api_key_headers_are_sensitive() {
    use std::os::unix::fs::PermissionsExt as _;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let r = Router::new()
            .route(
                "/v1/watchers/login",
                post(|headers: axum::http::HeaderMap| async move {
                    if headers
                        .get(reqwest::header::USER_AGENT)
                        .and_then(|h| h.to_str().ok())
                        != Some(concat!("opentatu/", env!("CARGO_PKG_VERSION")))
                    {
                        return Err(axum::http::StatusCode::UNAUTHORIZED);
                    }
                    Ok(Json(serde_json::json!({"token":"SYNTHETIC_JWT"})))
                }),
            )
            .route(
                "/v1/alerts",
                get(|| async { "x".repeat(crate::crowdsec::API_BYTES + 1) }),
            );
        let _ = axum::serve(listener, r).await;
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.json");
    std::fs::write(&path,r#"{"machine_id":"fixture","password":"SYNTHETIC_PRIVATE","bouncer_key":"SYNTHETIC_BOUNCER"}"#).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let mut p = crate::crowdsec_config::tests::profile();
    p.base_url = base;
    p.credentials_file = path;
    let api = Api::connect(&p).await.unwrap();
    let request = api
        .request(reqwest::Method::GET, "v1/decisions", true)
        .unwrap()
        .build()
        .unwrap();
    assert!(request.headers()["X-Api-Key"].is_sensitive());
    assert!(!format!("{:?}", request.headers()).contains("SYNTHETIC_BOUNCER"));
    assert_eq!(api.get("v1/alerts", &[], false).await, Err(F::Saturated));
    server.abort();
}
