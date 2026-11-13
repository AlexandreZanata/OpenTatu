//! Opt-in actual Abastevo/QEMU acceptance; see docs/LAB.md for local prerequisites.
#![allow(clippy::unwrap_used)] // Integration fixture failures must stop the test.
use opentatu_core::{
    Actor, AuditKind, CodeLocation, Confidence, EvidenceItem, EvidenceStep, Fingerprint,
    FingerprintInput, HypothesisContent, LabContext, LabOutcome, NewApp, Severity, SystemClock,
    Verdict, Verification,
};
use opentatu_scanners::{LabRequest, LabSuite, QemuLab};
use opentatu_services::{NewAudit, NewHypothesis, Services};
use opentatu_snapshots::Snapshots;
use std::{path::Path, sync::Arc};

async fn execute(expected: LabOutcome) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config_path = root.join("config/lab-acceptance.local.toml");
    let config = opentatu_scanners::LabsConfig::load(&config_path).unwrap().0;
    let app = "abastevo".parse().unwrap();
    let profile = &config.profiles[&app];
    let source = profile.image.parent().unwrap().join("abastevo");
    let lab = QemuLab::load(&config_path, root.join("target/release/opentatu")).unwrap();
    let policy = lab.policy(&app).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let layout = opentatu_core::DataLayout::new(temp.path());
    let history = tempfile::Builder::new()
        .prefix("acceptance-")
        .tempdir_in(profile.image.parent().unwrap())
        .unwrap()
        .keep();
    let services = Services::in_memory(Arc::new(SystemClock)).await.unwrap();
    services
        .register_app(NewApp {
            slug: app.clone(),
            name: "Abastevo (fictitious offline copy)".into(),
            repo_url: source.to_string_lossy().into(),
            branch: "HEAD".into(),
            stack: vec!["Go".into()],
            sensitive_data: vec![],
            lab_url: None,
        })
        .await
        .unwrap();
    let snapshots = Snapshots::new(layout);
    snapshots
        .refresh_mirror(&app, &source.to_string_lossy())
        .await
        .unwrap();
    let commit = snapshots.resolve(&app, "HEAD").await.unwrap();
    services
        .create_lab_audit(
            NewAudit {
                app: app.clone(),
                kind: AuditKind::Full,
                commit: commit.clone(),
                branch: None,
                requested_by: Actor::Owner,
            },
            &policy,
        )
        .await
        .unwrap();
    let audit = services.start_next_audit().await.unwrap().unwrap();
    let snapshot = snapshots.capture(&app, &commit).await.unwrap();
    let files = opentatu_context::SnapshotFiles::scan(&snapshot.dir).unwrap();
    let case = &policy.cases[0];
    let snippet = files.read_text(&case.file).unwrap().content;
    let line_of = |needle: &str| {
        snippet
            .lines()
            .position(|line| line.contains(needle))
            .unwrap()
            + 1
    };
    let actor_line = line_of("r.Header.Get");
    let account_line = line_of("chi.URLParam");
    let response_line = line_of("w.Write");
    let content = HypothesisContent {
        title: "Leitura de conta fictícia sem vínculo do proprietário".into(),
        category: "Controle de acesso".into(),
        cwe: "CWE-639".into(),
        asvs: vec![],
        entry_point: None,
        location: CodeLocation {
            file: case.file.clone(),
            lines: format!("{actor_line}-{response_line}"),
            symbol: "phase8Account".into(),
        },
        reasoning: "Caso de aceitação conhecido e explícito, não descoberta por modelo.".into(),
        impact_scenario: "Conta fictícia B lida pela conta A.".into(),
        required_conditions: vec![],
        how_to_verify: "Executar o caso aprovado na VM.".into(),
        potential_severity: Severity::High,
        confidence: Confidence::High,
        origin: "manual_analysis".into(),
        possible_regression_of: None,
    };
    let fingerprint = Fingerprint::compute(&FingerprintInput {
        cwe: &content.cwe,
        file: &case.file,
        symbol: "phase8Account",
        snippet: &snippet,
    });
    let hypothesis = services
        .record_hypothesis(
            &audit,
            NewHypothesis {
                content,
                fingerprint,
                model: "system".into(),
                prompt_version: "system".into(),
                system_notes: vec![],
            },
            &opentatu_memory::DedupOutcome::New {
                possibly_same_as: None,
            },
        )
        .await
        .unwrap()
        .unwrap();
    let context = LabContext {
        audit_id: audit.id.clone(),
        hypothesis_id: hypothesis.hypothesis_id.clone(),
        commit,
        profile_digest: policy.profile_digest.clone(),
        case_id: case.id.clone(),
    };
    let run = services
        .start_lab_run(context, policy.max_runs)
        .await
        .unwrap();
    let source = opentatu_context::prepare_lab_source(&files, &policy.source_paths).unwrap();
    std::fs::write(
        history.join("started-events.json"),
        serde_json::to_string_pretty(&services.audit_events(&audit.id).await.unwrap()).unwrap(),
    )
    .unwrap();
    let execution = lab
        .run(LabRequest {
            run,
            source_dir: source.path().into(),
            source_digest: source.digest.clone(),
            timeout_seconds: policy.timeout_seconds,
        })
        .await
        .inspect_err(|error| {
            let _ = std::fs::write(history.join("failure.txt"), error.to_string());
        })
        .unwrap();
    let run = services
        .complete_lab_run(&audit.id, 1, execution)
        .await
        .unwrap();
    assert!(run.completed_at.is_some());
    let execution = run.execution.as_ref().unwrap();
    let destination = history.join(format!("{}.json", audit.id));
    std::fs::write(&destination, serde_json::to_string_pretty(&run).unwrap()).unwrap();
    assert_eq!(execution.outcome, expected, "{}", execution.note);
    assert!(execution.zap.is_some(), "{}", execution.note);
    if expected == LabOutcome::Reproduced {
        let verification = Verification {
            lab_run_number: Some(1),
            verdict: Verdict::Confirmed,
            justification: "Controle autorizado e leitura indevida reproduzidos na VM.".into(),
            evidence: vec![EvidenceItem {
                evidence_id: None,
                kind: "code_flow".into(),
                description: "O handler omite o vínculo entre ator e conta.".into(),
                steps: vec![
                    EvidenceStep {
                        file: case.file.clone(),
                        line: actor_line.to_string(),
                        what_happens: "Identidade fictícia lida do cabeçalho.".into(),
                    },
                    EvidenceStep {
                        file: case.file.clone(),
                        line: account_line.to_string(),
                        what_happens: "Conta selecionada pelo parâmetro da URL.".into(),
                    },
                    EvidenceStep {
                        file: case.file.clone(),
                        line: response_line.to_string(),
                        what_happens: "Fixture retornada sem comparar identidade e conta.".into(),
                    },
                ],
            }],
            mitigations_found: vec![],
            regression_test: None,
            missing_to_conclude: String::new(),
            model: "system".into(),
            prompt_version: "system".into(),
            system_notes: vec![],
        };
        assert!(verification.evidence_problem(&files).is_none());
        assert!(
            services
                .record_verification(&hypothesis.hypothesis_id, verification)
                .await
                .unwrap()
                .1
                .is_some()
        );
    }
    services
        .finish_audit(
            &audit.id,
            opentatu_core::AuditStatus::Partial,
            &["Known-case VM acceptance only; no AI discovery or full audit coverage.".into()],
        )
        .await
        .unwrap();
    std::fs::write(
        history.join("events.json"),
        serde_json::to_string_pretty(&services.audit_events(&audit.id).await.unwrap()).unwrap(),
    )
    .unwrap();
    std::fs::write(
        history.join("findings.json"),
        serde_json::to_string_pretty(&services.findings_of_app(&app).await.unwrap()).unwrap(),
    )
    .unwrap();
}
#[tokio::test]
#[ignore = "requires prepared offline QEMU image and local fictitious Abastevo fixture; docs/LAB.md"]
async fn r10_actual_abastevo_vm_reproduces_access_control_flaw() {
    execute(LabOutcome::Reproduced).await;
}
#[tokio::test]
#[ignore = "requires the same local fixture after applying its ownership fix; docs/LAB.md"]
async fn r10_actual_abastevo_vm_refutes_after_ownership_fix() {
    execute(LabOutcome::NotReproduced).await;
}
