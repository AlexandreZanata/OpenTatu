//! Passive ZAP on the exact approved GET path, within the NIC-less guest only.
use crate::{Error, LabProfile, engine, lab_guest::GuestPermit};
use opentatu_core::{LabCase, ScannerKind, ScannerResult, ScannerRun};
use std::{path::Path, time::Duration};

pub(crate) async fn run(
    permit: &GuestPermit,
    profile: &LabProfile,
    case: &LabCase,
    url: &str,
    dir: &Path,
    number: u32,
) -> Result<ScannerRun, Error> {
    let requests = [&case.control_actor, &case.subject_actor]
        .into_iter()
        .map(|actor| {
            let headers: Vec<_> = profile.actors[actor]
                .headers
                .iter()
                .map(|(k, v)| format!("{k}: {}", v.expose()))
                .collect();
            serde_json::json!({"url":url,"method":"GET","headers":headers})
        })
        .collect::<Vec<_>>();
    let plan = serde_json::json!({"env":{"contexts":[{"name":"approved-lab","urls":[url],"includePaths":[format!("http://127\\.0\\.0\\.1:{}/.*",profile.port)]}],"parameters":{"failOnError":true,"failOnWarning":false,"progressToStdout":false}},"jobs":[{"type":"requestor","requests":requests},{"type":"passiveScan-wait","parameters":{"maxDuration":1}},{"type":"report","parameters":{"template":"traditional-json","reportDir":dir,"reportFile":"result","displayReport":false}}]});
    let input = dir.join("plan.yaml");
    std::fs::write(&input, plan.to_string())
        .and_then(|()| std::os::unix::fs::chown(&input, Some(2002), Some(2002)))
        .map_err(|e| Error::Engine(e.to_string()))?;
    let argv = command_line(profile, dir, &input);
    let env = vec![
        ("JAVA_HOME".into(), "/opt/jre".into()),
        (
            "JAVA_TOOL_OPTIONS".into(),
            format!(
                "-Xmx512m -Duser.home={} -Djava.io.tmpdir={}",
                dir.display(),
                dir.display()
            ),
        ),
    ];
    let mut child = engine::guest_child(permit, 2002, &argv, dir, &env)?;
    let status = tokio::time::timeout(Duration::from_secs(240), child.wait())
        .await
        .map_err(|_| Error::Engine("passive ZAP timed out".into()))?
        .map_err(|e| Error::Engine(e.to_string()))?;
    if !status.success() {
        return Err(Error::Engine("passive ZAP plan failed".into()));
    }
    let raw = std::fs::read(dir.join("result.json")).map_err(|e| Error::Engine(e.to_string()))?;
    if raw.len() > 4 * 1024 * 1024 {
        return Err(Error::Engine("ZAP report exceeds 4 MiB".into()));
    }
    parse(&raw, &profile.zap_version, &profile.image_digest, number)
}

// The pinned launcher consumes -Xmx itself; JAVA_TOOL_OPTIONS alone is overridden by
// its automatic heap selection. Keep the cap explicit in the executed argv.
fn command_line(profile: &LabProfile, dir: &Path, input: &Path) -> Vec<String> {
    vec![
        profile.zap_path.clone(),
        "-Xmx512m".into(),
        "-cmd".into(),
        "-silent".into(),
        "-host".into(),
        "127.0.0.1".into(),
        "-port".into(),
        "8091".into(),
        "-dir".into(),
        dir.to_string_lossy().into(),
        "-autorun".into(),
        input.to_string_lossy().into(),
    ]
}

pub(crate) fn parse(
    raw: &[u8],
    version: &str,
    image: &str,
    number: u32,
) -> Result<ScannerRun, Error> {
    let value: serde_json::Value = serde_json::from_slice(raw).map_err(|_| Error::Parse {
        scanner: ScannerKind::Zap,
        message: "invalid passive report JSON".into(),
    })?;
    if value["@version"] != version {
        return Err(Error::Config(
            "ZAP version differs from pinned profile".into(),
        ));
    }
    let mut results = Vec::new();
    if let Some(sites) = value["site"].as_array() {
        for site in sites {
            if let Some(alerts) = site["alerts"].as_array() {
                for alert in alerts {
                    if results.len() >= 50 {
                        return Err(Error::Config(
                            "passive ZAP alert limit exceeded; coverage incomplete".into(),
                        ));
                    }
                    let n = results.len() + 1;
                    results.push(ScannerResult {
                        scanner: ScannerKind::Zap,
                        id: format!(
                            "zap:{}",
                            usize::try_from(number).unwrap_or(0).saturating_mul(1000) + n
                        ),
                        rule_id: alert["pluginid"].as_str().unwrap_or("unknown").to_owned(),
                        message: crate::adapters::clean_message(
                            alert["name"].as_str().unwrap_or("Passive HTTP alert"),
                        ),
                        file: None,
                        start_line: None,
                        end_line: None,
                        reported_severity: alert["riskdesc"].as_str().map(str::to_owned),
                        cwe: alert["cweid"]
                            .as_str()
                            .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
                            .map(|s| format!("CWE-{s}")),
                    });
                }
            }
        }
    }
    Ok(ScannerRun {
        scanner: ScannerKind::Zap,
        version: version.into(),
        image: image.into(),
        duration_ms: 0,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn r13_zap_launcher_gets_the_explicit_heap_cap() {
        let profile = crate::lab_config::tests::profile();
        let args = command_line(
            &profile,
            Path::new("/work/scanner"),
            Path::new("/work/scanner/plan.yaml"),
        );
        assert!(args.iter().any(|a| a == "-Xmx512m"));
        assert!(args.iter().any(|a| a == "-silent"));
        assert!(!args.iter().any(|a| a == "-daemon"));
    }

    #[test]
    fn r15_zap_parser_keeps_only_minimized_alerts_and_checks_version() {
        let raw = serde_json::json!({"@version":"2.17.0","site":[{"alerts":[{"pluginid":"10020","name":"Header absent","riskdesc":"Low","cweid":"693","evidence":"must-never-leave","instances":[{"uri":"secret-url"}]}]}]});
        let bytes = serde_json::to_vec(&raw).unwrap();
        let scan = parse(&bytes, "2.17.0", "image", 2).unwrap();
        assert_eq!(scan.results[0].id, "zap:2001");
        assert_eq!(scan.results[0].cwe.as_deref(), Some("CWE-693"));
        assert!(
            !serde_json::to_string(&scan)
                .unwrap()
                .contains("must-never-leave")
        );
        assert!(parse(&bytes, "wrong-version", "image", 2).is_err());
        let many = serde_json::json!({"@version":"2.17.0","site":[{"alerts":vec![serde_json::json!({});51]}]});
        assert!(parse(&serde_json::to_vec(&many).unwrap(), "2.17.0", "image", 1).is_err());
    }
}
