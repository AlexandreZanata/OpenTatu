//! Hermetic one-shot inventory and opt-in disposable SSH acceptance (Q-035; R01/R14).
// Helpers intentionally fail fast in tests, never in production code.
#![allow(clippy::unwrap_used)]

use opentatu_core::{
    EventKind, FixedClock, HostCommand, HostObservation, HostPolicy, HostResult, NewApp, Slug,
};
use opentatu_scanners::testing::FakeHosts;
use opentatu_services::Services;
use opentatu_worker::inspect_host;
use std::{collections::BTreeMap, sync::Arc};

async fn services() -> Services {
    let s = Services::in_memory(Arc::new(
        FixedClock::at("2026-11-13T09:00:00-03:00").unwrap(),
    ))
    .await
    .unwrap();
    s.register_app(NewApp {
        slug: "abastevo".parse().unwrap(),
        name: "Abastevo".into(),
        repo_url: "/fixture/abastevo".into(),
        branch: "main".into(),
        stack: vec!["Go".into()],
        sensitive_data: vec![],
        lab_url: None,
    })
    .await
    .unwrap();
    s
}

fn policy() -> HostPolicy {
    HostPolicy {
        profile: "local-host".parse().unwrap(),
        profile_digest: format!("sha256:{}", "a".repeat(64)).parse().unwrap(),
    }
}

#[tokio::test]
async fn r01_r14_host_inventory_is_explicit_scoped_and_separate_from_findings() {
    let s = services().await;
    let app: Slug = "abastevo".parse().unwrap();
    let policy = policy();
    let results = [
        (
            HostCommand::Os,
            HostObservation::Os {
                id: "ubuntu".into(),
                version_id: Some("24.04".into()),
            },
        ),
        (
            HostCommand::Kernel,
            HostObservation::Kernel {
                release: "6.8.0".into(),
            },
        ),
        (
            HostCommand::Listeners,
            HostObservation::Listeners { sockets: vec![] },
        ),
    ]
    .into_iter()
    .map(|(k, observation)| (k, HostResult::Observed { observation }))
    .collect();
    let hosts = FakeHosts::new(app.clone(), policy.clone(), results);
    assert!(hosts.calls().is_empty());
    assert!(
        inspect_host(&s, &hosts, &app, &"other-host".parse().unwrap())
            .await
            .is_err()
    );
    assert!(
        inspect_host(&s, &hosts, &"another-app".parse().unwrap(), &policy.profile)
            .await
            .is_err()
    );
    assert!(hosts.calls().is_empty());
    let i = inspect_host(&s, &hosts, &app, &policy.profile)
        .await
        .unwrap();
    assert!(i.is_success());
    assert_eq!(hosts.calls(), HostCommand::ALL);
    assert_eq!(s.host_inspection(&app, 1).await.unwrap(), i);
    assert!(s.findings_of_app(&app).await.unwrap().is_empty());
    assert!(s.audits_of_app(&app, 10).await.unwrap().is_empty());
    let history = s.app_history(&app).await.unwrap();
    let host_events = history
        .iter()
        .filter(|e| {
            matches!(
                e.content.kind,
                EventKind::HostInspectionStarted
                    | EventKind::HostCommandStarted
                    | EventKind::HostCommandCompleted
                    | EventKind::HostInspectionCompleted
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(host_events.len(), 8);
    assert!(host_events.iter().all(|e| e.content.audit_id.is_none()));
    assert!(host_events.iter().all(|e| e.content.model.is_none()));
}

#[tokio::test]
async fn r11_r14_failing_or_mismatched_queries_remain_partial_and_recorded() {
    let s = services().await;
    let app: Slug = "abastevo".parse().unwrap();
    let policy = policy();
    let mut results = BTreeMap::new();
    results.insert(
        HostCommand::Os,
        HostResult::Observed {
            observation: HostObservation::Kernel {
                release: "6.8.0".into(),
            },
        },
    );
    let hosts = FakeHosts::new(app.clone(), policy.clone(), results);
    let i = inspect_host(&s, &hosts, &app, &policy.profile)
        .await
        .unwrap();
    assert!(!i.is_success());
    assert!(i.completed_at.is_some());
    assert_eq!(hosts.calls(), HostCommand::ALL);
    assert!(
        i.commands
            .iter()
            .all(|c| matches!(c.result, Some(HostResult::Failed { .. })))
    );
    assert!(
        serde_json::to_string(&i)
            .unwrap()
            .contains("INVALID_RESPONSE")
    );
}

#[tokio::test]
#[ignore = "requires the disposable local SSH fixture in docs/HOST_INSPECTION.md"]
async fn r12_actual_abastevo_ssh_inventory_and_wrong_host_key() {
    use opentatu_scanners::{HostsConfig, SshHosts};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/phase9-validation/hosts.local.toml");
    let config = HostsConfig::load(&path).unwrap();
    let app: Slug = "abastevo".parse().unwrap();
    let profile: Slug = "local-host".parse().unwrap();
    assert!(
        config.profiles[&profile].address.is_loopback(),
        "acceptance can only connect to local disposable SSH"
    );
    let mut wrong_key = config.clone();
    let key = &mut wrong_key.profiles.get_mut(&profile).unwrap().host_key;
    let last = key.pop().unwrap();
    key.push(if last == 'A' { 'B' } else { 'A' });
    let s = services().await;
    let hosts = SshHosts::new(config).unwrap();
    let i = inspect_host(&s, &hosts, &app, &profile).await.unwrap();
    assert!(i.is_success(), "{i:?}");
    let bad = inspect_host(&s, &SshHosts::new(wrong_key).unwrap(), &app, &profile)
        .await
        .unwrap();
    assert!(!bad.is_success());
    assert!(bad.commands.iter().all(|c| matches!(
        c.result,
        Some(HostResult::Failed {
            failure: opentatu_core::HostFailure::Transport
        })
    )));
    assert_eq!(s.host_inspections(&app, 10).await.unwrap().len(), 2);
}
