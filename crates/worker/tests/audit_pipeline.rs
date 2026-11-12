//! End-to-end tests of the audit pipeline: register → create → snapshot → scanners (fake
//! suite; real containers are tested in `opentatu-scanners`) → artifacts.

// Integration-test helpers are not `#[test]` functions, so clippy's test exemption does not
// reach them; failing fast is what a test helper should do.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use opentatu_core::{
    Actor, Audit, AuditKind, AuditStatus, DataLayout, EventKind, FixedClock, NewApp, ScannerKind,
    Slug,
};
use opentatu_scanners::testing::{FakeOutcome, FakeSuite};
use opentatu_services::{NewAudit, Services};
use opentatu_snapshots::{Snapshots, testing};
use opentatu_worker::{Worker, pipeline::PENDING_STEPS};

struct Setup {
    _source: tempfile::TempDir,
    data: tempfile::TempDir,
    services: Services,
    snapshots: Snapshots,
    worker: Worker,
}

async fn setup(repo_url: Option<&str>, scanners: FakeSuite) -> Setup {
    let source = tempfile::tempdir().unwrap();
    let url = match repo_url {
        Some(url) => url.to_owned(),
        None => testing::create_repo(
            source.path(),
            &[("src/route.ts", "export const GET = 1;\n")],
        ),
    };
    let data = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(data.path());
    let clock = Arc::new(FixedClock::at("2026-11-12T09:00:00-03:00").unwrap());
    let services = Services::in_memory(clock).await.unwrap();
    services
        .register_app(NewApp {
            slug: Slug::new("demo").unwrap(),
            name: "Demo".into(),
            repo_url: url,
            branch: "main".into(),
            stack: vec!["TypeScript".into()],
            sensitive_data: vec![],
            lab_url: None,
        })
        .await
        .unwrap();
    let snapshots = Snapshots::new(layout.clone());
    let worker = Worker::new(
        services.clone(),
        snapshots.clone(),
        Arc::new(scanners),
        layout,
    );
    Setup {
        _source: source,
        data,
        services,
        snapshots,
        worker,
    }
}

/// Enqueues a FULL audit of `main`.
async fn enqueue(setup: &Setup) -> Audit {
    let slug = Slug::new("demo").unwrap();
    setup
        .snapshots
        .refresh_mirror(
            &slug,
            &setup.services.app(&slug).await.unwrap().profile.repo_url,
        )
        .await
        .unwrap();
    let commit = setup.snapshots.resolve(&slug, "main").await.unwrap();
    setup
        .services
        .create_audit(NewAudit {
            app: slug,
            kind: AuditKind::Full,
            commit,
            branch: Some("main".into()),
            requested_by: Actor::Owner,
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn clean_audit_ends_partial_with_events_and_artifacts() {
    let setup = setup(None, FakeSuite::clean()).await;
    let slug = Slug::new("demo").unwrap();
    let audit = enqueue(&setup).await;
    assert_eq!(audit.id.as_str(), "AUD-DEMO-20261112-0900");

    let ran = setup.worker.drain().await.unwrap();
    assert_eq!(ran, vec![audit.id.clone()]);

    let finished = setup.services.audit(&audit.id).await.unwrap();
    assert_eq!(
        finished.status,
        AuditStatus::Partial,
        "R13/R14: AI steps not implemented yet"
    );
    assert_eq!(finished.limitations.len(), PENDING_STEPS.len());
    assert_eq!(finished.checkpoint.as_deref(), Some("ARTIFACTS"));

    let kinds: Vec<EventKind> = setup
        .services
        .audit_events(&audit.id)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.content.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            EventKind::AuditCreated,
            EventKind::AuditStarted,
            EventKind::SnapshotCaptured,
            EventKind::ScannerRan,
            EventKind::ScannerRan,
            EventKind::ScannerRan,
            EventKind::ScannerRan,
            EventKind::AuditPartial
        ]
    );

    let layout = DataLayout::new(setup.data.path());
    let memory = std::fs::read_to_string(layout.memory_file(&slug)).unwrap();
    assert!(
        memory.contains("AUD-DEMO-20261112-0900 (FULL, PARTIAL)"),
        "{memory}"
    );
    let record = std::fs::read_to_string(layout.audit_record_file(&audit.id)).unwrap();
    assert!(record.contains("\"status\": \"PARTIAL\""), "{record}");
    let history = std::fs::read_to_string(layout.history_file(&slug)).unwrap();
    assert_eq!(
        history.lines().count(),
        9,
        "APP_REGISTERED + 8 audit events"
    );
    assert!(
        layout
            .snapshot_dir(&slug, &finished.commit)
            .join("src/route.ts")
            .is_file()
    );
    assert!(layout.app_profile_file(&slug).is_file());
    assert!(
        layout
            .scanner_output_dir(&audit.id)
            .join("osv-scanner.json")
            .is_file()
    );
}

#[tokio::test]
async fn scanner_results_are_stored_and_scanners_that_cannot_run_become_limitations() {
    let suite = FakeSuite::clean()
        .with(
            ScannerKind::Semgrep,
            FakeOutcome::Results(vec![
                FakeSuite::result(ScannerKind::Semgrep, 1, "sql-injection", "src/route.ts"),
                FakeSuite::result(ScannerKind::Semgrep, 2, "xss", "src/route.ts"),
            ]),
        )
        .with(ScannerKind::Trivy, FakeOutcome::NotReady)
        .with(ScannerKind::Osv, FakeOutcome::Fails(2));
    let setup = setup(None, suite).await;
    let audit = enqueue(&setup).await;
    setup.worker.drain().await.unwrap();

    let finished = setup.services.audit(&audit.id).await.unwrap();
    assert_eq!(finished.status, AuditStatus::Partial);
    assert_eq!(finished.limitations.len(), PENDING_STEPS.len() + 2);
    assert!(finished.limitations[0].contains("opentatu scanners update"));
    assert!(finished.limitations[1].starts_with("Scanner osv-scanner falhou"));

    let results = setup.services.scanner_results(&audit.id).await.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[1].rule_id, "xss");
    let runs = setup.services.scanner_runs(&audit.id).await.unwrap();
    assert_eq!(runs.len(), 4, "one event per scanner, ran or failed");

    let layout = DataLayout::new(setup.data.path());
    let record: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(layout.audit_record_file(&audit.id)).unwrap(),
    )
    .unwrap();
    assert_eq!(record["scanners"]["semgrep"], "0.0.0-fake");
    assert_eq!(record["scanners"]["gitleaks"], "0.0.0-fake");
    assert!(
        record["scanners"].get("trivy").is_none(),
        "R17: only what ran"
    );
}

#[tokio::test]
async fn unreachable_repository_fails_the_audit_not_the_worker() {
    let setup = setup(
        Some("/nonexistent/repository/for/opentatu-tests"),
        FakeSuite::clean(),
    )
    .await;
    let slug = Slug::new("demo").unwrap();
    let audit = setup
        .services
        .create_audit(NewAudit {
            app: slug,
            kind: AuditKind::Full,
            commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
            branch: None,
            requested_by: Actor::Owner,
        })
        .await
        .unwrap();
    assert_eq!(setup.worker.drain().await.unwrap(), vec![audit.id.clone()]);
    let failed = setup.services.audit(&audit.id).await.unwrap();
    assert_eq!(failed.status, AuditStatus::Failed);
    let last = setup
        .services
        .audit_events(&audit.id)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(last.content.kind, EventKind::AuditFailed);
    assert!(
        last.content.data["reason"]
            .as_str()
            .unwrap()
            .contains("snapshot failed")
    );
}
