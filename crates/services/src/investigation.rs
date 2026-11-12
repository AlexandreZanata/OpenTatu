//! Use cases: the investigation of an audit (spec §A.6, §C.7 steps 7–14; R05, R06, R08,
//! R09, R13, R18).
//!
//! Every state change appends its event in the same transaction. Ids (`-H-NNN`,
//! `-EV-NNN`, `-SEC-NNNN`) and dates are assigned here, never by the models (R03).

use std::collections::BTreeSet;

use opentatu_core::{
    Actor, AiRole, Audit, AuditId, Comparison, Detection, Event, EventKind, EvidenceId, Finding,
    FindingId, FindingState, Fingerprint, HuntRound, Hypothesis, HypothesisContent, HypothesisId,
    NewEvent, ReportText, SkepticReview, Slug, Verification,
};
use opentatu_memory::{DedupOutcome, DismissedBefore, compare};

use crate::{Error, Services};

/// Input of [`Services::record_hypothesis`]: what the hunter said, plus what the system
/// computed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewHypothesis {
    /// Validated content.
    pub content: HypothesisContent,
    /// Fingerprint computed from the code at the location (§B.8).
    pub fingerprint: Fingerprint,
    /// Hunter model.
    pub model: String,
    /// `hunter@x.y.z`.
    pub prompt_version: String,
    /// What the system changed while validating (pt-BR).
    pub system_notes: Vec<String>,
}

impl Services {
    /// Stores one hunter round.
    pub async fn record_hunt_round(&self, audit: &AuditId, round: &HuntRound) -> Result<(), Error> {
        let mut conn = self.db.connection().await?;
        opentatu_db::investigation::insert_round(&mut conn, audit, round, &self.clock.now())
            .await?;
        Ok(())
    }

    /// Records a hypothesis according to what the memory says about it
    /// ([`opentatu_memory::classify`]); returns `None` for a duplicate within the audit.
    ///
    /// - new → `HYPOTHESIS`, to be verified;
    /// - persistent open finding → linked, `CONFIRMED` without a new verification, finding
    ///   re-detected;
    /// - closed finding → `REGRESSION` (R09);
    /// - known false positive → `DISMISSED` without a model call (R08);
    /// - active accepted risk → linked, not verified.
    pub async fn record_hypothesis(
        &self,
        audit: &Audit,
        input: NewHypothesis,
        outcome: &DedupOutcome,
    ) -> Result<Option<Hypothesis>, Error> {
        if matches!(outcome, DedupOutcome::DuplicateInAudit(_)) {
            return Ok(None);
        }
        let now = self.clock.now();
        let mut tx = self.db.begin().await?;
        let number = opentatu_db::investigation::hypothesis_count(&mut tx, &audit.id).await? + 1;
        let mut hypothesis = Hypothesis {
            hypothesis_id: HypothesisId::new(&audit.id, number),
            audit_id: audit.id.clone(),
            fingerprint: input.fingerprint,
            state: FindingState::Hypothesis,
            finding_id: None,
            created_at: now,
            model: input.model.clone(),
            prompt_version: input.prompt_version.clone(),
            content: input.content,
            verification: None,
            system_notes: input.system_notes,
        };
        let mut data = serde_json::json!({
            "fingerprint": hypothesis.fingerprint,
            "potential_severity": hypothesis.content.potential_severity,
            "confidence": hypothesis.content.confidence,
            "entry_point": hypothesis.content.entry_point,
        });
        let mut finding_change: Option<(Finding, EventKind)> = None;
        match outcome {
            DedupOutcome::New { possibly_same_as } => {
                if let Some(id) = possibly_same_as {
                    hypothesis.system_notes.push(format!(
                        "Possivelmente o mesmo que {id} (mesmo CWE, arquivo e símbolo; código diferente)."
                    ));
                    data["possibly_same_as"] = serde_json::json!(id);
                }
            }
            DedupOutcome::Persistent(id) | DedupOutcome::Regression(id) => {
                let mut finding = opentatu_db::investigation::finding(&mut tx, id)
                    .await?
                    .ok_or_else(|| Error::UnknownFinding(id.clone()))?;
                finding.last_detected = Detection {
                    audit_id: audit.id.clone(),
                    at: now,
                };
                finding.hypothesis_id = hypothesis.hypothesis_id.clone();
                finding.location = hypothesis.content.location.clone();
                finding.commit = audit.commit.clone();
                hypothesis.state = FindingState::Confirmed;
                hypothesis.finding_id = Some(id.clone());
                let kind = if matches!(outcome, DedupOutcome::Regression(_)) {
                    finding
                        .state
                        .validate_transition(FindingState::Regression)?;
                    finding.state = FindingState::Regression;
                    finding.regression = true;
                    hypothesis.system_notes.push(format!(
                        "Regressão de {id}: o fingerprint de um achado fechado reapareceu (R09)."
                    ));
                    EventKind::RegressionDetected
                } else {
                    hypothesis.system_notes.push(format!(
                        "Achado aberto {id} reencontrado com o mesmo código: persistente, sem nova verificação."
                    ));
                    EventKind::FindingConfirmed
                };
                data["finding_id"] = serde_json::json!(id);
                finding_change = Some((finding, kind));
            }
            DedupOutcome::AcceptedRisk(id) => {
                hypothesis.finding_id = Some(id.clone());
                hypothesis.system_notes.push(format!(
                    "Risco aceito ativo em {id}: não reaberto nem verificado."
                ));
                data["finding_id"] = serde_json::json!(id);
            }
            DedupOutcome::KnownFalsePositive(reference) => {
                hypothesis.state = FindingState::Dismissed;
                hypothesis.system_notes.push(format!(
                    "Falso positivo conhecido ({reference}): mesmo código descartado antes, não reaberto (R08)."
                ));
                data["known_false_positive"] = serde_json::json!(reference);
            }
            DedupOutcome::DuplicateInAudit(_) => {}
        }
        let mut event = NewEvent::new(
            EventKind::HypothesisCreated,
            Actor::Role(AiRole::Hunter),
            now,
        )
        .with_audit(audit.id.clone())
        .with_target(&hypothesis.hypothesis_id)
        .with_data(data);
        event.model = Some(input.model);
        event.prompt_version = Some(input.prompt_version);
        opentatu_db::investigation::insert_hypothesis(&mut tx, &hypothesis, &event).await?;
        if let Some((finding, kind)) = finding_change {
            let event = NewEvent::new(kind, Actor::System, now)
                .with_audit(audit.id.clone())
                .with_target(&finding.finding_id)
                .with_data(serde_json::json!({
                    "hypothesis_id": hypothesis.hypothesis_id,
                    "fingerprint": finding.fingerprint,
                    "redetected": true,
                }));
            opentatu_db::investigation::update_finding(&mut tx, &finding, &event).await?;
        }
        tx.commit().await?;
        Ok(Some(hypothesis))
    }

    /// Records the verifier's answer. Evidence ids are assigned here; a confirmation
    /// creates the finding (`<SLUG>-SEC-NNNN`, never reused) with `FINDING_CONFIRMED`.
    pub async fn record_verification(
        &self,
        id: &HypothesisId,
        mut verification: Verification,
    ) -> Result<(Hypothesis, Option<Finding>), Error> {
        let now = self.clock.now();
        let mut tx = self.db.begin().await?;
        let mut hypothesis = opentatu_db::investigation::hypothesis(&mut tx, id)
            .await?
            .ok_or_else(|| Error::UnknownHypothesis(id.clone()))?;
        let audit_id = hypothesis.audit_id.clone();
        let next = state_after(hypothesis.state, verification.verdict.state())?;
        let mut number = opentatu_db::investigation::evidence_count(&mut tx, &audit_id).await?;
        for item in &mut verification.evidence {
            number += 1;
            item.evidence_id = Some(EvidenceId::new(&audit_id, number));
        }
        let mut event = NewEvent::new(
            EventKind::HypothesisVerified,
            Actor::Role(AiRole::Verifier),
            now,
        )
        .with_audit(audit_id.clone())
        .with_target(id)
        .with_data(serde_json::json!({ "verdict": verification.verdict }));
        event.model = Some(verification.model.clone());
        event.prompt_version = Some(verification.prompt_version.clone());
        let model = verification.model.clone();
        let prompt = verification.prompt_version.clone();
        hypothesis.state = next;
        hypothesis.verification = Some(verification);

        let mut created = None;
        if next == FindingState::Confirmed {
            let app = audit_id.slug().clone();
            let number = opentatu_db::investigation::next_finding_number(&mut tx, &app).await?;
            let audit = opentatu_db::audits::get(&mut tx, &audit_id)
                .await?
                .ok_or_else(|| Error::UnknownAudit(audit_id.clone()))?;
            let detection = Detection {
                audit_id: audit_id.clone(),
                at: now,
            };
            let content = &hypothesis.content;
            let finding = Finding {
                finding_id: FindingId::new(&app, number),
                app,
                title: content.title.clone(),
                category: content.category.clone(),
                cwe: content.cwe.clone(),
                asvs: content.asvs.clone(),
                severity: content.potential_severity,
                state: FindingState::Confirmed,
                fingerprint: hypothesis.fingerprint.clone(),
                location: content.location.clone(),
                commit: audit.commit,
                first_detected: detection.clone(),
                last_detected: detection,
                hypothesis_id: hypothesis.hypothesis_id.clone(),
                regression: false,
                review: None,
            };
            hypothesis.finding_id = Some(finding.finding_id.clone());
            let mut confirmed = NewEvent::new(
                EventKind::FindingConfirmed,
                Actor::Role(AiRole::Verifier),
                now,
            )
            .with_audit(audit_id.clone())
            .with_target(&finding.finding_id)
            .with_data(serde_json::json!({
                "hypothesis_id": hypothesis.hypothesis_id,
                "severity": finding.severity,
                "fingerprint": finding.fingerprint,
            }));
            confirmed.model = Some(model);
            confirmed.prompt_version = Some(prompt);
            opentatu_db::investigation::insert_finding(&mut tx, &finding, &confirmed).await?;
            created = Some(finding);
        }
        opentatu_db::investigation::update_hypothesis(&mut tx, &hypothesis, &event).await?;
        tx.commit().await?;
        Ok((hypothesis, created))
    }

    /// Records the skeptic's review: decision and final severity computed by
    /// [`SkepticReview::decide`] (R18).
    pub async fn record_review(
        &self,
        id: &FindingId,
        review: SkepticReview,
    ) -> Result<Finding, Error> {
        let now = self.clock.now();
        let mut tx = self.db.begin().await?;
        let mut finding = opentatu_db::investigation::finding(&mut tx, id)
            .await?
            .ok_or_else(|| Error::UnknownFinding(id.clone()))?;
        finding.state.validate_transition(review.decision)?;
        let kind = match review.decision {
            FindingState::Upheld => EventKind::FindingUpheld,
            FindingState::Downgraded => EventKind::FindingDowngraded,
            _ => EventKind::FindingDismissed,
        };
        let mut event = NewEvent::new(kind, Actor::Role(AiRole::Skeptic), now)
            .with_audit(finding.last_detected.audit_id.clone())
            .with_target(id)
            .with_data(serde_json::json!({
                "base_severity": review.base_severity,
                "adjustments": review.adjustments,
                "final_severity": review.final_severity,
                "previous_severity": finding.severity,
            }));
        event.model = Some(review.model.clone());
        event.prompt_version = Some(review.prompt_version.clone());
        finding.state = review.decision;
        if review.decision != FindingState::Dismissed {
            finding.severity = review.final_severity;
        }
        finding.review = Some(review);
        opentatu_db::investigation::update_finding(&mut tx, &finding, &event).await?;
        tx.commit().await?;
        Ok(finding)
    }

    /// Records that the audit's budget ran out (`BUDGET_EXHAUSTED`, R13).
    pub async fn record_budget_exhausted(
        &self,
        audit: &AuditId,
        detail: serde_json::Value,
    ) -> Result<(), Error> {
        let event = NewEvent::new(EventKind::BudgetExhausted, Actor::System, self.clock.now())
            .with_audit(audit.clone())
            .with_target(audit)
            .with_data(detail);
        let mut tx = self.db.begin().await?;
        opentatu_db::events::append(&mut tx, &event).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Stores the report text of an audit, stamped with the clock.
    pub async fn record_report(&self, mut report: ReportText) -> Result<ReportText, Error> {
        report.created_at = self.clock.now();
        let mut conn = self.db.connection().await?;
        opentatu_db::investigation::insert_report(&mut conn, &report).await?;
        Ok(report)
    }

    /// Hypotheses of an audit, in order.
    pub async fn hypotheses_of_audit(&self, audit: &AuditId) -> Result<Vec<Hypothesis>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::investigation::hypotheses_of_audit(&mut conn, audit).await?)
    }

    /// The hypothesis with this id.
    pub async fn hypothesis(&self, id: &HypothesisId) -> Result<Hypothesis, Error> {
        let mut conn = self.db.connection().await?;
        opentatu_db::investigation::hypothesis(&mut conn, id)
            .await?
            .ok_or_else(|| Error::UnknownHypothesis(id.clone()))
    }

    /// Hunter rounds of an audit.
    pub async fn hunt_rounds(&self, audit: &AuditId) -> Result<Vec<HuntRound>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::investigation::rounds_of_audit(&mut conn, audit).await?)
    }

    /// Findings of an app, by number.
    pub async fn findings_of_app(&self, app: &Slug) -> Result<Vec<Finding>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::investigation::findings_of_app(&mut conn, app).await?)
    }

    /// The finding with this id.
    pub async fn finding(&self, id: &FindingId) -> Result<Finding, Error> {
        let mut conn = self.db.connection().await?;
        opentatu_db::investigation::finding(&mut conn, id)
            .await?
            .ok_or_else(|| Error::UnknownFinding(id.clone()))
    }

    /// Events about a finding (its timeline, §B.7), oldest first.
    pub async fn finding_timeline(&self, id: &FindingId) -> Result<Vec<Event>, Error> {
        let target = id.to_string();
        let events = self.app_history(id.slug()).await?;
        Ok(events
            .into_iter()
            .filter(|e| e.content.target.as_deref() == Some(target.as_str()))
            .collect())
    }

    /// Hypotheses dismissed in the app's earlier audits (R08).
    pub async fn dismissed_hypotheses(&self, audit: &Audit) -> Result<Vec<Hypothesis>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(
            opentatu_db::investigation::dismissed_hypotheses(
                &mut conn,
                &audit.app,
                Some(&audit.id),
            )
            .await?,
        )
    }

    /// Every hypothesis of the app dismissed as a false positive (R08), oldest first.
    pub async fn false_positives(&self, app: &Slug) -> Result<Vec<Hypothesis>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::investigation::dismissed_hypotheses(&mut conn, app, None).await?)
    }

    /// Evidence ids of every confirmation of a finding, across audits.
    pub async fn evidence_of_finding(&self, id: &FindingId) -> Result<Vec<EvidenceId>, Error> {
        let mut conn = self.db.connection().await?;
        let hypotheses = opentatu_db::investigation::hypotheses_of_finding(&mut conn, id).await?;
        Ok(hypotheses
            .iter()
            .filter_map(|h| h.verification.as_ref())
            .flat_map(|v| v.evidence.iter().filter_map(|e| e.evidence_id.clone()))
            .collect())
    }

    /// The memory of dismissals for [`opentatu_memory::classify`] (R08).
    pub async fn dismissed_before(&self, audit: &Audit) -> Result<Vec<DismissedBefore>, Error> {
        Ok(self
            .dismissed_hypotheses(audit)
            .await?
            .into_iter()
            .map(|h| DismissedBefore {
                fingerprint: h.fingerprint,
                reference: h.hypothesis_id.to_string(),
            })
            .collect())
    }

    /// The report text of an audit, if the reporter (or its fallback) ran.
    pub async fn report_of_audit(&self, audit: &AuditId) -> Result<Option<ReportText>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::investigation::report_of_audit(&mut conn, audit).await?)
    }

    /// Comparison of an audit with the app's previous final audit.
    pub async fn comparison_of_audit(&self, audit: &Audit) -> Result<Comparison, Error> {
        let findings = self.findings_of_app(&audit.app).await?;
        Ok(compare(
            &audit.id,
            audit.previous_audit_id.as_ref(),
            &findings,
        ))
    }

    /// ASVS requirement sets of an audit for [`opentatu_core::requirement_statuses`]:
    /// `(checked, failed, unresolved)`.
    pub(crate) async fn requirement_sets(
        &self,
        audit: &AuditId,
    ) -> Result<(BTreeSet<String>, BTreeSet<String>, BTreeSet<String>), Error> {
        let checked = self
            .hunt_rounds(audit)
            .await?
            .into_iter()
            .flat_map(|round| round.requirements_checked)
            .collect();
        let mut failed = BTreeSet::new();
        let mut unresolved = BTreeSet::new();
        for hypothesis in self.hypotheses_of_audit(audit).await? {
            let ids = hypothesis.content.asvs.iter().cloned();
            match hypothesis.state {
                FindingState::Hypothesis | FindingState::Inconclusive => unresolved.extend(ids),
                FindingState::Confirmed => {
                    let dismissed = match &hypothesis.finding_id {
                        Some(id) => self.finding(id).await?.state == FindingState::Dismissed,
                        None => false,
                    };
                    if !dismissed {
                        failed.extend(ids);
                    }
                }
                _ => {}
            }
        }
        Ok((checked, failed, unresolved))
    }
}

/// `HYPOTHESIS → verdict` (or `INCONCLUSIVE → verdict`) through the finding state machine.
fn state_after(current: FindingState, next: FindingState) -> Result<FindingState, Error> {
    current.validate_transition(next)?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use opentatu_core::{
        AuditKind, AuditStatus, CodeLocation, Confidence, EvidenceItem, EvidenceStep,
        FingerprintInput, RequirementStatus, ScannerTriage, Severity, SeverityAdjustments, Verdict,
    };
    use opentatu_memory::classify;

    use super::*;
    use crate::NewAudit;
    use crate::apps::tests::{new_app, services};
    use crate::maps::tests::new_map;

    fn content(asvs: &str) -> HypothesisContent {
        HypothesisContent {
            title: "Pedido de outro usuário acessível".into(),
            category: "Controle de acesso".into(),
            cwe: "CWE-639".into(),
            asvs: vec![asvs.into()],
            entry_point: Some("EP-001".into()),
            location: CodeLocation {
                file: "src/routes.rs".into(),
                lines: "3-5".into(),
                symbol: "login".into(),
            },
            reasoning: "x".into(),
            impact_scenario: "x".into(),
            required_conditions: vec![],
            how_to_verify: "x".into(),
            potential_severity: Severity::High,
            confidence: Confidence::High,
            origin: "manual_analysis".into(),
            possible_regression_of: None,
        }
    }

    fn new_hypothesis(snippet: &str, asvs: &str) -> NewHypothesis {
        NewHypothesis {
            content: content(asvs),
            fingerprint: Fingerprint::compute(&FingerprintInput {
                cwe: "CWE-639",
                file: "src/routes.rs",
                symbol: "login",
                snippet,
            }),
            model: "deepseek-v4.1-flash".into(),
            prompt_version: "hunter@2.0.0".into(),
            system_notes: vec![],
        }
    }

    fn verification(verdict: Verdict) -> Verification {
        Verification {
            verdict,
            justification: "x".into(),
            evidence: vec![EvidenceItem {
                evidence_id: None,
                kind: "code_flow".into(),
                description: "x".into(),
                steps: vec![EvidenceStep {
                    file: "src/routes.rs".into(),
                    line: "4".into(),
                    what_happens: "x".into(),
                }],
            }],
            mitigations_found: vec![],
            regression_test: None,
            missing_to_conclude: String::new(),
            model: "deepseek-v4.1-flash".into(),
            prompt_version: "verifier@1.1.0".into(),
            system_notes: vec![],
        }
    }

    async fn running_audit(services: &Services) -> Audit {
        services
            .create_audit(NewAudit {
                app: Slug::new("abastevo").unwrap(),
                kind: AuditKind::Full,
                commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
                branch: None,
                requested_by: Actor::Owner,
            })
            .await
            .unwrap();
        services.start_next_audit().await.unwrap().unwrap()
    }

    async fn classify_for(
        services: &Services,
        audit: &Audit,
        input: &NewHypothesis,
    ) -> DedupOutcome {
        let findings = services.findings_of_app(&audit.app).await.unwrap();
        let dismissed = services.dismissed_before(audit).await.unwrap();
        let earlier: Vec<_> = services
            .hypotheses_of_audit(&audit.id)
            .await
            .unwrap()
            .into_iter()
            .map(|h| (h.fingerprint, h.hypothesis_id))
            .collect();
        classify(
            &input.fingerprint,
            "CWE-639",
            "src/routes.rs",
            "login",
            &findings,
            &dismissed,
            &earlier,
        )
    }

    #[tokio::test]
    async fn r05_r08_r18_investigation_flow_across_two_audits() {
        let (services, clock) = services().await;
        services.register_app(new_app("abastevo")).await.unwrap();
        let first = running_audit(&services).await;
        services.record_map(&first, new_map()).await.unwrap();

        let confirmed_input = new_hypothesis("let order = load(id);", "V8.2.2");
        let outcome = classify_for(&services, &first, &confirmed_input).await;
        let confirmed = services
            .record_hypothesis(&first, confirmed_input, &outcome)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            confirmed.hypothesis_id.as_str(),
            "AUD-ABASTEVO-20261112-0900-H-001"
        );
        let dismissed_input = new_hypothesis("render(name)", "V1.2.1");
        let outcome = classify_for(&services, &first, &dismissed_input).await;
        let dismissed = services
            .record_hypothesis(&first, dismissed_input.clone(), &outcome)
            .await
            .unwrap()
            .unwrap();
        let outcome = classify_for(&services, &first, &dismissed_input).await;
        assert!(
            services
                .record_hypothesis(&first, dismissed_input.clone(), &outcome)
                .await
                .unwrap()
                .is_none(),
            "duplicate in audit"
        );

        let (hypothesis, finding) = services
            .record_verification(&confirmed.hypothesis_id, verification(Verdict::Confirmed))
            .await
            .unwrap();
        let finding = finding.unwrap();
        assert_eq!(finding.finding_id.as_str(), "ABASTEVO-SEC-0001");
        assert_eq!(
            hypothesis.verification.unwrap().evidence[0]
                .evidence_id
                .as_ref()
                .unwrap()
                .as_str(),
            "AUD-ABASTEVO-20261112-0900-EV-001"
        );
        services
            .record_verification(&dismissed.hypothesis_id, verification(Verdict::Dismissed))
            .await
            .unwrap();

        let (decision, final_severity) = SkepticReview::decide(
            false,
            Severity::High,
            Severity::High,
            SeverityAdjustments {
                requires_compromised_admin: true,
                ..Default::default()
            },
        );
        let review = SkepticReview {
            decision,
            base_severity: Severity::High,
            adjustments: SeverityAdjustments {
                requires_compromised_admin: true,
                ..Default::default()
            },
            final_severity,
            arguments_considered: vec![],
            justification: "x".into(),
            model: "muse-spark-1.3-contributor".into(),
            prompt_version: "skeptic@2.0.0".into(),
        };
        let reviewed = services
            .record_review(&finding.finding_id, review)
            .await
            .unwrap();
        assert_eq!(
            (reviewed.state, reviewed.severity),
            (FindingState::Downgraded, Severity::Medium)
        );

        services
            .record_hunt_round(
                &first.id,
                &HuntRound {
                    round: 1,
                    entry_points: vec!["EP-001".into()],
                    requirements_checked: vec!["V6.2.1".into(), "V8.2.2".into()],
                    scanner_triage: vec![ScannerTriage {
                        result_id: "semgrep:1".into(),
                        decision: opentatu_core::TriageDecision::Noise,
                        reason: "x".into(),
                    }],
                    new_hypotheses: 2,
                    strong_hypotheses: 2,
                    gaps: vec![],
                    not_analyzed: vec![],
                    model: "deepseek-v4.1-flash".into(),
                    prompt_version: "hunter@2.0.0".into(),
                },
            )
            .await
            .unwrap();
        let coverage = services
            .coverage_of_audit(&first.id)
            .await
            .unwrap()
            .unwrap();
        let status = |id: &str| {
            coverage
                .requirements
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.status)
        };
        assert_eq!(status("V6.2.1"), Some(RequirementStatus::Verified));
        assert_eq!(
            status("V13.1.1"),
            Some(RequirementStatus::NotTested),
            "applicable, never checked"
        );
        services
            .finish_audit(&first.id, AuditStatus::Partial, &[])
            .await
            .unwrap();

        // Second audit of the same code: the finding persists, the false positive stays closed.
        clock.advance_seconds(3600);
        let second = running_audit(&services).await;
        assert_eq!(second.previous_audit_id.as_ref(), Some(&first.id));
        let again = new_hypothesis("let order = load(id);", "V8.2.2");
        let outcome = classify_for(&services, &second, &again).await;
        assert_eq!(
            outcome,
            DedupOutcome::Persistent(finding.finding_id.clone())
        );
        services
            .record_hypothesis(&second, again, &outcome)
            .await
            .unwrap();
        let outcome = classify_for(&services, &second, &dismissed_input).await;
        assert!(
            matches!(outcome, DedupOutcome::KnownFalsePositive(_)),
            "R08"
        );
        let recorded = services
            .record_hypothesis(&second, dismissed_input, &outcome)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recorded.state, FindingState::Dismissed);

        let comparison = services.comparison_of_audit(&second).await.unwrap();
        assert_eq!(comparison.persistent, vec![finding.finding_id.clone()]);
        assert!(comparison.new.is_empty());
        let timeline = services
            .finding_timeline(&finding.finding_id)
            .await
            .unwrap();
        let kinds: Vec<_> = timeline.iter().map(|e| e.content.kind).collect();
        assert_eq!(
            kinds,
            [
                EventKind::FindingConfirmed,
                EventKind::FindingDowngraded,
                EventKind::FindingConfirmed
            ]
        );
    }
}
