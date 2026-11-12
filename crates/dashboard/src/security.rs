//! HTTP security middleware (docs/SECURITY.md).

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, HOST, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS,
    X_FRAME_OPTIONS,
};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// No scripts, no third-party resources, no framing.
const CSP: &str = "default-src 'none'; style-src 'self'; img-src 'self'; form-action 'self'; \
                   frame-ancestors 'none'; base-uri 'none'";

/// Rejects requests whose `Host` is not allowlisted (blocks DNS rebinding against the
/// local dashboard).
pub async fn check_host(
    State(allowed): State<Arc<Vec<String>>>,
    request: Request,
    next: Next,
) -> Response {
    let host = request
        .headers()
        .get(HOST)
        .and_then(|value| value.to_str().ok());
    match host {
        Some(host) if allowed.iter().any(|candidate| candidate == host) => next.run(request).await,
        _ => (StatusCode::MISDIRECTED_REQUEST, "host not allowed").into_response(),
    }
}

/// Adds security headers to every response; pages are never cached (they hold findings).
pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(CONTENT_SECURITY_POLICY, HeaderValue::from_static(CSP));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers
        .entry(CACHE_CONTROL)
        .or_insert(HeaderValue::from_static("no-store"));
    response
}
