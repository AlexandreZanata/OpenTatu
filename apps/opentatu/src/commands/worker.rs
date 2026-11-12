//! `opentatu worker` and `opentatu serve`.

use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use opentatu_dashboard::DashboardConfig;
use opentatu_snapshots::Snapshots;
use opentatu_worker::Worker;

use crate::cli::WorkerArgs;
use crate::output;

/// `opentatu worker [--once]`.
pub async fn run(data_dir: &Path, args: &WorkerArgs) -> anyhow::Result<()> {
    let worker = build(data_dir).await?;
    if args.once {
        let done = worker.drain().await?;
        output::line(&format!("processed {} audit(s)", done.len()));
        for id in done {
            output::line(&format!("  {id}"));
        }
        return Ok(());
    }
    output::line("OpenTatu worker running (Ctrl+C to stop)");
    worker.run(poll(args), shutdown_signal()).await?;
    Ok(())
}

/// `opentatu serve`: dashboard and worker in one process (default local mode).
pub async fn serve(data_dir: &Path, address: SocketAddr, args: &WorkerArgs) -> anyhow::Result<()> {
    let config = DashboardConfig::local(address)?;
    let services = super::services(data_dir).await?;
    let worker = build(data_dir).await?;
    output::line(&format!(
        "OpenTatu: http://{address}/ — dashboard and worker running (Ctrl+C to stop)"
    ));
    let dashboard = async {
        opentatu_dashboard::serve(config, services)
            .await
            .map_err(anyhow::Error::from)
    };
    let background = async {
        worker
            .run(poll(args), shutdown_signal())
            .await
            .map_err(anyhow::Error::from)
    };
    tokio::try_join!(dashboard, background)?;
    Ok(())
}

async fn build(data_dir: &Path) -> anyhow::Result<Worker> {
    let layout = super::layout(data_dir)?;
    let services = super::services(data_dir).await?;
    Ok(Worker::new(
        services,
        Snapshots::new(layout.clone()),
        layout,
    ))
}

fn poll(args: &WorkerArgs) -> Duration {
    Duration::from_secs(args.poll_seconds.max(1))
}

async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_err() {
        std::future::pending::<()>().await;
    }
}
