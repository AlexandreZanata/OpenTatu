//! The single place where routes are registered.

use std::sync::Arc;

use axum::Router;
use axum::middleware;
use axum::routing::get;
use opentatu_services::Services;

use crate::{DashboardConfig, assets, pages, security};

/// State shared by every handler.
#[derive(Debug, Clone)]
pub struct DashboardState {
    /// Use cases (the dashboard never touches the database directly).
    pub services: Services,
}

impl DashboardState {
    /// Wraps the services.
    pub fn new(services: Services) -> Self {
        Self { services }
    }
}

/// Builds the router with every route and the security middleware.
pub fn router(state: DashboardState, config: &DashboardConfig) -> Router {
    let allowed_hosts = Arc::new(config.allowed_hosts.clone());
    Router::new()
        .route("/", get(pages::home::page))
        .route("/health", get(pages::health::json))
        .route("/assets/app.css", get(assets::stylesheet))
        .layer(middleware::from_fn_with_state(
            allowed_hosts.clone(),
            security::check_origin,
        ))
        .layer(middleware::from_fn_with_state(
            allowed_hosts,
            security::check_host,
        ))
        .layer(middleware::from_fn(security::security_headers))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode, header};
    use opentatu_core::FixedClock;
    use tower::ServiceExt;

    use super::*;

    async fn app() -> Router {
        let clock = Arc::new(FixedClock::at("2026-11-29T14:32:00-03:00").unwrap());
        let services = Services::in_memory(clock).await.unwrap();
        let config = DashboardConfig::local("127.0.0.1:7878".parse().unwrap()).unwrap();
        router(DashboardState::new(services), &config)
    }

    async fn get(path: &str, host: &str) -> (StatusCode, header::HeaderMap, String) {
        let request = Request::get(path)
            .header(header::HOST, host)
            .body(Body::empty())
            .unwrap();
        let response = app().await.oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, headers, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn home_renders_components_in_pt_br() {
        let (status, headers, body) = get("/", "127.0.0.1:7878").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("Aplicações"), "{body}");
        assert!(body.contains("c-badge--critical"), "{body}");
        assert!(
            body.contains("29/11/2026 às 14:32 (horário de Brasília)"),
            "{body}"
        );
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert!(headers.contains_key(header::CONTENT_SECURITY_POLICY));
    }

    #[tokio::test]
    async fn health_returns_json() {
        let (status, _, body) = get("/health", "localhost:7878").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("\"database_ok\":true"), "{body}");
        assert!(
            body.contains("\"checked_at\":\"2026-11-29T14:32:00-03:00\""),
            "{body}"
        );
    }

    #[tokio::test]
    async fn foreign_host_is_rejected() {
        let (status, _, _) = get("/", "evil.test").await;
        assert_eq!(status, StatusCode::MISDIRECTED_REQUEST);
    }

    async fn post(origin: Option<&str>) -> StatusCode {
        let mut request = Request::post("/").header(header::HOST, "127.0.0.1:7878");
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        let response = app()
            .await
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        response.status()
    }

    #[tokio::test]
    async fn cross_origin_writes_are_blocked() {
        assert_eq!(post(Some("https://evil.test")).await, StatusCode::FORBIDDEN);
        assert_eq!(post(None).await, StatusCode::FORBIDDEN);
        // Same origin passes the middleware (405: `/` has no POST handler).
        assert_eq!(
            post(Some("http://127.0.0.1:7878")).await,
            StatusCode::METHOD_NOT_ALLOWED
        );
    }

    #[tokio::test]
    async fn stylesheet_is_served_and_cacheable() {
        let (status, headers, body) = get("/assets/app.css", "127.0.0.1:7878").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(":root"));
        assert_eq!(headers[header::CACHE_CONTROL], "public, max-age=3600");
    }
}
