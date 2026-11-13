//! [`ScannerSuite`] — the port the worker uses — and [`Scanners`], its container-backed
//! implementation.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use opentatu_core::{DataLayout, ScannerKind, ScannerRun};

use crate::adapters::{AnalysisDirs, adapter};
use crate::{ContainerRun, Error, ScannersConfig, engine};

/// Runs deterministic scanners on a snapshot. Implemented by [`Scanners`] (real containers)
/// and by `testing::FakeSuite` (feature `test-support`).
#[async_trait::async_trait]
pub trait ScannerSuite: Send + Sync + fmt::Debug {
    /// Scanners audits run, in a stable order.
    fn enabled(&self) -> Vec<ScannerKind>;

    /// Whether the offline data of `kind` exists (`opentatu scanners update`).
    fn ready(&self, kind: ScannerKind) -> bool;

    /// Runs `kind` offline on `source_dir` (read-only), writing its raw output to
    /// `output_dir/<tool>.json`, and returns the normalized results.
    async fn run(
        &self,
        kind: ScannerKind,
        source_dir: &Path,
        output_dir: &Path,
    ) -> Result<ScannerRun, Error>;
}

/// Scanners in isolated containers, configured by `config/scanners.toml`.
#[derive(Debug, Clone)]
pub struct Scanners {
    config: ScannersConfig,
    layout: DataLayout,
}

impl Scanners {
    /// Scanners using `config`, with caches under the data directory of `layout`.
    pub fn new(config: ScannersConfig, layout: DataLayout) -> Self {
        Self { config, layout }
    }

    /// The configuration in use.
    pub fn config(&self) -> &ScannersConfig {
        &self.config
    }

    /// Offline data directory of `kind`.
    pub fn cache_dir(&self, kind: ScannerKind) -> PathBuf {
        self.layout.scanner_cache_dir(kind)
    }

    /// Refreshes the rules/databases of `kind` (network access, never the source).
    pub async fn update(&self, kind: ScannerKind) -> Result<(), Error> {
        let tool = self.config.tool(kind).ok_or(Error::Disabled(kind))?;
        let adapter = adapter(kind)?;
        let cache = self.cache_dir(kind);
        create_dir(&cache)?;
        let seed = self
            .layout
            .root()
            .join("scanner-cache")
            .join(".seed")
            .join(kind.as_str());
        remove_dir(&seed)?;
        create_dir(&seed)?;
        let runs = adapter.updates(tool, &cache, &seed)?;
        let outcome = async {
            for run in runs {
                let finished = engine::execute(&self.prepare(run, &cache), self.timeout()).await?;
                if !adapter.succeeded(finished.code) {
                    return Err(Error::Failed {
                        scanner: kind,
                        code: finished.code,
                        stderr: finished.stderr_tail,
                    });
                }
            }
            adapter.finish_update(&cache)
        }
        .await;
        remove_dir(&seed)?;
        outcome
    }

    /// Wall-clock limit of one container run.
    fn timeout(&self) -> Duration {
        Duration::from_secs(self.config.engine.timeout_minutes.saturating_mul(60))
    }

    /// Fills the engine and runs the tool as the owner of `writable_dir`, so its files stay
    /// owned by the data directory's user.
    fn prepare(&self, mut run: ContainerRun, writable_dir: &Path) -> ContainerRun {
        run.engine = self.config.engine.command;
        run.user = owner(writable_dir);
        run
    }
}

#[async_trait::async_trait]
impl ScannerSuite for Scanners {
    fn enabled(&self) -> Vec<ScannerKind> {
        ScannerKind::STATIC
            .iter()
            .copied()
            .filter(|kind| self.config.tool(*kind).is_some_and(|tool| tool.enabled))
            .collect()
    }

    fn ready(&self, kind: ScannerKind) -> bool {
        adapter(kind).is_ok_and(|adapter| adapter.ready(&self.cache_dir(kind)))
    }

    async fn run(
        &self,
        kind: ScannerKind,
        source_dir: &Path,
        output_dir: &Path,
    ) -> Result<ScannerRun, Error> {
        let tool = self
            .config
            .tool(kind)
            .filter(|tool| tool.enabled)
            .ok_or(Error::Disabled(kind))?;
        if !self.ready(kind) {
            return Err(Error::NotReady(kind));
        }
        let adapter = adapter(kind)?;
        create_dir(output_dir)?;
        let output_file = output_dir.join(adapter.output_file());
        remove_file(&output_file)?;
        let dirs = AnalysisDirs {
            source: source_dir.to_owned(),
            output: output_dir.to_owned(),
            cache: self.cache_dir(kind),
        };
        let run = self.prepare(adapter.analysis(tool, &dirs), output_dir);
        let started = Instant::now();
        let finished = engine::execute(&run, self.timeout()).await?;
        let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        if !adapter.succeeded(finished.code) {
            return Err(Error::Failed {
                scanner: kind,
                code: finished.code,
                stderr: finished.stderr_tail,
            });
        }
        let raw = match std::fs::read(&output_file) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::MissingOutput(kind));
            }
            Err(source) => {
                return Err(Error::Io {
                    path: output_file,
                    source,
                });
            }
        };
        Ok(ScannerRun {
            scanner: kind,
            version: tool.version.clone(),
            image: tool.image.clone(),
            duration_ms,
            results: adapter.parse(&raw)?,
        })
    }
}

/// `uid:gid` owning `path` (Unix); `None` lets the image's default user run.
fn owner(path: &Path) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(path)
            .ok()
            .map(|meta| format!("{}:{}", meta.uid(), meta.gid()))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

fn create_dir(path: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(path).map_err(|source| Error::Io {
        path: path.to_owned(),
        source,
    })
}

fn remove_dir(path: &Path) -> Result<(), Error> {
    match std::fs::remove_dir_all(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(Error::Io {
            path: path.to_owned(),
            source: error,
        }),
        _ => Ok(()),
    }
}

fn remove_file(path: &Path) -> Result<(), Error> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(Error::Io {
            path: path.to_owned(),
            source: error,
        }),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scanners(root: &Path) -> Scanners {
        let config =
            ScannersConfig::from_toml(include_str!("../../../config/scanners.toml")).unwrap();
        Scanners::new(config, DataLayout::new(root))
    }

    #[tokio::test]
    async fn without_offline_data_scanners_are_not_ready() {
        let root = tempfile::tempdir().unwrap();
        let suite = scanners(root.path());
        assert_eq!(suite.enabled(), ScannerKind::STATIC);
        assert!(
            suite.ready(ScannerKind::Gitleaks),
            "gitleaks ships its rules"
        );
        assert!(!suite.ready(ScannerKind::Trivy));
        let result = suite
            .run(ScannerKind::Semgrep, root.path(), &root.path().join("out"))
            .await;
        assert!(matches!(result, Err(Error::NotReady(ScannerKind::Semgrep))));
    }

    #[test]
    fn runs_as_the_owner_of_the_data_directory() {
        let root = tempfile::tempdir().unwrap();
        let run = scanners(root.path()).prepare(
            crate::adapters::base_run(
                scanners(root.path())
                    .config()
                    .tool(ScannerKind::Gitleaks)
                    .unwrap(),
            ),
            root.path(),
        );
        assert!(run.user.is_some_and(|user| user.contains(':')));
    }
}
