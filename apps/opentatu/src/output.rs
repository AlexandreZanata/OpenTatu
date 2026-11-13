//! Terminal output. The only module allowed to print (clippy `print_stdout`/`print_stderr`).
#![allow(clippy::print_stdout, clippy::print_stderr)]

/// Prints one line of command output to stdout.
pub fn line(text: &str) {
    println!("{text}");
}

/// Prints structured non-secret owner output through the existing terminal boundary.
pub fn json(value: &serde_json::Value) -> anyhow::Result<()> {
    line(&serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Prints an error and its chain of causes to stderr.
pub fn error(error: &anyhow::Error) {
    eprintln!("error: {error}");
    for cause in error.chain().skip(1) {
        eprintln!("  caused by: {cause}");
    }
}
