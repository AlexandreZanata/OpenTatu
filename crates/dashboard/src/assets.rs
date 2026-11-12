//! Static assets embedded in the binary (no CDN, works offline).

use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::response::IntoResponse;

/// The single stylesheet. Design tokens live in its `:root` block.
const APP_CSS: &str = include_str!("../static/app.css");

/// `GET /assets/app.css`
pub async fn stylesheet() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "text/css; charset=utf-8"),
            (CACHE_CONTROL, "public, max-age=3600"),
        ],
        APP_CSS,
    )
}
