//! The single way pages turn a template into a response.

use askama::Template;
use axum::response::Html;

use crate::PageError;

/// Renders a page template. Every handler returns `render(&page)`; never call
/// `Template::render` directly in handlers.
pub fn render(page: &impl Template) -> Result<Html<String>, PageError> {
    Ok(Html(page.render()?))
}
