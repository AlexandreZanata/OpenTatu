//! Owner-scoped lab approval and transactional attempt history (R02–R05, R10; ADR-0034).
use crate::{Error, NewAudit, Services};
use opentatu_core::{
    Actor, Audit, AuditId, AuditStatus, EventKind, LabContext, LabExecution, LabOutcome, LabPolicy,
    LabRun, NewEvent,
};

impl Services {
    /// Atomically queues an owner audit and approves its exact commit/profile envelope.
    pub async fn create_lab_audit(
        &self,
        input: NewAudit,
        policy: &LabPolicy,
    ) -> Result<Audit, Error> {
        if input.requested_by != Actor::Owner
            || input.kind == opentatu_core::AuditKind::Reverification
        {
            return Err(Error::Conflict(
                "only the owner may create an approved lab audit".into(),
            ));
        }
        let mut tx = self.db.begin().await?;
        let audit = self.insert_audit(&mut tx, input).await?;
        self.approve_lab_in(&mut tx, &audit, policy).await?;
        tx.commit().await?;
        Ok(audit)
    }

    /// Approves a queued audit, including an MCP-submitted fix's exact new commit.
    pub async fn approve_lab_audit(&self, id: &AuditId, policy: &LabPolicy) -> Result<(), Error> {
        let mut tx = self.db.begin().await?;
        let audit = opentatu_db::audits::get(&mut tx, id)
            .await?
            .ok_or_else(|| Error::UnknownAudit(id.clone()))?;
        self.approve_lab_in(&mut tx, &audit, policy).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn approve_lab_in(
        &self,
        tx: &mut opentatu_db::SqliteConnection,
        audit: &Audit,
        policy: &LabPolicy,
    ) -> Result<(), Error> {
        let digest = &policy.profile_digest;
        if audit.status != AuditStatus::Created
            || audit.kind == opentatu_core::AuditKind::Pr
            || !(1..=8).contains(&policy.max_runs)
            || policy.cases.is_empty()
            || digest.parse::<opentatu_core::Fingerprint>().is_err()
        {
            return Err(Error::Conflict(
                "lab approval requires a queued audit and a SHA-256 profile digest".into(),
            ));
        }
        for case in &policy.cases {
            case.validate().map_err(Error::Conflict)?;
        }
        let event = NewEvent::new(EventKind::OwnerDecision, Actor::Owner, self.now()).with_audit(audit.id.clone())
            .with_target(&audit.id).with_data(serde_json::json!({"action":"approve_lab", "commit":audit.commit, "profile_digest":digest, "image_digest":policy.image_digest,"runner_digest":policy.runner_digest,"policy":"offline-qemu-get-cases-v1", "max_runs":policy.max_runs, "cases":policy.cases.iter().map(|c|serde_json::json!({"id":c.id,"file":c.file,"symbol":c.symbol,"cwes":c.cwes})).collect::<Vec<_>>()}));
        opentatu_db::events::append(tx, &event).await?;
        Ok(())
    }

    /// Current approval for this exact profile, commit and audit. No inherited grants.
    pub async fn lab_approved(&self, id: &AuditId, digest: &str) -> Result<bool, Error> {
        let mut conn = self.db.connection().await?;
        let audit = opentatu_db::audits::get(&mut conn, id)
            .await?
            .ok_or_else(|| Error::UnknownAudit(id.clone()))?;
        approved(&mut conn, &audit, digest).await
    }

    /// Starts only approved work, counting unfinished attempts against the same cap.
    pub async fn start_lab_run(&self, context: LabContext, max_runs: u32) -> Result<LabRun, Error> {
        let mut tx = self.db.begin().await?;
        let audit = opentatu_db::audits::get(&mut tx, &context.audit_id)
            .await?
            .ok_or_else(|| Error::UnknownAudit(context.audit_id.clone()))?;
        let hypothesis = opentatu_db::investigation::hypothesis(&mut tx, &context.hypothesis_id)
            .await?
            .ok_or_else(|| Error::UnknownHypothesis(context.hypothesis_id.clone()))?;
        if audit.status != AuditStatus::Running
            || audit.commit != context.commit
            || hypothesis.audit_id.slug() != &audit.app
            || !approved(&mut tx, &audit, &context.profile_digest).await?
        {
            return Err(Error::Conflict("lab execution requires matching owner approval, app, running audit and exact commit".into()));
        }
        let grant = approval(&mut tx, &audit)
            .await?
            .ok_or_else(|| Error::Conflict("no owner lab grant".into()))?;
        let matches = grant["cases"].as_array().is_some_and(|cases| {
            cases.iter().any(|c| {
                c["id"] == context.case_id.as_str()
                    && c["file"] == hypothesis.content.location.file
                    && c["symbol"] == hypothesis.content.location.symbol
                    && c["cwes"]
                        .as_array()
                        .is_some_and(|v| v.iter().any(|c| c == &hypothesis.content.cwe))
            })
        });
        if !matches || grant["max_runs"].as_u64() != Some(u64::from(max_runs)) {
            return Err(Error::Conflict(
                "case does not match approved source/CWE or attempt cap".into(),
            ));
        }
        let count = opentatu_db::lab::count(&mut tx, &audit.id).await?;
        if count >= max_runs || max_runs > 8 {
            return Err(Error::Conflict("lab attempt budget exhausted".into()));
        }
        let run = LabRun {
            number: count + 1,
            context,
            started_at: self.now(),
            completed_at: None,
            execution: None,
        };
        let event = NewEvent::new(EventKind::LabTestStarted, Actor::System, run.started_at)
            .with_audit(audit.id)
            .with_target(&run.context.hypothesis_id)
            .with_data(serde_json::json!(&run));
        opentatu_db::lab::insert(&mut tx, &run, &event).await?;
        tx.commit().await?;
        Ok(run)
    }

    /// Completes once, recomputing the outcome independently of the guest/model claim.
    pub async fn complete_lab_run(
        &self,
        audit: &AuditId,
        number: u32,
        mut execution: LabExecution,
    ) -> Result<LabRun, Error> {
        let mut tx = self.db.begin().await?;
        let mut run = opentatu_db::lab::list(&mut tx, audit)
            .await?
            .into_iter()
            .find(|r| r.number == number && r.completed_at.is_none())
            .ok_or_else(|| Error::Conflict("no undecided lab attempt".into()))?;
        execution.outcome = match (&execution.control, &execution.subject) {
            (Some(control), Some(subject)) => LabOutcome::from_observations(control, subject),
            _ => LabOutcome::Inconclusive,
        };
        let audit_record = opentatu_db::audits::get(&mut tx, audit)
            .await?
            .ok_or_else(|| Error::UnknownAudit(audit.clone()))?;
        let grant = approval(&mut tx, &audit_record).await?;
        if !grant.is_some_and(|g| {
            g["profile_digest"] == run.context.profile_digest
                && g["image_digest"] == execution.image_digest
                && g["runner_digest"] == execution.runner_digest
        }) || execution
            .source_digest
            .parse::<opentatu_core::Fingerprint>()
            .is_err()
        {
            execution.outcome = LabOutcome::Inconclusive;
            execution
                .note
                .push_str(" Lab receipt provenance failed the persistence gate.");
        }
        run.completed_at = Some(self.now());
        run.execution = Some(execution);
        let event = NewEvent::new(EventKind::LabTestCompleted, Actor::System, self.now())
            .with_audit(audit.clone())
            .with_target(&run.context.hypothesis_id)
            .with_data(serde_json::json!(&run));
        opentatu_db::lab::complete(&mut tx, &run, &event).await?;
        tx.commit().await?;
        Ok(run)
    }

    /// Dated attempts of an audit, including unavailable/interrupted results (R14).
    pub async fn lab_runs(&self, id: &AuditId) -> Result<Vec<LabRun>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::lab::list(&mut conn, id).await?)
    }
}

async fn approval(
    conn: &mut opentatu_db::SqliteConnection,
    audit: &Audit,
) -> Result<Option<serde_json::Value>, Error> {
    Ok(opentatu_db::events::list_by_audit(conn, &audit.id)
        .await?
        .into_iter()
        .rev()
        .find(|e| {
            e.content.kind == EventKind::OwnerDecision && e.content.data["action"] == "approve_lab"
        })
        .filter(|e| {
            e.content.actor == Actor::Owner && e.content.data["commit"] == audit.commit.as_str()
        })
        .map(|e| e.content.data))
}
async fn approved(
    conn: &mut opentatu_db::SqliteConnection,
    audit: &Audit,
    digest: &str,
) -> Result<bool, Error> {
    Ok(approval(conn, audit)
        .await?
        .is_some_and(|v| v["profile_digest"] == digest))
}

/// Persistence gate: dynamic-looking model text cannot substitute a completed VM row.
pub(crate) async fn enforce_proof(
    conn: &mut opentatu_db::SqliteConnection,
    audit: &Audit,
    hypothesis: Option<&opentatu_core::HypothesisId>,
    finding: Option<&opentatu_core::FindingId>,
    verification: &mut opentatu_core::Verification,
) -> Result<(), Error> {
    use opentatu_core::Verdict;
    let expected = match verification.verdict {
        Verdict::Confirmed => Some(LabOutcome::Reproduced),
        Verdict::Dismissed if finding.is_some() => Some(LabOutcome::NotReproduced),
        _ => None,
    };
    let Some(expected) = expected else {
        return Ok(());
    };
    let grant = approval(conn, audit).await?;
    let app = opentatu_db::apps::get(&mut *conn, &audit.app)
        .await?
        .ok_or_else(|| Error::UnknownApp(audit.app.clone()))?;
    if grant.is_none() && app.profile.lab_url.is_none() {
        return Ok(());
    }
    let mut valid = false;
    if let (Some(grant), Some(number)) = (grant, verification.lab_run_number)
        && let Some(run) = opentatu_db::lab::list(conn, &audit.id)
            .await?
            .into_iter()
            .find(|r| r.number == number)
    {
        let subject =
            opentatu_db::investigation::hypothesis(&mut *conn, &run.context.hypothesis_id).await?;
        let same_subject = hypothesis.is_some_and(|id| id == &run.context.hypothesis_id)
            || finding
                .is_some_and(|id| subject.as_ref().and_then(|h| h.finding_id.as_ref()) == Some(id));
        valid = same_subject
            && run.completed_at.is_some()
            && run.context.commit == audit.commit
            && grant["profile_digest"] == run.context.profile_digest
            && run.execution.as_ref().is_some_and(|e| {
                e.outcome == expected && !e.source_digest.is_empty() && !e.runner_digest.is_empty()
            });
    }
    if !valid {
        verification.verdict = Verdict::Inconclusive;
        verification.missing_to_conclude="Falta resultado dinâmico executado e aprovado para a mesma hipótese, auditoria, commit e perfil.".into();
        verification
            .system_notes
            .push(verification.missing_to_conclude.clone());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::tests::{new_app, services};
    use crate::investigation::tests::{new_hypothesis, verification};
    use opentatu_core::{AuditKind, Clock as _, LabCase, LabObservation, Verdict};
    use opentatu_memory::DedupOutcome;

    fn policy() -> LabPolicy {
        let digest = format!("sha256:{}", "a".repeat(64));
        LabPolicy {
            port: 8080,
            profile_digest: digest.clone(),
            image_digest: digest.clone(),
            runner_digest: digest,
            max_runs: 1,
            timeout_seconds: 120,
            source_paths: vec!["src".into()],
            cases: vec![LabCase {
                id: "account".parse().unwrap(),
                path: "/accounts/b".into(),
                file: "src/routes.rs".into(),
                symbol: "login".into(),
                cwes: vec!["CWE-639".into()],
                control_actor: "owner-b".parse().unwrap(),
                subject_actor: "stranger-a".parse().unwrap(),
                marker: "FAKE_B".into(),
            }],
        }
    }
    fn new_audit() -> NewAudit {
        NewAudit {
            app: "abastevo".parse().unwrap(),
            kind: AuditKind::Full,
            commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
            branch: None,
            requested_by: Actor::Owner,
        }
    }
    #[tokio::test]
    async fn r02_r03_r04_r10_approval_attempt_cap_and_decided_once() {
        let (services, clock) = services().await;
        services.register_app(new_app("abastevo")).await.unwrap();
        let policy = policy();
        let audit = services
            .create_lab_audit(new_audit(), &policy)
            .await
            .unwrap();
        assert!(
            services
                .lab_approved(&audit.id, &policy.profile_digest)
                .await
                .unwrap()
        );
        assert!(!services.lab_approved(&audit.id, "changed").await.unwrap());
        let audit = services.start_next_audit().await.unwrap().unwrap();
        assert!(
            services
                .approve_lab_audit(&audit.id, &policy)
                .await
                .is_err()
        );
        let hypothesis = services
            .record_hypothesis(
                &audit,
                new_hypothesis("load(id)", "V8.2.2"),
                &DedupOutcome::New {
                    possibly_same_as: None,
                },
            )
            .await
            .unwrap()
            .unwrap();
        let context = LabContext {
            audit_id: audit.id.clone(),
            hypothesis_id: hypothesis.hypothesis_id.clone(),
            commit: audit.commit.clone(),
            profile_digest: policy.profile_digest.clone(),
            case_id: "account".parse().unwrap(),
        };
        let mut wrong = context.clone();
        wrong.commit = "0".repeat(40).parse().unwrap();
        assert!(services.start_lab_run(wrong, 1).await.is_err());
        assert!(services.start_lab_run(context.clone(), 2).await.is_err());
        wrong = context.clone();
        wrong.case_id = "unapproved".parse().unwrap();
        assert!(services.start_lab_run(wrong, 1).await.is_err());
        let run = services.start_lab_run(context.clone(), 1).await.unwrap();
        assert_eq!(run.started_at, clock.now());
        assert!(
            services.start_lab_run(context, 1).await.is_err(),
            "unfinished attempts consume the cap"
        );
        let observation = LabObservation {
            status: 200,
            marker_found: true,
            bytes: 6,
            body_digest: policy.image_digest.clone(),
        };
        let execution = LabExecution {
            outcome: LabOutcome::NotReproduced,
            control: Some(observation.clone()),
            subject: Some(observation),
            image_digest: policy.image_digest.clone(),
            source_digest: policy.image_digest.clone(),
            runner_digest: policy.runner_digest.clone(),
            zap: None,
            note: String::new(),
        };
        let completed = services
            .complete_lab_run(&audit.id, 1, execution.clone())
            .await
            .unwrap();
        assert_eq!(
            completed.execution.unwrap().outcome,
            LabOutcome::Reproduced,
            "host recomputes the verdict"
        );
        assert!(
            services
                .complete_lab_run(&audit.id, 1, execution)
                .await
                .is_err()
        );
        let mut verdict = verification(Verdict::Confirmed);
        verdict.lab_run_number = Some(99);
        let (inconclusive, finding) = services
            .record_verification(&hypothesis.hypothesis_id, verdict)
            .await
            .unwrap();
        assert_eq!(
            inconclusive.verification.unwrap().verdict,
            Verdict::Inconclusive
        );
        assert!(finding.is_none(), "model text cannot invent a receipt");
        let mut verdict = verification(Verdict::Confirmed);
        verdict.lab_run_number = Some(1);
        let (_, finding) = services
            .record_verification(&hypothesis.hypothesis_id, verdict)
            .await
            .unwrap();
        assert!(finding.is_some());
        assert_eq!(services.lab_runs(&audit.id).await.unwrap().len(), 1);
        assert_eq!(
            services
                .audit_events(&audit.id)
                .await
                .unwrap()
                .iter()
                .filter(|e| e.content.kind == EventKind::LabTestCompleted)
                .count(),
            1
        );
    }
    #[tokio::test]
    async fn r10_approval_is_never_inherited_and_only_owner_can_grant() {
        let (services, _) = services().await;
        services.register_app(new_app("abastevo")).await.unwrap();
        let mut input = new_audit();
        input.requested_by = Actor::System;
        assert!(services.create_lab_audit(input, &policy()).await.is_err());
        let first = services
            .create_lab_audit(new_audit(), &policy())
            .await
            .unwrap();
        let second = services.create_audit(new_audit()).await.unwrap();
        assert_ne!(first.id, second.id);
        assert!(
            !services
                .lab_approved(&second.id, &policy().profile_digest)
                .await
                .unwrap()
        );
    }
}
