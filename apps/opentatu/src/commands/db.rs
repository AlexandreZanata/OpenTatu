//! `opentatu db ...`

use std::path::Path;

use crate::output;

/// `opentatu db migrate`: opening the database applies pending migrations.
pub async fn migrate(data_dir: &Path) -> anyhow::Result<()> {
    let services = super::services(data_dir).await?;
    let health = services.health().await;
    anyhow::ensure!(
        health.database_ok,
        "database is not reachable after migration"
    );
    output::line(&format!(
        "database ready at {} (schema v{})",
        data_dir.display(),
        health.schema_version
    ));
    Ok(())
}
