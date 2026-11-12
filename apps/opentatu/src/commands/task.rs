//! `opentatu task ...`: the fix tasks planned by the audits, for the owner.

use std::path::Path;

use opentatu_core::{Slug, TaskId, TaskState};

use crate::output;

/// `opentatu task list`.
pub async fn list(data_dir: &Path, app: &str, state: Option<TaskState>) -> anyhow::Result<()> {
    let services = super::services(data_dir).await?;
    let tasks = services.tasks_of_app(&Slug::new(app)?, state).await?;
    if tasks.is_empty() {
        output::line("no tasks");
    }
    for task in tasks {
        output::line(&format!(
            "{}\tP{}\t{}\t{}\t{}\t{}",
            task.task_id,
            task.priority,
            task.severity,
            task.state,
            task.claimed_by.as_deref().unwrap_or("-"),
            task.content.title
        ));
    }
    Ok(())
}

/// `opentatu task show`.
pub async fn show(data_dir: &Path, task_id: &str) -> anyhow::Result<()> {
    let id: TaskId = task_id.trim().parse()?;
    let status = super::services(data_dir).await?.task_status(&id).await?;
    let task = &status.task;
    output::line(&format!("{} — {}", task.task_id, task.content.title));
    output::line(&format!(
        "state {} · priority {} · severity {} · planned in {}",
        task.state, task.priority, task.severity, task.audit_id
    ));
    if let Some(agent) = &task.claimed_by {
        output::line(&format!("claimed by {agent}"));
    }
    for reverification in &status.reverifications {
        let result = reverification
            .result
            .map_or("PENDING", |result| result.as_str());
        output::line(&format!(
            "reverification {} of {} ({}): {result}",
            reverification.audit_id, reverification.commit, reverification.branch
        ));
        for reason in &reverification.reasons {
            output::line(&format!("  - {reason}"));
        }
    }
    Ok(())
}
