//! Per-audit budget (spec §A.2, §C.7 "At any step", R13): tokens, model calls and minutes
//! from `config/models.toml`. Every model call asks [`BudgetGuard::allow`] first; when a
//! limit is reached the guard records `BUDGET_EXHAUSTED` once and refuses every later call,
//! and the audit lists what was left out (it never pretends to be complete).

use std::sync::Mutex;

use opentatu_core::Audit;
use opentatu_llm::AuditBudget;

use crate::{Error, Worker};

/// Shared by the AI steps of one audit (verifier calls run concurrently).
#[derive(Debug)]
pub(crate) struct BudgetGuard {
    limits: Option<AuditBudget>,
    exhausted: Mutex<Option<String>>,
}

/// Usage of the audit so far.
struct Used {
    tokens: u64,
    calls: u64,
    minutes: i64,
}

impl BudgetGuard {
    /// Guard with the limits of the audit's kind (`None`: no limits configured).
    pub(crate) fn new(limits: Option<AuditBudget>) -> Self {
        Self {
            limits,
            exhausted: Mutex::new(None),
        }
    }

    /// Whether one more model call fits. The first refusal records `BUDGET_EXHAUSTED`.
    pub(crate) async fn allow(&self, worker: &Worker, audit: &Audit) -> Result<bool, Error> {
        if self.reason().is_some() {
            return Ok(false);
        }
        let Some(limits) = self.limits else {
            return Ok(true);
        };
        let used = used(worker, audit).await?;
        let reason = if used.tokens >= limits.max_tokens {
            Some(format!("{} de {} tokens", used.tokens, limits.max_tokens))
        } else if used.calls >= u64::from(limits.max_calls) {
            Some(format!("{} de {} chamadas", used.calls, limits.max_calls))
        } else if used.minutes >= i64::from(limits.max_minutes) {
            Some(format!(
                "{} de {} minutos",
                used.minutes, limits.max_minutes
            ))
        } else {
            None
        };
        let Some(reason) = reason else {
            return Ok(true);
        };
        let first = {
            let mut slot = self
                .exhausted
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let first = slot.is_none();
            if first {
                *slot = Some(reason.clone());
            }
            first
        };
        if first {
            tracing::warn!(audit_id = %audit.id, %reason, "audit budget exhausted");
            worker
                .services
                .record_budget_exhausted(
                    &audit.id,
                    serde_json::json!({
                        "reason": reason,
                        "used": { "tokens": used.tokens, "calls": used.calls, "minutes": used.minutes },
                        "limits": { "tokens": limits.max_tokens, "calls": limits.max_calls, "minutes": limits.max_minutes },
                    }),
                )
                .await?;
        }
        Ok(false)
    }

    /// Remaining budget for the base prompt (`remaining_budget`).
    pub(crate) async fn summary(&self, worker: &Worker, audit: &Audit) -> Result<String, Error> {
        let Some(limits) = self.limits else {
            return Ok("no budget configured for this audit kind (100% remaining)".to_owned());
        };
        let used = used(worker, audit).await?;
        let share =
            |used: u64, max: u64| 100u64.saturating_sub(used.saturating_mul(100) / max.max(1));
        let percent = share(used.tokens, limits.max_tokens)
            .min(share(used.calls, u64::from(limits.max_calls)))
            .min(share(
                u64::try_from(used.minutes).unwrap_or(0),
                u64::from(limits.max_minutes),
            ));
        Ok(format!(
            "{percent}% remaining — {} of {} tokens, {} of {} model calls, {} of {} minutes used",
            used.tokens,
            limits.max_tokens,
            used.calls,
            limits.max_calls,
            used.minutes,
            limits.max_minutes
        ))
    }

    /// Owner-facing (pt-BR) limitation when the budget ran out.
    pub(crate) fn limitation(&self) -> Option<String> {
        self.reason().map(|reason| {
            format!("Orçamento da auditoria esgotado ({reason}): as etapas de IA restantes não foram executadas (R13).")
        })
    }

    fn reason(&self) -> Option<String> {
        self.exhausted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

async fn used(worker: &Worker, audit: &Audit) -> Result<Used, Error> {
    let calls = worker.services.llm_usage_of_audit(&audit.id).await?;
    let tokens = calls
        .iter()
        .map(|c| c.usage.input_tokens + c.usage.output_tokens)
        .sum();
    let started = audit.started_at.unwrap_or(audit.created_at);
    Ok(Used {
        tokens,
        calls: u64::try_from(calls.len()).unwrap_or(u64::MAX),
        minutes: worker.services.now().seconds_since(&started) / 60,
    })
}
