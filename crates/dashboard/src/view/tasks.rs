//! View data of fix tasks and reverifications (pt-BR labels).

use opentatu_core::{Reverification, ReverificationResult, Task, TaskState};

use super::{Badge, severity_badge};

/// Badge of a task state.
pub fn task_state_badge(state: TaskState) -> Badge {
    let (label, modifier) = match state {
        TaskState::Open => ("Aberta", "queued"),
        TaskState::Claimed => ("Assumida", "running"),
        TaskState::FixSubmitted => ("Correção em reverificação", "running"),
        TaskState::Done => ("Concluída", "ok"),
    };
    Badge { label, modifier }
}

/// Badge of a reverification result (`None` = pending).
pub fn reverification_badge(result: Option<ReverificationResult>) -> Badge {
    let (label, modifier) = match result {
        None => ("Pendente", "queued"),
        Some(ReverificationResult::Passed) => ("Aprovada", "ok"),
        Some(ReverificationResult::Failed) => ("Rejeitada", "failed"),
    };
    Badge { label, modifier }
}

/// One row of the `ui::tasks_table` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRow {
    /// `<FINDING>-T<N>`.
    pub id: String,
    /// Finding it fixes (link target).
    pub finding: String,
    /// Title (pt-BR).
    pub title: String,
    /// Fix priority (1 = most urgent).
    pub priority: u8,
    /// Severity badge.
    pub severity: Badge,
    /// State badge.
    pub state: Badge,
    /// Agent holding it, or `—`.
    pub agent: String,
    /// Rejected fixes so far.
    pub rejections: usize,
}

/// Row data of a task.
pub fn task_row(task: &Task) -> TaskRow {
    TaskRow {
        id: task.task_id.to_string(),
        finding: task.task_id.finding().to_string(),
        title: task.content.title.clone(),
        priority: task.priority,
        severity: severity_badge(task.severity),
        state: task_state_badge(task.state),
        agent: task.claimed_by.clone().unwrap_or_else(|| "—".to_owned()),
        rejections: task.rejections.len(),
    }
}

/// One line of a task's reverifications (`ui::item_list` item: title + detail).
pub fn reverification_item(reverification: &Reverification) -> super::Item {
    let badge = reverification_badge(reverification.result);
    super::Item {
        title: format!(
            "{} — {} · commit {} ({})",
            reverification.audit_id,
            badge.label,
            reverification.commit.short(),
            reverification.branch
        ),
        detail: reverification.reasons.join(" "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_task_state_and_result_has_a_label() {
        for state in TaskState::ALL {
            assert!(!task_state_badge(*state).label.is_empty());
        }
        assert_eq!(reverification_badge(None).label, "Pendente");
        assert_eq!(
            reverification_badge(Some(ReverificationResult::Passed)).modifier,
            "ok"
        );
    }
}
