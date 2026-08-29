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
/// PB §7.1's rule, both halves: "is **TTY-only** and refuses under
/// `SPINE_AGENT=1`". `crate::tty` owns it so `spine new`'s three signing forms
/// and this command's four cannot drift apart.
fn signing_refusal(check: &Check) -> Option<crate::tty::Refusal> {
    check
        .signs_with_a_human_key()
        .then(crate::tty::check_this_process)
        .and_then(Result::err)
}

pub fn run(check: &Check) -> ExitCode {
    if let Some(refusal) = signing_refusal(check) {
        eprintln!("spine check: {refusal}");
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
        return crate::collect::run(check);
    } else if let Some(id) = &check.approve {
        return crate::approve::run(id, check.reason.as_deref());
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
