//! `opentatu benchmark ...` (roadmap phase 6, ADR-0031): the IO around
//! `opentatu-benchmark`. The benchmark app is cloned at its pinned commit and never run;
//! the copy that is audited has its vulnerability markers stripped and its answer files left
//! out; each run is a normal FULL audit of that copy as a fresh app, scored against the
//! known flaws.

use std::path::{Path, PathBuf};

use anyhow::Context as _;
use opentatu_benchmark::{
    BenchmarkApp, Expected, RunResult, expected_flaws, find_markers, parse_challenges,
    pipeline_counts, render_scoreboard, score, strip_markers,
};
use opentatu_core::{Actor, AuditKind, AuditStatus, NewApp, UsageTotals};
use opentatu_services::NewAudit;
use opentatu_snapshots::Snapshots;

use super::worker::{Configs, Overrides};
use crate::output;

/// Marker file the snapshot step writes (never part of the app).
const SNAPSHOT_MARKER: &str = ".opentatu-snapshot";

fn app_file(dir: &Path, name: &str) -> PathBuf {
    dir.join("apps").join(format!("{name}.toml"))
}

fn expected_file(dir: &Path, name: &str) -> PathBuf {
    dir.join("expected").join(format!("{name}.json"))
}

fn load_app(dir: &Path, name: &str) -> anyhow::Result<BenchmarkApp> {
    let path = app_file(dir, name);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("no benchmark definition at {}", path.display()))?;
    Ok(BenchmarkApp::from_toml(&text)?)
}

/// `opentatu benchmark prepare <name>`.
pub async fn prepare(data_dir: &Path, dir: &Path, name: &str) -> anyhow::Result<()> {
    let app = load_app(dir, name)?;
    let layout = super::layout(data_dir)?;
    let snapshots = Snapshots::new(layout.clone());
    let upstream = app.upstream_slug()?;
    snapshots.refresh_mirror(&upstream, &app.repo_url).await?;
    let commit = snapshots.resolve(&upstream, &app.revision).await?;
    anyhow::ensure!(
        commit == app.commit,
        "{} {} now points to {commit}, not the pinned {} (R02): update the definition on purpose",
        app.name,
        app.revision,
        app.commit
    );
    let source = snapshots.capture(&upstream, &commit).await?.dir;
    let catalog_path = source.join(&app.challenges_file);
    let catalog = parse_challenges(
        &std::fs::read_to_string(&catalog_path)
            .with_context(|| format!("reading {}", catalog_path.display()))?,
    );

    let tree = layout.benchmark_tree_dir(&upstream);
    if tree.exists() {
        std::fs::remove_dir_all(&tree)
            .with_context(|| format!("removing the previous copy {}", tree.display()))?;
    }
    let mut files = Vec::new();
    collect_files(&source, &source, &mut files)?;
    let (mut copied, mut excluded, mut stripped) = (0usize, 0usize, 0usize);
    let mut markers = Vec::new();
    for (relative, path) in files {
        if app.is_excluded(&relative) {
            excluded += 1;
            continue;
        }
        let bytes = std::fs::read(&path).with_context(|| format!("reading {relative}"))?;
        let target = tree.join(&relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = match std::str::from_utf8(&bytes) {
            Ok(text) => {
                let found = find_markers(text, &app.marker);
                if !found.is_empty() {
                    markers.push((relative.clone(), found));
                }
                match strip_markers(text, &app.marker) {
                    Some(clean) => {
                        stripped += 1;
                        clean.into_bytes()
                    }
                    None => bytes,
                }
            }
            Err(_) => bytes,
        };
        std::fs::write(&target, content).with_context(|| format!("writing {relative}"))?;
        copied += 1;
    }
    let expected = expected_flaws(&app, &markers, &catalog);
    let expected_path = expected_file(dir, &app.name);
    if let Some(parent) = expected_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        &expected_path,
        serde_json::to_string_pretty(&expected)? + "\n",
    )?;
    let prepared = snapshots
        .record_tree(
            &tree,
            &format!(
                "{} {} ({}) prepared for the OpenTatu benchmark",
                app.title,
                app.revision,
                app.commit.short()
            ),
        )
        .await?;
    output::line(&format!(
        "prepared {} {}: {copied} files copied ({stripped} with markers stripped), {excluded} left out",
        app.title, app.revision
    ));
    output::line(&format!(
        "known flaws: {} → {}",
        expected.flaws.len(),
        expected_path.display()
    ));
    output::line(&format!(
        "audited copy: {} (commit {})",
        tree.display(),
        prepared.short()
    ));
    output::line(&format!("next: opentatu benchmark run {}", app.name));
    Ok(())
}

/// Every regular file under `dir` (no symlinks, no `.git`, no snapshot marker), with its
/// `/`-separated path relative to `root`, in a stable order.
fn collect_files(
    root: &Path,
    dir: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> anyhow::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("listing {}", dir.display()))?
        .collect::<Result<_, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type()?;
        let name = entry.file_name();
        if kind.is_symlink() || (dir == root && (name == ".git" || name == SNAPSHOT_MARKER)) {
            continue;
        }
        if kind.is_dir() {
            collect_files(root, &path, files)?;
        } else if kind.is_file() {
            let relative = path
                .strip_prefix(root)?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            files.push((relative, path));
        }
    }
    Ok(())
}

/// `opentatu benchmark run <name> [--role-model role=model]... [--max-tokens N]
/// [--max-minutes M]`.
pub async fn run(
    data_dir: &Path,
    dir: &Path,
    configs: Configs<'_>,
    name: &str,
    overrides: &Overrides,
) -> anyhow::Result<()> {
    let app = load_app(dir, name)?;
    let expected_path = expected_file(dir, &app.name);
    let expected: Expected = serde_json::from_str(
        &std::fs::read_to_string(&expected_path)
            .with_context(|| format!("run `opentatu benchmark prepare {}` first", app.name))?,
    )?;
    anyhow::ensure!(
        expected.upstream_commit == app.commit,
        "{} is from another commit: run `opentatu benchmark prepare {}` again",
        expected_path.display(),
        app.name
    );
    let layout = super::layout(data_dir)?;
    let tree = layout.benchmark_tree_dir(&app.upstream_slug()?);
    anyhow::ensure!(
        tree.join(".git").is_dir(),
        "no prepared copy at {}: run `opentatu benchmark prepare {}`",
        tree.display(),
        app.name
    );
    let services = super::services(data_dir).await?;
    let queued = services
        .recent_audits(1_000)
        .await?
        .into_iter()
        .filter(|a| a.status == AuditStatus::Created)
        .count();
    anyhow::ensure!(
        queued == 0,
        "{queued} audit(s) already queued: run `opentatu worker --once` first, so the benchmark measures only its own audit"
    );
    let worker = super::worker::build(data_dir, configs, overrides, true).await?;

    let taken: Vec<String> = services
        .apps()
        .await?
        .iter()
        .map(|a| a.slug().to_string())
        .collect();
    let mut number = 1;
    while taken.contains(&app.run_slug(number)?.to_string()) {
        number += 1;
    }
    let slug = app.run_slug(number)?;
    let url = tree.to_string_lossy().into_owned();
    services
        .register_app(NewApp {
            slug: slug.clone(),
            name: format!("{} — benchmark {number}", app.title),
            repo_url: url.clone(),
            branch: "main".into(),
            stack: app.stack.clone(),
            sensitive_data: app.sensitive_data.clone(),
            lab_url: None,
        })
        .await?;
    let snapshots = Snapshots::new(layout.clone());
    snapshots.refresh_mirror(&slug, &url).await?;
    let commit = snapshots.resolve(&slug, "main").await?;
    let audit = services
        .create_audit(NewAudit {
            app: slug.clone(),
            kind: AuditKind::Full,
            commit,
            branch: Some("main".into()),
            requested_by: Actor::Owner,
        })
        .await?;
    output::line(&format!(
        "benchmark {} run {number}: audit {} queued, running…",
        app.name, audit.id
    ));
    worker.drain().await?;

    let finished = services.audit(&audit.id).await?;
    anyhow::ensure!(
        matches!(
            finished.status,
            AuditStatus::Completed | AuditStatus::Partial
        ),
        "audit {} ended {}: {:?}",
        audit.id,
        finished.status,
        finished.limitations
    );
    let record: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        layout.audit_record_file(&audit.id),
    )?)?;
    let findings = services.findings_of_app(&slug).await?;
    let hypotheses = services.hypotheses_of_audit(&audit.id).await?;
    let result = RunResult {
        benchmark: app.name.clone(),
        revision: app.revision.clone(),
        upstream_commit: app.commit.clone(),
        audit_id: audit.id.clone(),
        status: finished.status.to_string(),
        completed_at: finished.completed_at,
        duration_seconds: finished.duration_seconds(),
        models: serde_json::from_value(record["models"].clone()).unwrap_or_default(),
        prompt_versions: serde_json::from_value(record["prompt_versions"].clone())
            .unwrap_or_default(),
        usage: serde_json::from_value::<UsageTotals>(record["usage"].clone()).unwrap_or_default(),
        overrides: overrides.describe(),
        pipeline: pipeline_counts(&findings, &hypotheses),
        score: score(&expected, &findings, &hypotheses, app.match_lines),
    };
    let results = dir.join("results");
    std::fs::create_dir_all(&results)?;
    let result_path = results.join(format!("{}.json", audit.id));
    std::fs::write(&result_path, serde_json::to_string_pretty(&result)? + "\n")?;
    let board = write_scoreboard(dir)?;
    let s = &result.score;
    output::line(&format!(
        "found {}/{} known flaws (recall {:.0}%), {} only as hypotheses, {} missed; {} of {} reported findings outside the list (precision {:.0}%); {} calls, US$ {:.3}",
        s.found.len(),
        s.expected,
        s.recall * 100.0,
        s.hypothesized_only.len(),
        s.missed.len(),
        s.false_positives.len(),
        s.reported,
        s.precision * 100.0,
        result.usage.calls,
        result.usage.cost_usd
    ));
    output::line(&format!(
        "result: {} · scoreboard: {}",
        result_path.display(),
        board.display()
    ));
    Ok(())
}

/// `opentatu benchmark rescore <AUDIT>`: recomputes the score and pipeline counts of a past
/// run from its data directory (after a change of the known flaws or of the matching rule),
/// keeping everything else of its result file.
pub async fn rescore(data_dir: &Path, dir: &Path, audit: &str) -> anyhow::Result<()> {
    let audit: opentatu_core::AuditId = audit.trim().parse()?;
    let path = dir.join("results").join(format!("{audit}.json"));
    let mut result: RunResult = serde_json::from_str(
        &std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?,
    )?;
    let app = load_app(dir, &result.benchmark)?;
    let expected: Expected =
        serde_json::from_str(&std::fs::read_to_string(expected_file(dir, &app.name))?)?;
    let services = super::services(data_dir).await?;
    let found = services.audit(&audit).await?;
    let findings = services.findings_of_app(&found.app).await?;
    let hypotheses = services.hypotheses_of_audit(&audit).await?;
    result.pipeline = pipeline_counts(&findings, &hypotheses);
    result.score = score(&expected, &findings, &hypotheses, app.match_lines);
    std::fs::write(&path, serde_json::to_string_pretty(&result)? + "\n")?;
    let board = write_scoreboard(dir)?;
    output::line(&format!(
        "rescored {audit}: found {}/{} · {}",
        result.score.found.len(),
        result.score.expected,
        board.display()
    ));
    Ok(())
}

/// `opentatu benchmark scoreboard`: `SCOREBOARD.md` from every result file.
pub fn scoreboard(dir: &Path) -> anyhow::Result<()> {
    let board = write_scoreboard(dir)?;
    output::line(&format!("wrote {}", board.display()));
    Ok(())
}

fn write_scoreboard(dir: &Path) -> anyhow::Result<PathBuf> {
    let mut results: Vec<RunResult> = Vec::new();
    let results_dir = dir.join("results");
    if results_dir.is_dir() {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&results_dir)?
            .map(|entry| entry.map(|e| e.path()))
            .collect::<Result<_, _>>()?;
        paths.retain(|p| p.extension().is_some_and(|e| e == "json"));
        for path in paths {
            let text = std::fs::read_to_string(&path)?;
            results.push(
                serde_json::from_str(&text)
                    .with_context(|| format!("reading {}", path.display()))?,
            );
        }
    }
    results.sort_by(|a, b| {
        (a.completed_at, a.audit_id.as_str()).cmp(&(b.completed_at, b.audit_id.as_str()))
    });
    let path = dir.join("SCOREBOARD.md");
    std::fs::write(&path, render_scoreboard(&results))?;
    Ok(path)
}
