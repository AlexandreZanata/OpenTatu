//! Test double of [`ScannerSuite`] (feature `test-support`): no containers, canned results.

#![allow(clippy::panic)] // test support: a poisoned lock is a test bug

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use opentatu_core::{ScannerKind, ScannerResult, ScannerRun};

use crate::{Error, ScannerSuite};

/// Socket-free host transport for admission/orchestration tests, using the actual schema.
#[derive(Debug)]
pub struct FakeHosts {
    app: opentatu_core::Slug,
    policy: opentatu_core::HostPolicy,
    results: BTreeMap<opentatu_core::HostCommand, opentatu_core::HostResult>,
    calls: Mutex<Vec<opentatu_core::HostCommand>>,
}

impl FakeHosts {
    /// Canned normalized results; absent commands explicitly fail rather than looking safe.
    pub fn new(
        app: opentatu_core::Slug,
        policy: opentatu_core::HostPolicy,
        results: BTreeMap<opentatu_core::HostCommand, opentatu_core::HostResult>,
    ) -> Self {
        Self {
            app,
            policy,
            results,
            calls: Mutex::default(),
        }
    }
    /// Ordered fixed queries received so far.
    pub fn calls(&self) -> Vec<opentatu_core::HostCommand> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[async_trait::async_trait]
impl crate::HostSuite for FakeHosts {
    fn policy(
        &self,
        app: &opentatu_core::Slug,
        profile: &opentatu_core::Slug,
    ) -> Option<opentatu_core::HostPolicy> {
        (app == &self.app && profile == &self.policy.profile).then(|| self.policy.clone())
    }
    async fn query(
        &self,
        app: &opentatu_core::Slug,
        policy: &opentatu_core::HostPolicy,
        command: opentatu_core::HostCommand,
    ) -> opentatu_core::HostResult {
        use opentatu_core::{HostFailure, HostResult};
        if app != &self.app || policy != &self.policy {
            return HostResult::Failed {
                failure: HostFailure::Configuration,
            };
        }
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(command);
        self.results
            .get(&command)
            .cloned()
            .unwrap_or(HostResult::Failed {
                failure: HostFailure::Transport,
            })
    }
}

/// What the fake does for one scanner.
#[derive(Debug, Clone)]
pub enum FakeOutcome {
    /// Succeeds with these results and writes `<tool>.json` with `[]`.
    Results(Vec<ScannerResult>),
    /// Offline data missing ([`Error::NotReady`]).
    NotReady,
    /// The tool fails with this exit code.
    Fails(i32),
}

/// [`ScannerSuite`] returning canned outcomes and recording what ran.
#[derive(Debug, Default)]
pub struct FakeSuite {
    outcomes: BTreeMap<ScannerKind, FakeOutcome>,
    calls: Mutex<Vec<ScannerKind>>,
}

impl FakeSuite {
    /// Every scanner succeeds with no results.
    pub fn clean() -> Self {
        let outcomes = ScannerKind::STATIC
            .iter()
            .map(|kind| (*kind, FakeOutcome::Results(Vec::new())));
        Self {
            outcomes: outcomes.collect(),
            calls: Mutex::default(),
        }
    }

    /// Replaces the outcome of `kind`.
    #[must_use]
    pub fn with(mut self, kind: ScannerKind, outcome: FakeOutcome) -> Self {
        self.outcomes.insert(kind, outcome);
        self
    }

    /// Scanners run so far, in order.
    pub fn calls(&self) -> Vec<ScannerKind> {
        self.calls
            .lock()
            .unwrap_or_else(|_| panic!("fake suite lock poisoned"))
            .clone()
    }

    /// A result of `kind` with `n` as its sequence number.
    pub fn result(kind: ScannerKind, n: usize, rule_id: &str, file: &str) -> ScannerResult {
        ScannerResult {
            scanner: kind,
            id: format!("{kind}:{n}"),
            rule_id: rule_id.to_owned(),
            message: format!("{rule_id} in {file}"),
            file: Some(file.to_owned()),
            start_line: Some(1),
            end_line: Some(1),
            reported_severity: Some("HIGH".to_owned()),
            cwe: None,
        }
    }
}

/// VM-free dynamic port using real policy/receipt contracts and recording executions.
#[derive(Debug)]
pub struct FakeLab {
    app: opentatu_core::Slug,
    policy: opentatu_core::LabPolicy,
    outcome: opentatu_core::LabOutcome,
    calls: Mutex<Vec<crate::LabRequest>>,
    tamper: bool,
}
impl FakeLab {
    /// Scripted dynamic outcome; no VM, sockets or real data.
    pub fn new(
        app: opentatu_core::Slug,
        policy: opentatu_core::LabPolicy,
        outcome: opentatu_core::LabOutcome,
    ) -> Self {
        Self {
            app,
            policy,
            outcome,
            calls: Mutex::new(Vec::new()),
            tamper: false,
        }
    }
    /// Simulates a receipt from a different source/controller.
    #[must_use]
    pub fn with_tampered_provenance(mut self) -> Self {
        self.tamper = true;
        self
    }
    /// Requests received so far.
    pub fn calls(&self) -> Vec<crate::LabRequest> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}
#[async_trait::async_trait]
impl crate::LabSuite for FakeLab {
    fn policy(&self, app: &opentatu_core::Slug) -> Option<opentatu_core::LabPolicy> {
        (app == &self.app).then(|| self.policy.clone())
    }
    async fn run(&self, request: crate::LabRequest) -> Result<opentatu_core::LabExecution, Error> {
        use opentatu_core::{LabExecution, LabObservation, LabOutcome};
        let observation = |status, marker_found| LabObservation {
            status,
            marker_found,
            bytes: 7,
            body_digest: "synthetic-hash".into(),
        };
        let subject = match self.outcome {
            LabOutcome::Reproduced => observation(200, true),
            LabOutcome::NotReproduced => observation(403, false),
            LabOutcome::Inconclusive => observation(500, false),
        };
        let source_digest = if self.tamper {
            "wrong-source".into()
        } else {
            request.source_digest.clone()
        };
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(request.clone());
        Ok(LabExecution {
            outcome: self.outcome,
            control: Some(observation(200, true)),
            subject: Some(subject),
            image_digest: self.policy.image_digest.clone(),
            runner_digest: self.policy.runner_digest.clone(),
            source_digest,
            zap: Some(ScannerRun {
                scanner: ScannerKind::Zap,
                version: "fake-zap".into(),
                image: self.policy.image_digest.clone(),
                duration_ms: 1,
                results: Vec::new(),
            }),
            note: String::new(),
        })
    }
}

#[async_trait::async_trait]
impl ScannerSuite for FakeSuite {
    fn enabled(&self) -> Vec<ScannerKind> {
        self.outcomes.keys().copied().collect()
    }

    fn ready(&self, kind: ScannerKind) -> bool {
        !matches!(self.outcomes.get(&kind), Some(FakeOutcome::NotReady))
    }

    async fn run(
        &self,
        kind: ScannerKind,
        _source_dir: &Path,
        output_dir: &Path,
    ) -> Result<ScannerRun, Error> {
        self.calls
            .lock()
            .unwrap_or_else(|_| panic!("fake suite lock poisoned"))
            .push(kind);
        match self.outcomes.get(&kind) {
            None => Err(Error::Disabled(kind)),
            Some(FakeOutcome::NotReady) => Err(Error::NotReady(kind)),
            Some(FakeOutcome::Fails(code)) => Err(Error::Failed {
                scanner: kind,
                code: *code,
                stderr: "fake failure".to_owned(),
            }),
            Some(FakeOutcome::Results(results)) => {
                let file = output_dir.join(format!("{kind}.json"));
                std::fs::create_dir_all(output_dir)
                    .and_then(|()| std::fs::write(&file, "[]"))
                    .map_err(|source| Error::Io { path: file, source })?;
                Ok(ScannerRun {
                    scanner: kind,
                    version: "0.0.0-fake".to_owned(),
                    image: format!("fake/{kind}@sha256:0"),
                    duration_ms: 1,
                    results: results.clone(),
                })
            }
        }
    }
}
