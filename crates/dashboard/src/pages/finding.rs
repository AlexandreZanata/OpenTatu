//! `GET /findings/{id}` — the live record of a finding (spec §B.7): details, the skeptic's
//! review, the evidence of its confirmations and its timeline.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::Html;
use opentatu_core::FindingId;

use crate::PageError;
use crate::render::render;
use crate::routes::DashboardState;
use crate::view::{
    FindingRow, Item, KeyValue, Layout, TimelineEntry, finding_row, finding_state_badge,
    timeline_entry,
};

/// Template data of the finding page.
#[derive(Template)]
#[template(path = "pages/finding.html")]
struct FindingPage {
    layout: Layout,
    finding: FindingRow,
    details: Vec<KeyValue>,
    review: Vec<String>,
    evidence: Vec<Item>,
    timeline: Vec<TimelineEntry>,
}

/// Handler of `GET /findings/{id}`.
pub async fn page(
    State(state): State<DashboardState>,
    Path(id): Path<String>,
) -> Result<Html<String>, PageError> {
    let id: FindingId = id.parse().map_err(|_| PageError::NotFound)?;
    let services = &state.services;
    let finding = services.finding(&id).await?;
    let kv = |key: &'static str, value: String| KeyValue { key, value };
    let details = vec![
        kv("Severidade", finding.severity.to_string()),
        kv(
            "Estado",
            finding_state_badge(finding.state).label.to_owned(),
        ),
        kv("Categoria", finding.category.clone()),
        kv("CWE", finding.cwe.clone()),
        kv("ASVS", finding.asvs.join(", ")),
        kv(
            "Local",
            format!(
                "{}:{} ({})",
                finding.location.file, finding.location.lines, finding.location.symbol
            ),
        ),
        kv("Commit", finding.commit.to_string()),
        kv(
            "Detectado primeiro em",
            finding.first_detected.audit_id.to_string(),
        ),
        kv(
            "Detectado por último em",
            finding.last_detected.audit_id.to_string(),
        ),
        kv(
            "Prioridade de correção",
            format!("{} (1 = mais urgente)", finding.fix_priority()),
        ),
        kv("Fingerprint", finding.fingerprint.to_string()),
    ];
    let review = finding
        .review
        .as_ref()
        .map(|r| {
            vec![
                format!(
                    "Nível base {}; ajustes: dados sensíveis {}, exige admin comprometido {}; severidade final {} (calculada pelo sistema).",
                    r.base_severity,
                    if r.adjustments.involves_sensitive_data { "sim" } else { "não" },
                    if r.adjustments.requires_compromised_admin { "sim" } else { "não" },
                    r.final_severity
                ),
                r.justification.clone(),
            ]
        })
        .unwrap_or_default();
    let mut evidence = Vec::new();
    for hypothesis in services
        .hypotheses_of_audit(&finding.first_detected.audit_id)
        .await?
    {
        if hypothesis.finding_id.as_ref() != Some(&finding.finding_id) {
            continue;
        }
        for item in hypothesis.verification.iter().flat_map(|v| &v.evidence) {
            let steps: Vec<String> = item
                .steps
                .iter()
                .map(|s| format!("{}:{} — {}", s.file, s.line, s.what_happens))
                .collect();
            evidence.push(Item {
                title: format!(
                    "{} ({}): {}",
                    item.evidence_id
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                    item.kind,
                    item.description
                ),
                detail: steps.join(" → "),
            });
        }
    }
    let mut events = services.finding_timeline(&id).await?;
    events.reverse();
    render(&FindingPage {
        layout: Layout::default(),
        finding: finding_row(&finding),
        details,
        review,
        evidence,
        timeline: events.iter().map(timeline_entry).collect(),
    })
}
