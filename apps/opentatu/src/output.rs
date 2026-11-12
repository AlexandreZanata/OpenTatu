//! Terminal output. The only module allowed to print (clippy `print_stdout`/`print_stderr`).
#![allow(clippy::print_stdout, clippy::print_stderr)]

/// Prints one line of command output to stdout.
pub fn line(text: &str) {
    println!("{text}");
}

/// Prints an error and its chain of causes to stderr.
pub fn error(error: &anyhow::Error) {
    eprintln!("error: {error}");
    for cause in error.chain().skip(1) {
        eprintln!("  caused by: {cause}");
    }
}
