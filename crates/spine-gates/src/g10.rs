//! **G10 — Integrity · Reconstruction.** The one gate whose result is in no
//! report.
//!
//! GR §5.6.1 and §5.6.2: G10's result "is never in `gates` and never in
//! `Spine-Gates` — it runs after the seal, and its own result cannot be inside
//! the message `L`'s seal covers", and it raises no wire either. PB §6.3: "A
//! failure **refuses the push**, ends the run as `reconstruction-failed`
//! without a retry. The discarded `L` never becomes a git object, so the run's
//! own report is the only record."
//!
//! **The comparison itself is `spine-graph`'s and is not repeated here.**
//! DM §11 step 5 fixes it as `D_S == D_C` over byte strings, `spine-graph`
//! implements it as [`spine_graph::dump::g10_compare`] against DM §12's
//! vectors, and a second implementation of a byte comparison would be a second
//! thing to keep in agreement with those vectors. This module supplies only
//! what the *gate* adds: the shape of the run around that comparison, and the
//! rule that its answer never reaches a report.

use crate::gate::{Gate, LandingShape};
use crate::status::RunStatus;
use spine_graph::dump::{Dump, G10, g10_compare};
use spine_graph::status::Refusal;

/// DM §11 step 5, delegated. Both dumps must have been produced by *this*
/// binary in one process tree (DM §3.2), which is why a version skew is a
/// refusal rather than a comparison.
pub fn compare(scratch: &Dump, clone: &Dump) -> Result<G10, Refusal> {
    g10_compare(scratch, clone)
}

/// What the run does with the answer.
///
/// PB §6.3: "There is no deferred mode. … A repo too large to pay is a repo
/// whose landings are not proved reconstructible, and it should have to say
/// that in its own words rather than select it from a menu."
pub fn run_status(outcome: G10) -> Option<RunStatus> {
    match outcome {
        G10::Pass => None,
        G10::ReconstructionFailed => Some(RunStatus::ReconstructionFailed),
    }
}

/// GR §5.6.2, as a predicate the report assembler can assert against.
///
/// "no row" and "no wire" are two different things to an implementer reading a
/// table for the value to write (GR §6.3), so this is stated rather than left
/// to the absence of a branch.
pub fn appears_in_report(shape: LandingShape) -> bool {
    Gate::G10.runs_on(shape)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g10s_result_is_in_no_version_1_report_on_any_shape() {
        for shape in [
            LandingShape::GatedLand,
            LandingShape::Tombstone,
            LandingShape::Quick,
            LandingShape::Reseal,
        ] {
            assert!(!appears_in_report(shape));
        }
    }

    #[test]
    fn a_failure_ends_the_run_and_a_pass_says_nothing() {
        assert_eq!(
            run_status(G10::ReconstructionFailed),
            Some(RunStatus::ReconstructionFailed)
        );
        assert_eq!(run_status(G10::Pass), None);
        assert_eq!(
            RunStatus::ReconstructionFailed.to_string(),
            "reconstruction-failed"
        );
    }
}
