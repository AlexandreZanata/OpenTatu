//! `findings.sarif` — SARIF 2.1.0 for GitHub code scanning (spec §B.4).

use std::collections::BTreeMap;

use opentatu_core::{Finding, Severity};
use serde_json::{Value, json};

/// SARIF log of `findings` (dismissed ones are left out).
pub fn findings_sarif(findings: &[Finding], tool_version: &str) -> Value {
    let mut rules: BTreeMap<String, Value> = BTreeMap::new();
    let mut results = Vec::new();
    for finding in findings.iter().filter(|f| f.is_open()) {
        let rule_id = if finding.cwe.is_empty() {
            "OPENTATU".to_owned()
        } else {
            finding.cwe.clone()
        };
        rules.entry(rule_id.clone()).or_insert_with(
            || json!({ "id": rule_id, "shortDescription": { "text": finding.category } }),
        );
        let mut region = serde_json::Map::new();
        if let Some((start, end)) = finding.location.line_range() {
            region.insert("startLine".into(), json!(start));
            region.insert("endLine".into(), json!(end));
        }
        results.push(json!({
            "ruleId": rule_id,
            "level": level(finding.severity),
            "message": { "text": format!("{}: {}", finding.finding_id, finding.title) },
            "locations": [ { "physicalLocation": {
                "artifactLocation": { "uri": finding.location.file },
                "region": region,
            } } ],
            "partialFingerprints": { "opentatu/v1": finding.fingerprint },
            "properties": {
                "finding_id": finding.finding_id,
                "severity": finding.severity,
                "state": finding.state,
                "asvs": finding.asvs,
            },
        }));
    }
    json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [ {
            "tool": { "driver": {
                "name": "OpenTatu",
                "version": tool_version,
                "informationUri": "https://github.com/AlexandreZanata/OpenTatu",
                "rules": rules.into_values().collect::<Vec<_>>(),
            } },
            "results": results,
        } ],
    })
}

fn level(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical | Severity::High => "error",
        Severity::Medium => "warning",
        Severity::Low | Severity::Info => "note",
    }
}

#[cfg(test)]
mod tests {
    use opentatu_core::{
        CodeLocation, Detection, FindingId, FindingState, Fingerprint, FingerprintInput, Slug,
    };

    use super::*;

    fn finding(state: FindingState) -> Finding {
        let slug = Slug::new("demo").unwrap();
        let detection = Detection {
            audit_id: "AUD-DEMO-20261112-0900".parse().unwrap(),
            at: "2026-11-12T09:00:00-03:00".parse().unwrap(),
        };
        Finding {
            finding_id: FindingId::new(&slug, 1),
            app: slug,
            title: "IDOR".into(),
            category: "Controle de acesso".into(),
            cwe: "CWE-639".into(),
            asvs: vec!["V8.2.2".into()],
            severity: Severity::High,
            state,
            fingerprint: Fingerprint::compute(&FingerprintInput {
                cwe: "CWE-639",
                file: "a.go",
                symbol: "f",
                snippet: "x",
            }),
            location: CodeLocation {
                file: "api/orders.go".into(),
                lines: "10-20".into(),
                symbol: "GetOrder".into(),
            },
            commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
            first_detected: detection.clone(),
            last_detected: detection,
            hypothesis_id: "AUD-DEMO-20261112-0900-H-001".parse().unwrap(),
            regression: false,
            review: None,
        }
    }

    #[test]
    fn open_findings_become_sarif_results() {
        let sarif = findings_sarif(
            &[
                finding(FindingState::Upheld),
                finding(FindingState::Dismissed),
            ],
            "0.1.0",
        );
        let results = sarif["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results.len(), 1, "dismissed left out");
        assert_eq!(results[0]["ruleId"], "CWE-639");
        assert_eq!(results[0]["level"], "error");
        assert_eq!(
            results[0]["locations"][0]["physicalLocation"]["region"]["startLine"],
            10
        );
        assert_eq!(sarif["version"], "2.1.0");
    }
}
