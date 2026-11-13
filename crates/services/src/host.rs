//! Owner-only live inventory admission and immutable command history (Q-035; R01/R03/R04).

use opentatu_core::{
    Actor, EventKind, HostCommand, HostCommandRun, HostInspection, HostPolicy, HostResult,
    NewEvent, Slug,
};

use crate::{Error, Services};

impl Services {
    /// One explicit owner invocation authorizes this inspection only. No reusable grant.
    pub async fn start_host_inspection(
        &self,
        app: &Slug,
        policy: HostPolicy,
        actor: &Actor,
    ) -> Result<HostInspection, Error> {
        if *actor != Actor::Owner {
            return Err(Error::Unauthorized);
        }
        let mut tx = self.db.begin().await?;
        if opentatu_db::apps::get(&mut tx, app).await?.is_none() {
            return Err(Error::UnknownApp(app.clone()));
        }
        let inspection = HostInspection {
            app: app.clone(),
            number: opentatu_db::host::next_number(&mut tx, app).await?,
            policy,
            started_at: self.clock.now(),
            completed_at: None,
            commands: vec![],
        };
        let event = NewEvent::new(
            EventKind::HostInspectionStarted,
            Actor::Owner,
            inspection.started_at,
        )
        .with_app(app.clone())
        .with_target(inspection.number)
        .with_data(
            serde_json::json!({ "number": inspection.number, "policy": inspection.policy,
                "commands": HostCommand::ALL, "authorization": "explicit-owner-invocation" }),
        );
        opentatu_db::host::insert(&mut tx, &inspection, &event).await?;
        tx.commit().await?;
        Ok(inspection)
    }

    /// Records a fixed query before any network access. No replay or parallel query.
    pub async fn start_host_command(
        &self,
        inspection: &HostInspection,
        command: HostCommand,
    ) -> Result<(), Error> {
        let mut tx = self.db.begin().await?;
        let current = checked(&mut tx, inspection).await?;
        if HostCommand::ALL.get(current.commands.len()) != Some(&command)
            || current.commands.iter().any(|r| r.completed_at.is_none())
        {
            return Err(Error::Conflict(
                "host queries must start once in allowlist order".into(),
            ));
        }
        let run = HostCommandRun {
            command,
            started_at: self.clock.now(),
            completed_at: None,
            result: None,
        };
        let event = NewEvent::new(EventKind::HostCommandStarted, Actor::System, run.started_at)
            .with_app(current.app.clone()).with_target(current.number)
            .with_data(serde_json::json!({ "number": current.number, "command": command, "request": command.request() }));
        opentatu_db::host::start_command(&mut tx, &current.app, current.number, &run, &event)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Revalidates typed observations independently of the transport before persistence.
    pub async fn complete_host_command(
        &self,
        inspection: &HostInspection,
        command: HostCommand,
        result: HostResult,
    ) -> Result<(), Error> {
        if let HostResult::Observed { observation } = &result
            && (observation.command() != command || !observation.is_valid())
        {
            return Err(Error::Conflict(
                "host observation does not match the bounded query schema".into(),
            ));
        }
        let mut tx = self.db.begin().await?;
        let current = checked(&mut tx, inspection).await?;
        let mut run = current
            .commands
            .iter()
            .find(|r| r.command == command && r.completed_at.is_none())
            .cloned()
            .ok_or_else(|| {
                Error::Conflict("host command was not started or is already complete".into())
            })?;
        let now = self.clock.now();
        run.completed_at = Some(now);
        run.result = Some(result);
        let event = NewEvent::new(EventKind::HostCommandCompleted, Actor::System, now)
            .with_app(current.app.clone())
            .with_target(current.number)
            .with_data(serde_json::json!({ "number": current.number, "run": run }));
        opentatu_db::host::complete_command(&mut tx, &current.app, current.number, &run, &event)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Final collection status is computed, never supplied by a model or SSH peer.
    pub async fn complete_host_inspection(
        &self,
        inspection: &HostInspection,
    ) -> Result<HostInspection, Error> {
        let mut tx = self.db.begin().await?;
        let mut current = checked(&mut tx, inspection).await?;
        if current.commands.len() != HostCommand::ALL.len()
            || current.commands.iter().any(|r| r.completed_at.is_none())
        {
            return Err(Error::Conflict(
                "all host queries must complete or fail before finalization".into(),
            ));
        }
        let now = self.clock.now();
        current.completed_at = Some(now);
        let event = NewEvent::new(EventKind::HostInspectionCompleted, Actor::System, now)
            .with_app(current.app.clone()).with_target(current.number)
            .with_data(serde_json::json!({ "number": current.number, "collection_complete": current.is_success() }));
        opentatu_db::host::complete(&mut tx, &current.app, current.number, now, &event).await?;
        tx.commit().await?;
        Ok(current)
    }

    /// One numbered live inspection, without credentials, raw output or address.
    pub async fn host_inspection(&self, app: &Slug, number: u32) -> Result<HostInspection, Error> {
        let mut conn = self.db.connection().await?;
        opentatu_db::host::get(&mut conn, app, number)
            .await?
            .ok_or_else(|| Error::Conflict("unknown host inspection".into()))
    }

    /// Recent inspections; interrupted attempts remain visible. At most fifty per page.
    pub async fn host_inspections(
        &self,
        app: &Slug,
        limit: u32,
    ) -> Result<Vec<HostInspection>, Error> {
        self.app(app).await?;
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::host::list(&mut conn, app, limit.min(50)).await?)
    }
}

async fn checked(
    conn: &mut opentatu_db::SqliteConnection,
    supplied: &HostInspection,
) -> Result<HostInspection, Error> {
    let current = opentatu_db::host::get(conn, &supplied.app, supplied.number)
        .await?
        .ok_or_else(|| Error::Conflict("unknown host inspection".into()))?;
    if current.policy != supplied.policy || current.completed_at.is_some() {
        return Err(Error::Conflict(
            "host inspection policy differs or is already finalized".into(),
        ));
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::tests::{new_app, services};
    use opentatu_core::{Clock, HostFailure, HostObservation};

    fn policy() -> HostPolicy {
        HostPolicy {
            profile: "test-host".parse().unwrap(),
            profile_digest: format!("sha256:{}", "a".repeat(64)).parse().unwrap(),
        }
    }

    #[tokio::test]
    async fn r01_r03_r04_owner_only_registered_scoped_ordered_dated_history() {
        let (s, clock) = services().await;
        let app = "abastevo".parse().unwrap();
        assert!(matches!(
            s.start_host_inspection(&app, policy(), &Actor::Owner).await,
            Err(Error::UnknownApp(_))
        ));
        s.register_app(new_app("abastevo")).await.unwrap();
        assert!(matches!(
            s.start_host_inspection(
                &app,
                policy(),
                &Actor::Role(opentatu_core::AiRole::Verifier)
            )
            .await,
            Err(Error::Unauthorized)
        ));
        let i = s
            .start_host_inspection(&app, policy(), &Actor::Owner)
            .await
            .unwrap();
        assert_eq!(i.started_at, clock.now());
        assert!(s.complete_host_inspection(&i).await.is_err());
        assert!(s.start_host_command(&i, HostCommand::Kernel).await.is_err());
        let mut forged = i.clone();
        forged.policy.profile = "other-host".parse().unwrap();
        assert!(
            s.start_host_command(&forged, HostCommand::Os)
                .await
                .is_err()
        );
        for c in HostCommand::ALL {
            s.start_host_command(&i, *c).await.unwrap();
            assert!(s.start_host_command(&i, *c).await.is_err());
            if *c == HostCommand::Os {
                let bad = HostResult::Observed {
                    observation: HostObservation::Kernel {
                        release: "6.8.0".into(),
                    },
                };
                assert!(s.complete_host_command(&i, *c, bad).await.is_err());
            }
            s.complete_host_command(
                &i,
                *c,
                HostResult::Failed {
                    failure: HostFailure::Transport,
                },
            )
            .await
            .unwrap();
            assert!(
                s.complete_host_command(
                    &i,
                    *c,
                    HostResult::Failed {
                        failure: HostFailure::Transport
                    }
                )
                .await
                .is_err()
            );
        }
        let completed = s.complete_host_inspection(&i).await.unwrap();
        assert!(!completed.is_success());
        assert_eq!(completed.completed_at, Some(clock.now()));
        assert_eq!(s.host_inspection(&app, 1).await.unwrap(), completed);
        assert!(s.start_host_command(&i, HostCommand::Os).await.is_err());
        assert_eq!(s.host_inspections(&app, 100).await.unwrap(), [completed]);
        let events = s.app_history(&app).await.unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|e| e.content.kind == EventKind::HostCommandStarted)
                .count(),
            3
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| e.content.kind == EventKind::HostCommandCompleted)
                .count(),
            3
        );
        assert!(
            !serde_json::to_string(&events)
                .unwrap()
                .contains("identity_file")
        );
    }

    #[tokio::test]
    async fn r14_interruption_is_visible_and_a_new_invocation_has_a_fresh_number() {
        let (s, _) = services().await;
        s.register_app(new_app("abastevo")).await.unwrap();
        let app = "abastevo".parse().unwrap();
        let i = s
            .start_host_inspection(&app, policy(), &Actor::Owner)
            .await
            .unwrap();
        s.start_host_command(&i, HostCommand::Os).await.unwrap();
        assert!(!s.host_inspection(&app, 1).await.unwrap().is_success());
        assert!(s.start_host_command(&i, HostCommand::Kernel).await.is_err());
        assert_eq!(
            s.start_host_inspection(&app, policy(), &Actor::Owner)
                .await
                .unwrap()
                .number,
            2
        );
    }
}
