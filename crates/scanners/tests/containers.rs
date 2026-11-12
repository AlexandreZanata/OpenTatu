//! Real containers: refreshes the offline data (network, first run only — about 2 GB, cached
//! under `target/tmp/`) and scans a synthetic vulnerable repository with every scanner.
//!
//! Needs Docker (or Podman, per `config/scanners.toml`) and `git`:
//! `cargo test -p opentatu-scanners --test containers -- --ignored --nocapture`.

#![allow(clippy::unwrap_used)] // test helpers outside #[test] functions

use std::path::Path;

use opentatu_core::{DataLayout, ScannerKind};
use opentatu_scanners::{ScannerSuite, Scanners, ScannersConfig};

/// Fake token assembled at runtime so this source file never contains one.
fn fake_token() -> String {
    format!("ghp_{}{}", "ese1eeXvqogDN5zz7p7D3", "qC1Sr1Y63faBljU")
}

fn write(root: &Path, file: &str, contents: &str) {
    let path = root.join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn git(dir: &Path, args: &[&str]) {
    // Test fixture: a throwaway repository (R12 allows processes in this crate).
    #[allow(clippy::disallowed_methods)]
    let status = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.test",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn vulnerable_repository(dir: &Path) {
    write(
        dir,
        "Dockerfile",
        "FROM node:14\nCOPY . /app\nCMD [\"node\", \"/app/src/orders.js\"]\n",
    );
    write(
        dir,
        "package-lock.json",
        r#"{"name":"demo","version":"1.0.0","lockfileVersion":3,"requires":true,"packages":{"":{"name":"demo","version":"1.0.0","dependencies":{"lodash":"4.17.15"}},"node_modules/lodash":{"version":"4.17.15","resolved":"https://registry.npmjs.org/lodash/-/lodash-4.17.15.tgz"}}}"#,
    );
    write(
        dir,
        "src/orders.js",
        "const express = require(\"express\");\nconst app = express();\napp.get(\"/hello\", (req, res) => {\n  res.send(\"<h1>Hello \" + req.query.name + \"</h1>\");\n});\nmodule.exports = app;\n",
    );
    write(
        dir,
        "src/config.js",
        &format!(
            "export const config = {{ githubToken: \"{}\" }};\n",
            fake_token()
        ),
    );
    git(dir, &["init", "--quiet"]);
    git(dir, &["add", "."]);
    git(dir, &["commit", "--quiet", "-m", "demo"]);
}

#[tokio::test]
#[ignore = "needs a container engine and network on the first run"]
async fn every_scanner_finds_something_in_a_vulnerable_repository() {
    let config = ScannersConfig::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/scanners.toml"),
    )
    .unwrap();
    let data = Path::new(env!("CARGO_TARGET_TMPDIR")).join("scanners-e2e");
    let scanners = Scanners::new(config, DataLayout::new(&data));
    let repo = tempfile::tempdir().unwrap();
    vulnerable_repository(repo.path());
    let output = tempfile::tempdir().unwrap();

    for kind in scanners.enabled() {
        if !scanners.ready(kind) {
            eprintln!("updating {kind}…");
            scanners.update(kind).await.unwrap();
        }
        assert!(scanners.ready(kind), "{kind} not ready after update");
        let run = scanners
            .run(kind, repo.path(), output.path())
            .await
            .unwrap();
        eprintln!(
            "{kind}: {} results in {} ms",
            run.results.len(),
            run.duration_ms
        );
        assert!(!run.results.is_empty(), "{kind} found nothing");
        let expected = match kind {
            ScannerKind::Semgrep => "src/orders.js",
            ScannerKind::Gitleaks => "src/config.js",
            ScannerKind::Trivy | ScannerKind::Osv => "package-lock.json",
        };
        assert!(
            run.results
                .iter()
                .any(|r| r.file.as_deref() == Some(expected)),
            "{kind}"
        );
    }

    // R15: neither the raw outputs nor the normalized results contain the secret.
    for entry in std::fs::read_dir(output.path()).unwrap() {
        let raw = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        assert!(!raw.contains(&fake_token()));
    }
}
