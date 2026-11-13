//! Explicit one-shot live inventory; no audit queue, model, scheduler or remediation.

use opentatu_core::{Actor, HostCommand, HostFailure, HostInspection, HostResult, Slug};
use opentatu_scanners::HostSuite;
use opentatu_services::Services;

use crate::Error;

/// Inspects one owner-configured host of a registered app. Every attempt is committed
/// before calling SSH; partial failures still produce a complete, honest collection trail.
pub async fn inspect_host(
    services: &Services,
    hosts: &dyn HostSuite,
    app: &Slug,
    profile: &Slug,
) -> Result<HostInspection, Error> {
    let policy = hosts.policy(app, profile).ok_or_else(|| {
        opentatu_services::Error::Conflict(
            "host profile is absent or belongs to another app".into(),
        )
    })?;
    let inspection = services
        .start_host_inspection(app, policy, &Actor::Owner)
        .await?;
    for command in HostCommand::ALL {
        services.start_host_command(&inspection, *command).await?;
        let mut result = hosts.query(app, &inspection.policy, *command).await;
        if let HostResult::Observed { observation } = &result
            && (observation.command() != *command || !observation.is_valid())
        {
            result = HostResult::Failed {
                failure: HostFailure::InvalidResponse,
            };
        }
        services
            .complete_host_command(&inspection, *command, result)
            .await?;
    }
    Ok(services.complete_host_inspection(&inspection).await?)
}
