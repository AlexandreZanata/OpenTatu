//! Thin CLI adapter: review policy, grant owner approval, or run the strict guest entry.
use crate::{cli::LabCommand, output};
use opentatu_core::{LabPolicy, Slug};
use opentatu_scanners::{LabSuite as _, QemuLab};
use std::path::Path;

pub(super) fn policy(path: Option<&Path>, app: &Slug) -> anyhow::Result<LabPolicy> {
    let path = path.ok_or_else(|| {
        anyhow::anyhow!("provide --lab-config with an owner-controlled offline VM profile")
    })?;
    QemuLab::load(path, crate::config::current_binary()?)?
        .policy(app)
        .ok_or_else(|| anyhow::anyhow!("no lab profile for {app}"))
}
pub async fn run(data: &Path, path: Option<&Path>, command: LabCommand) -> anyhow::Result<()> {
    match command {
        LabCommand::Profile { app } => {
            let p = policy(path, &app.parse()?)?;
            output::line(&serde_json::to_string_pretty(&p)?);
        }
        LabCommand::Approve { audit } => {
            let id = audit.parse()?;
            let services = super::services(data).await?;
            let audit = services.audit(&id).await?;
            let p = policy(path, &audit.app)?;
            services.approve_lab_audit(&id, &p).await?;
            output::line(&format!(
                "approved restricted offline lab for {id}, commit {}, profile {}",
                audit.commit, p.profile_digest
            ));
        }
        LabCommand::Guest { manifest } => {
            let receipt = match opentatu_scanners::run_guest(&manifest).await {
                Ok(receipt) => receipt,
                Err(error) => {
                    // Fixed stage diagnostics only; guest errors contain no HTTP bodies,
                    // source, environment or fictitious header values.
                    output::line(&format!("OPENTATU_LAB_ERROR {error}"));
                    return Err(error.into());
                }
            };
            output::line(&format!(
                "OPENTATU_LAB_V1 {}",
                serde_json::to_string(&receipt)?
            ));
        }
    }
    Ok(())
}
