//! `spine check` — the dispatch, and what each mode still owes.
//!
//! PB §7.1 splits this command across the trust boundary, and the split is the
//! reason the modes are dispatched separately rather than run as one pass:
//! `--collect`, `--approve` and `--constitution` execute repository code and
//! never run in the trusted stage, while `--land` *is* the trusted stage. A
//! single "run everything" entry point would have to hold both at once.
//!
//! What is not built yet says so **by name and by mode**. A blanket "not yet
//! implemented" over the whole command would let `spine check --pre-receive`
//! and `spine check --land INT-042` fail identically, and a hook that refuses
//! for the wrong reason is worse than one that is absent.

use std::process::ExitCode;

use crate::argv::{Check, Subject};
use crate::exit;

/// PB §7.1: the invocations that produce a `-Sig` line under a human key are
/// "TTY-only and refuse under `SPINE_AGENT=1`".
///
/// Checked here rather than at the signing site because it is a property of
/// the *invocation*, and the refusal must land before anything is read.
fn refuses_under_an_agent(check: &Check) -> bool {
    check.signs_with_a_human_key() && std::env::var_os("SPINE_AGENT").is_some_and(|v| v == "1")
}

pub fn run(check: &Check) -> ExitCode {
    if refuses_under_an_agent(check) {
        eprintln!(
            "spine check: this invocation signs under a human key and refuses under SPINE_AGENT=1 (PB §7.1)"
        );
        return ExitCode::from(exit::REFUSED);
    }

    // Ordered so that the diagnostic names the mode the caller asked for
    // first, not whichever is checked first by accident.
    let owed = if check.pre_receive {
        "--pre-receive"
    } else if check.verify.is_some() {
        "--verify"
    } else if check.reconstruct {
        "--reconstruct"
    } else if check.authority {
        "--authority"
    } else if check.constitution {
        "--constitution"
    } else if check.collect {
        "--collect"
    } else if check.approve.is_some() {
        "--approve"
    } else if check.review.is_some() {
        "--review"
    } else if let Some(subject) = &check.land {
        match subject {
            Subject::Intent(_) => "--land <id>",
            Subject::Quick(_) => "--land --quick <branch>",
            Subject::Reseal => "--land --reseal",
            Subject::Upgrade => "--land (upgrade)",
        }
    } else {
        // The bare form: "reported on every `spine check`" (PB §11's state
        // table) — the read-only pass over every gate.
        "the read-only gate pass"
    };

    eprintln!("spine check: {owed} is not yet implemented");
    ExitCode::from(exit::ERROR)
}
