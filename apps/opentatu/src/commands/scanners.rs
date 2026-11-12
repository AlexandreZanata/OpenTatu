//! `opentatu scanners ...`: refresh the scanners' offline data and show readiness.

use std::path::Path;

use anyhow::Context as _;
use opentatu_core::ScannerKind;
use opentatu_scanners::{ScannerSuite as _, Scanners, ScannersConfig};

use crate::output;

/// `opentatu scanners update [--only <scanner>]`.
pub async fn update(
    data_dir: &Path,
    config_path: &Path,
    only: Option<ScannerKind>,
) -> anyhow::Result<()> {
    let scanners = build(data_dir, config_path)?;
    let kinds = match only {
        Some(kind) => vec![kind],
        None => scanners.enabled(),
    };
    for kind in kinds {
        output::line(&format!(
            "{kind}: updating (network access; application code is never mounted)…"
        ));
        scanners
            .update(kind)
            .await
            .with_context(|| format!("updating {kind}"))?;
        output::line(&format!("{kind}: ready"));
    }
    Ok(())
}

/// `opentatu scanners status`.
pub fn status(data_dir: &Path, config_path: &Path) -> anyhow::Result<()> {
    let scanners = build(data_dir, config_path)?;
    let enabled = scanners.enabled();
    for kind in ScannerKind::ALL.iter().copied() {
        let Some(tool) = scanners.config().tool(kind) else {
            continue;
        };
        let state = if !enabled.contains(&kind) {
            "disabled"
        } else if scanners.ready(kind) {
            "ready"
        } else {
            "NOT READY (opentatu scanners update)"
        };
        output::line(&format!(
            "{kind}\t{}\t{state}\t{}",
            tool.version, tool.image
        ));
    }
    Ok(())
}

/// The container-backed scanners: the single place they are assembled.
pub fn build(data_dir: &Path, config_path: &Path) -> anyhow::Result<Scanners> {
    let layout = super::layout(data_dir)?;
    let config = ScannersConfig::load(config_path)
        .with_context(|| format!("loading {}", config_path.display()))?;
    Ok(Scanners::new(config, layout))
}
