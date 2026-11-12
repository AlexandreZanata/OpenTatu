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

/// Error returned by page handlers. Details go to the log; the owner sees a generic pt-BR
/// page (no internals leak into HTML).
#[derive(Debug)]
pub enum PageError {
    /// The requested app or audit does not exist (404).
    NotFound,
    /// Anything else (500); the message is logged only.
    Internal(String),
}

impl From<askama::Error> for PageError {
    fn from(error: askama::Error) -> Self {
        Self::Internal(format!("template rendering failed: {error}"))
    }
}

impl From<opentatu_services::Error> for PageError {
    fn from(error: opentatu_services::Error) -> Self {
        match error {
            opentatu_services::Error::UnknownApp(_) | opentatu_services::Error::UnknownAudit(_) => {
                Self::NotFound
            }
            other => Self::Internal(other.to_string()),
        }
    }
}

impl IntoResponse for PageError {
    fn into_response(self) -> Response {
        let (status, title, message) = match self {
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "Não encontrado",
                "Esta página não existe.",
            ),
            Self::Internal(detail) => {
                tracing::error!(error = %detail, "dashboard page failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Erro interno",
                    "Detalhes no log do OpenTatu.",
                )
            }
        };
        let body = Html(format!(
            "<!doctype html><html lang=\"pt-BR\"><title>{title}</title>\
             <h1>{title}</h1><p>{message}</p><p><a href=\"/\">Início</a></p></html>"
        ));
        (status, body).into_response()
    }
}
