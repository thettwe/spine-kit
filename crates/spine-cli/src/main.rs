//! `spine` — the four commands.
//!
//! PB §7.1 puts the CLI in the untrusted half of the design for most of what it
//! does, so the rules that shape this file are about what it *refuses*:
//! diagnostics go to stderr, the exit code is the machine-readable answer, and
//! a refusal leaves the repository exactly as it was.

mod allocate;
mod approve;
mod argv;
mod check;
mod collect;
mod g12;
mod index;
mod init;
mod new;
mod sign;
mod tree_source;
mod tty;

use std::process::ExitCode;

/// Exit codes.
///
/// PB §6.7 fixes one of these directly — `--dry-run` "exits 0, or 2 if it would
/// refuse" — and `ci.md` §5.5 uses 2 for a refused platform. The rest follow
/// that: **2 is a refusal**, the outcome the design reaches for everywhere, and
/// 1 is reserved for the tool failing rather than deciding.
mod exit {
    pub const OK: u8 = 0;
    /// The tool itself failed: git missing, unreadable repository, IO error.
    pub const ERROR: u8 = 1;
    /// A refusal — the design decided, and the answer is no.
    pub const REFUSED: u8 = 2;
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let command = match argv::parse(&args) {
        Ok(command) => command,
        Err(e) => {
            // "every diagnostic, and all of the collector's own output, goes to
            // stderr" (ci.md §5.1) — the same rule holds for the CLI, so a
            // caller piping stdout gets only the artifact it asked for.
            eprintln!("spine: {e}");
            return ExitCode::from(exit::REFUSED);
        }
    };

    match command {
        argv::Command::Init(options) => match init::run(options.as_ref()) {
            Ok(code) => ExitCode::from(code),
            Err(e) => {
                eprintln!("spine init: {e}");
                ExitCode::from(exit::ERROR)
            }
        },
        argv::Command::New(options) => new::run(options.as_ref()),
        argv::Command::Index { fresh, dump } => index::run(fresh, dump),
        argv::Command::Check(options) => check::run(options.as_ref()),
    }
}
