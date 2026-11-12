//! Dashboard errors.

use std::net::SocketAddr;

use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Errors starting or running the dashboard server.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Non-loopback bind requested before owner authentication exists.
    #[error(
        "refusing to listen on {0}: the dashboard binds only to loopback until owner \
         authentication exists (docs/SECURITY.md, docs/OPEN-QUESTIONS.md Q-005)"
    )]
    NotLoopback(SocketAddr),

    /// Socket or server failure.
    #[error("dashboard server error: {0}")]
    Io(#[from] std::io::Error),
}

/// Error returned by page handlers. Details go to the log; the owner sees a generic
/// pt-BR page (no internals leak into HTML).
#[derive(Debug)]
pub struct PageError(String);

impl From<askama::Error> for PageError {
    fn from(error: askama::Error) -> Self {
        Self(format!("template rendering failed: {error}"))
    }
}

impl IntoResponse for PageError {
    fn into_response(self) -> Response {
        tracing::error!(error = %self.0, "dashboard page failed");
        let body = Html(
            "<!doctype html><html lang=\"pt-BR\"><title>Erro</title>\
                         <h1>Erro interno</h1><p>Detalhes no log do OpenTatu.</p></html>",
        );
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}
