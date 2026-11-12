//! §C.7 steps 7–8: hunter rounds and deduplication against the memory.
//!
//! Each round is one hunter call over the next batch of entry points, prioritized (no or
//! unknown authentication, sensitive data, administrative paths, state-changing methods
//! first), with the code each one reaches (`entry_point_code`). At most [`MAX_ROUNDS`]; the
//! hunt stops early when a round yields fewer than [`MIN_STRONG`] new hypotheses with medium
//! or high confidence (spec §C.7 step 7). Entry points never reached are a limitation (R14).
//!
//! Every hypothesis is validated against the commit (file, lines, entry point, ASVS ids,
//! CWE) and fingerprinted from the code at its lines before the memory decides what it is
//! (`opentatu_memory::classify`).

use std::collections::BTreeSet;

use opentatu_context::{Seed, entry_point_code};
use opentatu_core::{
    AiRole, AuthenticationRequirement, Confidence, EntryPoint, FileIndex, HuntRound,
    HypothesisContent, ScannerKind, ScannerResult, SecurityMap,
};
use opentatu_investigator::{
    Block, HunterOutput, RoleCall, finding_lines, hypothesis_lines, map_excerpt, run_role,
    scanner_lines,
};
use opentatu_memory::{DedupOutcome, classify};
use opentatu_services::{NewHypothesis, Reporter};

use crate::Error;
use crate::ai_run::AiRun;
use crate::cartography::left_out_note;

/// Hunter calls per audit, at most (spec §C.7 step 7).
pub(crate) const MAX_ROUNDS: usize = 3;
/// Fewer new medium/high-confidence hypotheses than this ends the hunt.
const MIN_STRONG: usize = 2;
/// Entry points per round, at most (larger rounds made answers exceed 5 minutes).
const ROUND_ENTRY_POINTS: usize = 20;
/// Distinct handler files per round, at most.
const ROUND_FILES: usize = 10;
/// `<code>` of one round, in bytes (about 85K tokens).
const ROUND_CODE_BYTES: usize = 300_000;
/// One file in `<code>`, in bytes.
const ROUND_FILE_BYTES: usize = 40_000;
/// Scanner results given to one round, at most.
const MAX_SCANNER_LINES: usize = 150;
/// Remembered false positives given to the hunter, at most.
const MAX_FALSE_POSITIVES: usize = 40;

/// Runs the hunter rounds. Returns the limitations they produced.
pub(crate) async fn run(run: &AiRun) -> Result<Vec<String>, Error> {
    let services = &run.worker.services;
    let audit = &run.audit;
    let Some(map) = services.map_of_audit(&audit.id).await? else {
        return Ok(vec![
            "Caça de vulnerabilidades não executada: a auditoria não tem mapa de segurança."
                .to_owned(),
        ]);
    };
    let batches = batches(&prioritized(&map.content.entry_points));
    let scanner_results = services.scanner_results(&audit.id).await?;
    let dismissed = services.dismissed_hypotheses(audit).await?;
    let dismissed_before = services.dismissed_before(audit).await?;
    let mut limitations = Vec::new();
    let mut covered: Vec<String> = Vec::new();

    for (index, batch) in batches.iter().take(MAX_ROUNDS).enumerate() {
        let round = u32::try_from(index + 1).unwrap_or(u32::MAX);
        if !run.allow_call().await? {
            break;
        }
        let mut seeds: Vec<Seed> = Vec::new();
        for entry in batch {
            if !seeds
                .iter()
                .any(|s| s.path == entry.file && s.line == entry.line)
            {
                seeds.push(Seed {
                    path: entry.file.clone(),
                    line: entry.line,
                });
            }
        }
        let code = entry_point_code(&run.files, &seeds, ROUND_CODE_BYTES, ROUND_FILE_BYTES);
        let code_files: BTreeSet<&str> = code.excerpts.iter().map(|e| e.path.as_str()).collect();
        let scanner: Vec<&ScannerResult> = scanner_results
            .iter()
            .filter(|r| {
                r.file.as_deref().is_some_and(|f| code_files.contains(f))
                    || round == 1 && r.scanner != ScannerKind::Semgrep
            })
            .take(MAX_SCANNER_LINES)
            .collect();
        let findings = services.findings_of_app(&audit.app).await?;
        let open: Vec<_> = findings.iter().filter(|f| f.is_open()).collect();
        let recorded = services.hypotheses_of_audit(&audit.id).await?;
        let false_positives: Vec<_> = dismissed.iter().rev().take(MAX_FALSE_POSITIVES).collect();
        let inputs = vec![
            (Block::Map, map_excerpt(&map, batch)),
            (Block::Scanner, scanner_lines(&scanner)),
            (Block::Code, code.render()),
            (Block::OpenFindings, finding_lines(&open)),
            (Block::FalsePositives, hypothesis_lines(&false_positives)),
            (
                Block::Hypotheses,
                hypothesis_lines(&recorded.iter().collect::<Vec<_>>()),
            ),
        ];
        let variables = run.variables().await?;
        let call = RoleCall {
            audit_id: &audit.id,
            variables: &variables,
            inputs,
            conversation: round,
        };
        tracing::info!(audit_id = %audit.id, round, entry_points = batch.len(), code_bytes = code.bytes, "hunter round started");
        let answer = match run_role::<HunterOutput>(&run.ai.llm, call).await {
            Ok(answer) => answer,
            Err(error) => {
                tracing::warn!(audit_id = %audit.id, round, %error, "hunter round failed");
                limitations.push(format!("Rodada {round} do caçador falhou ({error})."));
                break;
            }
        };
        covered.extend(batch.iter().map(|e| e.id.clone()));
        let prompt = answer.prompt_versions.last().cloned().unwrap_or_default();
        services
            .record_suspicious_instructions(
                &audit.id,
                Reporter {
                    role: AiRole::Hunter,
                    model: &answer.model,
                    prompt_version: &prompt,
                },
                &answer.output.common.suspicious_instructions,
            )
            .await?;

        let (mut new, mut strong, mut dropped) = (0usize, 0usize, Vec::new());
        for content in answer.output.hypotheses {
            let (content, notes) = match validate(content, run, &map) {
                Ok(valid) => valid,
                Err(note) => {
                    dropped.push(note);
                    continue;
                }
            };
            let fingerprint = run.fingerprint(&content.cwe, &content.location);
            let findings = services.findings_of_app(&audit.app).await?;
            let earlier: Vec<_> = services
                .hypotheses_of_audit(&audit.id)
                .await?
                .into_iter()
                .map(|h| (h.fingerprint, h.hypothesis_id))
                .collect();
            let location = &content.location;
            let outcome = classify(
                &fingerprint,
                &content.cwe,
                &location.file,
                &location.symbol,
                &findings,
                &dismissed_before,
                &earlier,
            );
            let is_new = matches!(outcome, DedupOutcome::New { .. });
            let is_strong = content.confidence != Confidence::Low;
            let input = NewHypothesis {
                content,
                fingerprint,
                model: answer.model.clone(),
                prompt_version: prompt.clone(),
                system_notes: notes,
            };
            if services
                .record_hypothesis(audit, input, &outcome)
                .await?
                .is_some()
                && is_new
            {
                new += 1;
                strong += usize::from(is_strong);
            }
        }
        let triage = answer
            .output
            .scanner_triage
            .into_iter()
            .filter(|t| scanner_results.iter().any(|r| r.id == t.result_id))
            .collect();
        let mut checked: Vec<String> = Vec::new();
        for id in &answer.output.requirements_checked {
            if let Some(requirement) = run.catalog.resolve(id)
                && !checked.contains(&requirement.id)
            {
                checked.push(requirement.id.clone());
            }
        }
        services
            .record_hunt_round(
                &audit.id,
                &HuntRound {
                    round,
                    entry_points: batch.iter().map(|e| e.id.clone()).collect(),
                    requirements_checked: checked,
                    scanner_triage: triage,
                    new_hypotheses: new,
                    strong_hypotheses: strong,
                    gaps: answer.output.common.gaps,
                    not_analyzed: answer.output.common.not_analyzed.clone(),
                    model: answer.model.clone(),
                    prompt_version: prompt.clone(),
                },
            )
            .await?;
        tracing::info!(audit_id = %audit.id, round, new, strong, dropped = dropped.len(), "hunter round recorded");
        limitations.extend(
            answer
                .output
                .common
                .not_analyzed
                .iter()
                .map(|item| format!("Caçador: {item}")),
        );
        limitations.extend(left_out_note(
            "código",
            &format!("ao caçador (rodada {round})"),
            &code,
        ));
        if !dropped.is_empty() {
            limitations.push(format!(
                "Caçador: {} hipótese(s) descartada(s) pelo sistema por citarem arquivos inexistentes no commit.",
                dropped.len()
            ));
        }
        if strong < MIN_STRONG {
            break;
        }
    }

    let missing: Vec<&str> = map
        .content
        .entry_points
        .iter()
        .filter(|e| !covered.contains(&e.id))
        .map(|e| e.id.as_str())
        .collect();
    if !missing.is_empty() {
        let shown = missing
            .iter()
            .take(20)
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        let more = missing.len().saturating_sub(20);
        let suffix = if more > 0 {
            format!(" e mais {more}")
        } else {
            String::new()
        };
        limitations.push(format!(
            "Entradas não analisadas pelo caçador ({} de {}): {shown}{suffix}.",
            missing.len(),
            map.content.entry_points.len()
        ));
    }
    Ok(limitations)
}

/// Entry points in hunting order: no/unknown authentication, sensitive data,
/// administrative paths and state-changing methods first; then the map order.
fn prioritized(entry_points: &[EntryPoint]) -> Vec<&EntryPoint> {
    let mut ordered: Vec<&EntryPoint> = entry_points.iter().collect();
    // Lower is earlier: authentication (none 0, unknown 1, required 2), reads +2, no
    // sensitive data +1, not administrative +1.
    ordered.sort_by_key(|e| {
        let auth = match e.authentication {
            AuthenticationRequirement::No => 0,
            AuthenticationRequirement::Unknown => 1,
            AuthenticationRequirement::Yes => 2,
        };
        let method = e.method.to_ascii_uppercase();
        let reads = matches!(method.as_str(), "GET" | "HEAD" | "OPTIONS" | "JOB" | "CLI");
        let admin = e.path.to_ascii_lowercase().contains("admin");
        auth + 2 * usize::from(reads)
            + usize::from(e.sensitive_data.is_empty())
            + usize::from(!admin)
    });
    ordered
}

/// Rounds of at most [`ROUND_ENTRY_POINTS`] entry points over [`ROUND_FILES`] files.
fn batches<'a>(ordered: &[&'a EntryPoint]) -> Vec<Vec<&'a EntryPoint>> {
    let mut batches: Vec<Vec<&EntryPoint>> = Vec::new();
    let mut files: BTreeSet<&str> = BTreeSet::new();
    for entry in ordered {
        let full = batches.last().is_none_or(|b| {
            b.len() >= ROUND_ENTRY_POINTS
                || !files.contains(entry.file.as_str()) && files.len() >= ROUND_FILES
        });
        if full {
            batches.push(Vec::new());
            files.clear();
        }
        files.insert(&entry.file);
        if let Some(batch) = batches.last_mut() {
            batch.push(entry);
        }
    }
    batches
}

/// Checks a hypothesis against the commit and the map (base prompt principle 3). Returns
/// the cleaned content and the owner-facing notes, or why it was dropped.
fn validate(
    mut content: HypothesisContent,
    run: &AiRun,
    map: &SecurityMap,
) -> Result<(HypothesisContent, Vec<String>), String> {
    let mut notes = Vec::new();
    let location = &mut content.location;
    location.file = location
        .file
        .trim()
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_owned();
    if !run.files.contains(&location.file) {
        return Err(format!("{} ({})", content.title, location.file));
    }
    let lines = run.files.line_count(&location.file).unwrap_or(0);
    if location
        .line_range()
        .is_none_or(|(_, last)| last as usize > lines)
    {
        notes.push(format!(
            "Linhas \"{}\" removidas: fora do arquivo {}.",
            location.lines, location.file
        ));
        location.lines = String::new();
    }
    if let Some(id) = &content.entry_point
        && !map.content.entry_points.iter().any(|e| &e.id == id)
    {
        notes.push(format!("Entrada {id} inexistente no mapa: removida."));
        content.entry_point = None;
    }
    let mut known = Vec::new();
    for id in &content.asvs {
        match run.catalog.resolve(id) {
            Some(r) if !known.contains(&r.id) => known.push(r.id.clone()),
            Some(_) => {}
            None => notes.push(format!("Requisito ASVS inexistente ignorado: {id}.")),
        }
    }
    content.asvs = known;
    let cwe = content.cwe.trim().to_ascii_uppercase();
    content.cwe = match cwe.strip_prefix("CWE-") {
        Some(digits) if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) => cwe,
        _ => {
            if !content.cwe.trim().is_empty() {
                notes.push(format!("CWE inválido removido: {}.", content.cwe));
            }
            String::new()
        }
    };
    Ok((content, notes))
}

#[cfg(test)]
mod tests {
    use opentatu_core::AuthenticationRequirement as Auth;

    use super::*;

    fn entry(id: &str, method: &str, path: &str, file: &str, auth: Auth) -> EntryPoint {
        EntryPoint {
            id: id.into(),
            method: method.into(),
            path: path.into(),
            file: file.into(),
            line: Some(1),
            symbol: "h".into(),
            authentication: auth,
            allowed_roles: vec![],
            inputs: vec![],
            authorization_check: String::new(),
            sensitive_data: vec![],
            applicable_asvs: vec![],
        }
    }

    #[test]
    fn unauthenticated_writes_and_admin_paths_come_first() {
        let entries = [
            entry("EP-001", "GET", "/health", "a.go", Auth::No),
            entry("EP-002", "GET", "/me", "b.go", Auth::Yes),
            entry("EP-003", "POST", "/login", "c.go", Auth::No),
            entry("EP-004", "POST", "/admin/users", "d.go", Auth::Yes),
        ];
        let ordered: Vec<_> = prioritized(&entries)
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(ordered, ["EP-003", "EP-004", "EP-001", "EP-002"]);
    }

    #[test]
    fn batches_respect_entry_point_and_file_limits() {
        let entries: Vec<EntryPoint> = (0..40)
            .map(|n| {
                entry(
                    &format!("EP-{n:03}"),
                    "POST",
                    "/x",
                    &format!("f{}.go", n % 20),
                    Auth::No,
                )
            })
            .collect();
        let ordered: Vec<&EntryPoint> = entries.iter().collect();
        let batches = batches(&ordered);
        assert!(batches.iter().all(|b| b.len() <= ROUND_ENTRY_POINTS));
        for batch in &batches {
            let files: BTreeSet<_> = batch.iter().map(|e| e.file.as_str()).collect();
            assert!(files.len() <= ROUND_FILES);
        }
        assert_eq!(batches.iter().map(Vec::len).sum::<usize>(), 40);
    }
}
