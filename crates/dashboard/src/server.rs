//! Server startup.

use opentatu_services::Services;
use tokio::net::TcpListener;

use crate::{DashboardConfig, DashboardState, Error, router};

/// Binds the configured address and serves the dashboard until Ctrl+C.
pub async fn serve(config: DashboardConfig, services: Services) -> Result<(), Error> {
    let listener = TcpListener::bind(config.address).await?;
    tracing::info!(address = %config.address, "dashboard listening");
    let app = router(DashboardState::new(services), &config);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_err() {
        tracing::warn!("could not listen for Ctrl+C; stop the process to shut down");
        std::future::pending::<()>().await;
    }
}
