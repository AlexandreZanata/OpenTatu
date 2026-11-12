//! `cargo xtask` — repository automation (not part of the product).
//!
//! - `cargo xtask check` — everything CI runs: rustfmt check, clippy with warnings as
//!   errors, the whole test suite (which includes the repository checks below) and rustdoc
//!   with warnings as errors (no broken intra-doc links).
//! - `cargo xtask arch` — crate dependency rules of `architecture.toml`.
//! - `cargo xtask docs` — AGENTS.md/CLAUDE.md per crate, catalog coverage, recipe index,
//!   relative links in Markdown.
//! - `cargo xtask glossary` — spec (pt-BR) terms never appear in code, comments or docs.
//!
//! The repository checks are also unit tests of this crate, so `cargo test --workspace`
//! enforces them too.

// A command-line tool: printing is its job.
#![allow(clippy::print_stdout, clippy::print_stderr)]
// Runs cargo subprocesses; R12 (no external processes) applies to the product, not tooling.
#![allow(clippy::disallowed_methods)]

mod architecture;
mod docs;
mod glossary;
mod repo;

use std::process::{Command, ExitCode};

const USAGE: &str = "usage: cargo xtask <check | arch | docs | glossary>";

fn main() -> ExitCode {
    let root = repo::root();
    match std::env::args().nth(1).as_deref() {
        Some("check") => check_all(),
        Some("arch") => report("architecture", &architecture::check(&root)),
        Some("docs") => report("documentation", &docs::check(&root)),
        Some("glossary") => report("glossary", &glossary::check(&root)),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// One `cargo` invocation of `check`: name, arguments and extra environment.
struct Step {
    name: &'static str,
    args: &'static [&'static str],
    env: &'static [(&'static str, &'static str)],
}

/// The steps of `cargo xtask check`, in order. CI runs exactly these.
const STEPS: &[Step] = &[
    Step {
        name: "format",
        args: &["fmt", "--all", "--check"],
        env: &[],
    },
    Step {
        name: "clippy",
        args: &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        env: &[],
    },
    Step {
        name: "tests (includes architecture, docs and glossary checks)",
        args: &["test", "--workspace"],
        env: &[],
    },
    Step {
        name: "rustdoc",
        args: &["doc", "--workspace", "--no-deps"],
        env: &[("RUSTDOCFLAGS", "-D warnings")],
    },
];

/// Runs the same steps as CI, stopping at the first failure.
fn check_all() -> ExitCode {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    for Step { name, args, env } in STEPS {
        println!("==> {name}");
        let ok = Command::new(&cargo)
            .args(*args)
            .envs(env.iter().copied())
            .current_dir(repo::root())
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            eprintln!("FAILED: {name} (cargo {})", args.join(" "));
            return ExitCode::FAILURE;
        }
    }
    println!("All checks passed.");
    ExitCode::SUCCESS
}

/// Prints the problems of one repository check.
fn report(name: &str, problems: &[String]) -> ExitCode {
    if problems.is_empty() {
        println!("{name}: ok");
        return ExitCode::SUCCESS;
    }
    eprintln!("{name}: {} problem(s)", problems.len());
    for problem in problems {
        eprintln!("  - {problem}");
    }
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_clean(name: &str, problems: &[String]) {
        assert!(
            problems.is_empty(),
            "{name} check failed:\n  - {}",
            problems.join("\n  - ")
        );
    }

    #[test]
    fn architecture_rules_hold() {
        assert_clean("architecture", &architecture::check(&repo::root()));
    }

    #[test]
    fn documentation_is_complete() {
        assert_clean("documentation", &docs::check(&repo::root()));
    }

    #[test]
    fn spec_terms_stay_out_of_code_and_docs() {
        assert_clean("glossary", &glossary::check(&repo::root()));
    }
}
