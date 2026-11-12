//! HTTP security middleware (docs/SECURITY.md). Loopback mode relies on these instead of
//! passwords (Q-005, answered): `Host` allowlist against DNS rebinding, `Origin` check
//! against cross-site requests (CSRF), strict headers against framing and injection.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, HOST, ORIGIN, REFERER, REFERRER_POLICY,
    X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::http::{HeaderValue, Method, StatusCode};
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

/// Rejects state-changing requests (any method but GET, HEAD and OPTIONS) unless their
/// `Origin` — or, when absent, `Referer` — is the dashboard itself: a cross-site page cannot
/// make the owner's browser change anything (CSRF). Forms must still use POST + a CSRF
/// token when sessions arrive (docs/recipes/add-dashboard-page.md).
pub async fn check_origin(
    State(allowed): State<Arc<Vec<String>>>,
    request: Request,
    next: Next,
) -> Response {
    if matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) {
        return next.run(request).await;
    }
    let headers = request.headers();
    let source = headers
        .get(ORIGIN)
        .or_else(|| headers.get(REFERER))
        .and_then(|value| value.to_str().ok());
    let trusted = source.is_some_and(|value| allowed.iter().any(|host| same_origin(value, host)));
    if trusted {
        next.run(request).await
    } else {
        (StatusCode::FORBIDDEN, "cross-origin request blocked").into_response()
    }
}

/// `http://<host>` (Origin) or `http://<host>/...` (Referer) for an allowlisted host.
fn same_origin(value: &str, host: &str) -> bool {
    value
        .strip_prefix("http://")
        .and_then(|rest| rest.strip_prefix(host))
        .is_some_and(|tail| tail.is_empty() || tail.starts_with('/'))
}

/// Adds security headers to every response; pages are never cached (they hold findings).
/// `Referrer-Policy: same-origin` keeps URLs private while letting browsers send a real
/// `Origin` on same-origin POSTs (with `no-referrer` they send `Origin: null`).
pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(CONTENT_SECURITY_POLICY, HeaderValue::from_static(CSP));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("same-origin"));
    headers
        .entry(CACHE_CONTROL)
        .or_insert(HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_origin_matches_exact_host_only() {
        assert!(same_origin("http://127.0.0.1:7878", "127.0.0.1:7878"));
        assert!(same_origin("http://127.0.0.1:7878/apps", "127.0.0.1:7878"));
        assert!(!same_origin("http://127.0.0.1:78789", "127.0.0.1:7878"));
        assert!(!same_origin("https://evil.test", "127.0.0.1:7878"));
        assert!(!same_origin("null", "127.0.0.1:7878"));
    }
}
