//! # opentatu-benchmark — how well OpenTatu finds known flaws (roadmap phase 6)
//!
//! Pure functions; the binary does the IO (`opentatu benchmark`, ADR-0031).
//!
//! - [`BenchmarkApp`]: one intentionally vulnerable app (`benchmark/apps/<name>.toml`):
//!   pinned upstream commit, marker syntax, files excluded because they hold the answers.
//! - [`markers`]: the app's own vulnerability markers — found ([`find_markers`]) to build
//!   the ground truth, and removed ([`strip_markers`]) from the copy that is audited, so
//!   the models never read the answers. Line numbers are preserved.
//! - [`Expected`] / [`expected_flaws`]: the known flaws (`benchmark/expected/<name>.json`),
//!   derived only from the markers and the app's challenge catalog — never written by hand
//!   or by a model.
//! - [`score`]: reported findings matched to known flaws by file and line → found, missed,
//!   false positives (findings outside the list), and flaws only hypothesized.
//! - [`RunResult`] / [`render_scoreboard`]: one result file per run
//!   (`benchmark/results/`) and `benchmark/SCOREBOARD.md`, per prompt version and per model.

mod app;
mod error;
mod expected;
pub mod markers;
mod score;
mod scoreboard;

pub use app::BenchmarkApp;
pub use error::Error;
pub use expected::{Challenge, Expected, ExpectedFlaw, Location, expected_flaws, parse_challenges};
pub use markers::{Marker, find_markers, strip_markers};
pub use score::{FalsePositive, FlawMatch, PipelineCounts, Score, pipeline_counts, score};
pub use scoreboard::{RunResult, render_scoreboard};
