//! Use cases: audit creation, queue and lifecycle (spec §A.5, §B.2, §C.7; R02, R03, R13).
//!
//! The audits table is the worker queue (ADR-0021): `create_audit` enqueues (`CREATED`),
//! `start_next_audit` claims the oldest queued audit (`RUNNING`), and the pipeline ends it
//! with `finish_audit` or `fail_audit`. Every transition is checked by
//! `AuditStatus::validate_transition` and recorded as an event in the same transaction.

use opentatu_core::{
    Actor, Audit, AuditId, AuditKind, AuditStatus, CommitHash, EventKind, NewEvent, Slug,
};

use crate::{Error, Services};

/// Upper bound of same-minute collisions tried for the audit ID suffix (spec §B.2).
const MAX_SEQUENCE: u32 = 1_000;

/// Input of [`Services::create_audit`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAudit {
    /// App to audit (must be registered, R01).
    pub app: Slug,
    /// Audit kind.
    pub kind: AuditKind,
    /// Exact commit, already resolved (R02).
    pub commit: CommitHash,
    /// Branch the commit was resolved from, if any.
    pub branch: Option<String>,
    /// Who asked for it (owner, scheduler, or a fixing agent for reverifications).
    pub requested_by: Actor,
}

impl Services {
    /// Creates and enqueues an audit. The ID is stamped with `created_at` (Q-001), with a
    /// `-N` suffix on same-minute collisions. Emits `AUDIT_CREATED`.
    /// A REVERIFICATION is queued only by [`Self::submit_fix`], with the fix it checks.
    pub async fn create_audit(&self, input: NewAudit) -> Result<Audit, Error> {
        if input.kind == AuditKind::Reverification {
            return Err(Error::Conflict(
                "a reverification is queued by submitting a fix (opentatu_submit_fix)".into(),
            ));
        }
        let mut tx = self.db.begin().await?;
        let audit = self.insert_audit(&mut tx, input).await?;
        tx.commit().await?;
        Ok(audit)
    }

    /// [`Self::create_audit`] inside a caller's transaction (a submitted fix enqueues its
    /// reverification together with `FIX_SUBMITTED`).
    pub(crate) async fn insert_audit(
        &self,
        tx: &mut opentatu_db::SqliteConnection,
        input: NewAudit,
    ) -> Result<Audit, Error> {
        let now = self.clock.now();
        if opentatu_db::apps::get(&mut *tx, &input.app)
            .await?
            .is_none()
        {
            return Err(Error::UnknownApp(input.app));
        }
        let mut sequence = 1;
        let id = loop {
            let candidate = AuditId::new(&input.app, &now, sequence);
            if !opentatu_db::audits::exists(&mut *tx, &candidate).await? {
                break candidate;
            }
            sequence += 1;
            if sequence > MAX_SEQUENCE {
                return Err(Error::Conflict(
                    "too many audits created in the same minute".into(),
                ));
            }
        };
        let audit = Audit {
            id: id.clone(),
            app: input.app,
            kind: input.kind,
            status: AuditStatus::Created,
            commit: input.commit,
            branch: input.branch,
            previous_audit_id: opentatu_db::audits::last_final(&mut *tx, id.slug()).await?,
            created_at: now,
            started_at: None,
            completed_at: None,
            checkpoint: None,
            limitations: vec![],
        };
        let event = NewEvent::new(EventKind::AuditCreated, input.requested_by, now)
            .with_audit(id.clone())
            .with_target(&id)
            .with_data(serde_json::json!({
                "kind": audit.kind,
                "commit": audit.commit,
                "branch": audit.branch,
                "previous_audit_id": audit.previous_audit_id,
            }));
        opentatu_db::audits::insert(&mut *tx, &audit, &event).await?;
        Ok(audit)
    }

    /// Claims the oldest queued audit (`CREATED → RUNNING`, `AUDIT_STARTED`), if any.
    pub async fn start_next_audit(&self) -> Result<Option<Audit>, Error> {
        let mut tx = self.db.begin().await?;
        let Some(queued) = opentatu_db::audits::next_queued(&mut tx).await? else {
            return Ok(None);
        };
        let now = self.clock.now();
        let event = NewEvent::new(EventKind::AuditStarted, Actor::System, now)
            .with_audit(queued.id.clone())
            .with_target(&queued.id);
        let running = opentatu_db::audits::update_status(
            &mut tx,
            &queued.id,
            AuditStatus::Running,
            &now,
            &[],
            &event,
        )
        .await?;
        tx.commit().await?;
        Ok(Some(running))
    }

    /// Records that the audit's commit was checked out (`SNAPSHOT_CAPTURED`, R02).
    pub async fn record_snapshot(&self, audit: &Audit, reused: bool) -> Result<(), Error> {
        let now = self.clock.now();
        let event = NewEvent::new(EventKind::SnapshotCaptured, Actor::System, now)
            .with_audit(audit.id.clone())
            .with_target(&audit.commit)
            .with_data(serde_json::json!({ "commit": audit.commit, "reused": reused }));
        let mut tx = self.db.begin().await?;
        opentatu_db::snapshots::record(&mut tx, &audit.app, &audit.commit, &now, &event).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Stores the last pipeline step completed (for resuming; not an event).
    pub async fn set_audit_checkpoint(&self, id: &AuditId, step: &str) -> Result<(), Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::audits::set_checkpoint(&mut conn, id, step).await?)
    }

    /// Ends a running audit as `COMPLETED` or `PARTIAL` with its limitations (R13, R14).
    /// A `PARTIAL` audit is never reported as complete.
    pub async fn finish_audit(
        &self,
        id: &AuditId,
        status: AuditStatus,
        limitations: &[String],
    ) -> Result<Audit, Error> {
        let kind = match status {
            AuditStatus::Completed => EventKind::AuditCompleted,
            AuditStatus::Partial => EventKind::AuditPartial,
            other => {
                return Err(Error::Conflict(format!(
                    "{other} is not a way to finish an audit"
                )));
            }
        };
        let data = serde_json::json!({ "limitations": limitations });
        self.transition(id, status, kind, limitations, data).await
    }

    /// Ends an audit as `FAILED` with the reason (`AUDIT_FAILED`).
    pub async fn fail_audit(&self, id: &AuditId, reason: &str) -> Result<Audit, Error> {
        let data = serde_json::json!({ "reason": reason });
        self.transition(id, AuditStatus::Failed, EventKind::AuditFailed, &[], data)
            .await
    }

    /// The audit with `id`.
    pub async fn audit(&self, id: &AuditId) -> Result<Audit, Error> {
        let mut conn = self.db.connection().await?;
        opentatu_db::audits::get(&mut conn, id)
            .await?
            .ok_or_else(|| Error::UnknownAudit(id.clone()))
    }

    /// Audits of an app, newest first.
    pub async fn audits_of_app(&self, app: &Slug, limit: u32) -> Result<Vec<Audit>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::audits::list_by_app(&mut conn, app, limit).await?)
    }

    /// Audits of every app, newest first.
    pub async fn recent_audits(&self, limit: u32) -> Result<Vec<Audit>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::audits::list_recent(&mut conn, limit).await?)
    }

    async fn transition(
        &self,
        id: &AuditId,
        to: AuditStatus,
        kind: EventKind,
        limitations: &[String],
        data: serde_json::Value,
    ) -> Result<Audit, Error> {
        let now = self.clock.now();
        let event = NewEvent::new(kind, Actor::System, now)
            .with_audit(id.clone())
            .with_target(id)
            .with_data(data);
        let mut tx = self.db.begin().await?;
        let audit =
            opentatu_db::audits::update_status(&mut tx, id, to, &now, limitations, &event).await?;
        tx.commit().await?;
        Ok(audit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::tests::{new_app, services};

    const COMMIT: &str = "4c4a5df92ace8d8068e57362a7308e8b9d031b3a";

    fn new_audit(app: &str) -> NewAudit {
        NewAudit {
            app: Slug::new(app).unwrap(),
            kind: AuditKind::Full,
            commit: COMMIT.parse().unwrap(),
            branch: Some("main".into()),
            requested_by: Actor::Owner,
        }
    }

    #[tokio::test]
    async fn r03_audit_id_is_stamped_with_created_at_and_suffixed_on_collision() {
        let (services, clock) = services().await;
        services.register_app(new_app("abastevo")).await.unwrap();
        let first = services.create_audit(new_audit("abastevo")).await.unwrap();
        let second = services.create_audit(new_audit("abastevo")).await.unwrap();
        assert_eq!(first.id.as_str(), "AUD-ABASTEVO-20261112-0900");
        assert_eq!(second.id.as_str(), "AUD-ABASTEVO-20261112-0900-2");
        assert_eq!(first.created_at.iso(), "2026-11-12T09:00:00-03:00");
        clock.advance_seconds(60);
        let third = services.create_audit(new_audit("abastevo")).await.unwrap();
        assert_eq!(third.id.as_str(), "AUD-ABASTEVO-20261112-0901");
    }

    #[tokio::test]
    async fn r01_unregistered_apps_cannot_be_audited() {
        let (services, _) = services().await;
        assert!(matches!(
            services.create_audit(new_audit("ghost")).await,
            Err(Error::UnknownApp(_))
        ));
    }

    #[tokio::test]
    async fn r06_reverifications_come_only_from_a_submitted_fix() {
        let (services, _) = services().await;
        services.register_app(new_app("abastevo")).await.unwrap();
        let mut input = new_audit("abastevo");
        input.kind = AuditKind::Reverification;
        assert!(matches!(
            services.create_audit(input).await,
            Err(Error::Conflict(_))
        ));
    }

    #[tokio::test]
    async fn queue_runs_audits_in_order_and_records_every_step() {
        let (services, clock) = services().await;
        services.register_app(new_app("abastevo")).await.unwrap();
        let created = services.create_audit(new_audit("abastevo")).await.unwrap();

        clock.advance_seconds(5);
        let running = services.start_next_audit().await.unwrap().unwrap();
        assert_eq!(running.id, created.id);
        assert_eq!(running.status, AuditStatus::Running);
        assert!(
            services.start_next_audit().await.unwrap().is_none(),
            "queue is empty"
        );

        services.record_snapshot(&running, false).await.unwrap();
        services
            .set_audit_checkpoint(&running.id, "SNAPSHOT")
            .await
            .unwrap();
        clock.advance_seconds(10);
        let limits = vec!["Scanners estáticos ainda não executados.".to_owned()];
        let done = services
            .finish_audit(&running.id, AuditStatus::Partial, &limits)
            .await
            .unwrap();
        assert_eq!(done.duration_seconds(), Some(10));
        assert_eq!(done.limitations, limits);

        let kinds: Vec<EventKind> = services
            .audit_events(&created.id)
            .await
            .unwrap()
            .into_iter()
            .map(|event| event.content.kind)
            .collect();
        assert_eq!(
            kinds,
            [
                EventKind::AuditCreated,
                EventKind::AuditStarted,
                EventKind::SnapshotCaptured,
                EventKind::AuditPartial
            ]
        );
        let next = services.create_audit(new_audit("abastevo")).await.unwrap();
        assert_eq!(next.previous_audit_id, Some(created.id));
    }

    #[tokio::test]
    async fn r13_finished_audits_cannot_restart_or_fail() {
        let (services, _) = services().await;
        services.register_app(new_app("abastevo")).await.unwrap();
        let audit = services.create_audit(new_audit("abastevo")).await.unwrap();
        services.start_next_audit().await.unwrap();
        services
            .fail_audit(&audit.id, "git clone failed")
            .await
            .unwrap();
        assert!(services.fail_audit(&audit.id, "again").await.is_err());
        assert!(
            services
                .finish_audit(&audit.id, AuditStatus::Running, &[])
                .await
                .is_err()
        );
    }
}
