//! Phase 8 orchestration with a fake VM and scripted model answers.
use super::*;
use opentatu_core::{LabCase, LabOutcome, LabPolicy};
use opentatu_scanners::testing::FakeLab;

fn policy() -> LabPolicy {
    let hash = format!("sha256:{}", "a".repeat(64));
    LabPolicy {
        port: 8080,
        profile_digest: hash.clone(),
        image_digest: hash.clone(),
        runner_digest: hash,
        max_runs: 2,
        timeout_seconds: 120,
        source_paths: vec!["src".into()],
        cases: vec![LabCase {
            id: "account".parse().unwrap(),
            path: "/accounts/b".into(),
            file: "src/route.ts".into(),
            symbol: "GET".into(),
            cwes: vec!["CWE-284".into()],
            control_actor: "owner-b".parse().unwrap(),
            subject_actor: "stranger-a".parse().unwrap(),
            marker: "FAKE_B".into(),
        }],
    }
}
async fn exercise(approve: bool, case: &str, tamper: bool) -> (Setup, Audit, Arc<FakeLab>) {
    let model = Arc::new(FakeClient::new());
    let mut verifier: serde_json::Value = serde_json::from_str(VERIFIER_ANSWER).unwrap();
    verifier["lab_case_id"] = case.into();
    // Deliberately forged model evidence must be removed before system receipts enter.
    verifier["evidence"].as_array_mut().unwrap().push(
        serde_json::json!({"kind":"test_result","description":"invented test success","steps":[]}),
    );
    model.push_answer(CARTOGRAPHER_ANSWER);
    model.push_answer(HUNTER_ANSWER);
    model.push_answer(verifier.to_string());
    let executed = approve && case == "account";
    if executed {
        model.push_answer(verifier.to_string());
    }
    if executed && !tamper {
        model.push_answer(SKEPTIC_ANSWER);
        model.push_answer(PLANNER_ANSWER);
    }
    model.push_answer(REPORTER_ANSWER);
    let mut setup = setup(None, FakeSuite::clean(), Some((model.clone(), models()))).await;
    let policy = policy();
    let lab = FakeLab::new(
        "demo".parse().unwrap(),
        policy.clone(),
        LabOutcome::Reproduced,
    );
    let lab = Arc::new(if tamper {
        lab.with_tampered_provenance()
    } else {
        lab
    });
    setup.worker = setup.worker.with_lab(lab.clone());
    let audit = enqueue(&setup).await;
    if approve {
        setup
            .services
            .approve_lab_audit(&audit.id, &policy)
            .await
            .unwrap();
    }
    setup.worker.drain().await.unwrap();
    if executed {
        let request = &model.requests()[3];
        assert!(request.messages[1].content.contains("executed_result"));
        assert!(request.messages[1].content.contains("<lab>\n"));
        assert_eq!(request.messages[1].content.matches("</lab>").count(), 1);
    }
    (setup, audit, lab)
}
#[tokio::test]
async fn r05_r10_confirmed_requires_executed_receipt_and_fenced_second_review() {
    let (setup, audit, lab) = exercise(true, "account", false).await;
    assert_eq!(lab.calls().len(), 1);
    let runs = setup.services.lab_runs(&audit.id).await.unwrap();
    assert_eq!(
        runs[0].execution.as_ref().unwrap().outcome,
        LabOutcome::Reproduced
    );
    let hypotheses = setup.services.hypotheses_of_audit(&audit.id).await.unwrap();
    let verification = hypotheses[0].verification.as_ref().unwrap();
    assert_eq!(verification.lab_run_number, Some(1));
    assert_eq!(verification.verdict, opentatu_core::Verdict::Confirmed);
    assert_eq!(
        verification
            .evidence
            .iter()
            .filter(|e| e.kind == "lab_request_response")
            .count(),
        1
    );
    assert!(
        !verification
            .evidence
            .iter()
            .any(|e| e.description.contains("invented test"))
    );
    let layout = DataLayout::new(setup.data.path());
    let record = std::fs::read_to_string(layout.audit_record_file(&audit.id)).unwrap();
    assert!(record.contains("lab_runs"));
    assert!(layout.lab_runs_file(&audit.id).exists());
    let report = std::fs::read_to_string(layout.report_file(&audit.id)).unwrap();
    assert!(report.contains("Laboratório dinâmico (sistema)"));
    assert!(report.contains("REPRODUCED"));
    assert!(
        setup
            .services
            .scanner_results(&audit.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        setup
            .services
            .scanner_runs(&audit.id)
            .await
            .unwrap()
            .iter()
            .any(|s| s.scanner == ScannerKind::Zap)
    );
}
#[tokio::test]
async fn r10_missing_approval_or_unknown_case_never_starts_vm() {
    for (approved, case) in [(false, "account"), (true, "invented")] {
        let (setup, audit, lab) = exercise(approved, case, false).await;
        assert!(lab.calls().is_empty());
        assert!(setup.services.lab_runs(&audit.id).await.unwrap().is_empty());
        assert!(
            setup
                .services
                .findings_of_app(&audit.app)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            setup.services.hypotheses_of_audit(&audit.id).await.unwrap()[0].state,
            FindingState::Inconclusive
        );
    }
}
#[tokio::test]
async fn r02_r05_changed_source_receipt_cannot_confirm_a_finding() {
    let (setup, audit, lab) = exercise(true, "account", true).await;
    assert_eq!(lab.calls().len(), 1);
    assert!(
        setup
            .services
            .scanner_runs(&audit.id)
            .await
            .unwrap()
            .iter()
            .any(|s| s.scanner == ScannerKind::Zap
                && matches!(s.outcome, opentatu_services::ScannerOutcome::Failed { .. }))
    );

    assert_eq!(
        setup.services.lab_runs(&audit.id).await.unwrap()[0]
            .execution
            .as_ref()
            .unwrap()
            .outcome,
        LabOutcome::Inconclusive
    );
    assert!(
        setup
            .services
            .findings_of_app(&audit.app)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn r06_r10_fix_needs_fresh_approval_and_a_negative_probe_with_positive_control() {
    for approved in [false, true] {
        let model = Arc::new(FakeClient::new());
        let (mut setup, audit) = submitted_fix(&model, true).await;
        let policy = policy();
        let lab = Arc::new(FakeLab::new(
            "demo".parse().unwrap(),
            policy.clone(),
            LabOutcome::NotReproduced,
        ));
        setup.worker = setup.worker.with_lab(lab.clone());
        if approved {
            setup
                .services
                .approve_lab_audit(&audit.id, &policy)
                .await
                .unwrap();
        }
        let mut answer: serde_json::Value =
            serde_json::from_str(&reverifier_answer(audit.id.as_str(), "dismissed")).unwrap();
        answer["lab_case_id"] = "account".into();
        model.push_answer(answer.to_string());
        if approved {
            model.push_answer(answer.to_string());
        }
        setup.worker.drain().await.unwrap();
        let finding = setup
            .services
            .finding(&"DEMO-SEC-0001".parse().unwrap())
            .await
            .unwrap();
        let task = setup
            .services
            .task(&"DEMO-SEC-0001-T1".parse().unwrap())
            .await
            .unwrap();
        if approved {
            assert_eq!(finding.state, FindingState::FixApproved);
            assert_eq!(task.state, TaskState::Done);
            assert_eq!(lab.calls().len(), 1);
        } else {
            assert_eq!(finding.state, FindingState::TaskCreated);
            assert_eq!(task.state, TaskState::Open);
            assert!(lab.calls().is_empty());
        }
        assert!(
            !setup
                .services
                .audit_events(&audit.id)
                .await
                .unwrap()
                .iter()
                .any(|e| e.content.kind == EventKind::FindingClosed)
        );
    }
}

#[tokio::test]
async fn r05_r17_failed_follow_up_keeps_prompt_provenance_without_confirming() {
    let model = Arc::new(FakeClient::new());
    model.push_answer(CARTOGRAPHER_ANSWER);
    model.push_answer(HUNTER_ANSWER);
    let mut verifier: serde_json::Value = serde_json::from_str(VERIFIER_ANSWER).unwrap();
    verifier["lab_case_id"] = "account".into();
    model.push_answer(verifier.to_string());
    let mut setup = setup(None, FakeSuite::clean(), Some((model.clone(), models()))).await;
    let policy = policy();
    let lab = Arc::new(FakeLab::new(
        "demo".parse().unwrap(),
        policy.clone(),
        LabOutcome::Reproduced,
    ));
    setup.worker = setup.worker.with_lab(lab);
    let audit = enqueue(&setup).await;
    setup
        .services
        .approve_lab_audit(&audit.id, &policy)
        .await
        .unwrap();
    // The actual result exists, but the scripted model has no follow-up answer.
    setup.worker.drain().await.unwrap();
    assert_eq!(setup.services.lab_runs(&audit.id).await.unwrap().len(), 1);
    assert!(
        setup
            .services
            .findings_of_app(&audit.app)
            .await
            .unwrap()
            .is_empty()
    );
    let layout = DataLayout::new(setup.data.path());
    let record: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(layout.audit_record_file(&audit.id)).unwrap(),
    )
    .unwrap();
    assert_eq!(record["prompt_versions"]["verifier"], "1.3.0");
}
