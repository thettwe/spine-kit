//! `spine new` — dispatch, and what each of PB §11's four forms still owes.
//!
//! Three of the four *sign*, and PB §7.1 puts them behind the same rule
//! `spine check`'s signing invocations sit behind: "any invocation that
//! produces a `-Sig` line with a key that is not the `--ci` pipeline secret —
//! `--sign`, `--reopen`, `--withdraw` … — is TTY-only and refuses under
//! `SPINE_AGENT=1`". That refusal is a property of the invocation, so it is
//! decided from the parsed form before anything is read.

use std::process::ExitCode;

use crate::argv::New;
use crate::exit;

/// PB §7.1's three signing forms of this command. The creation form writes a
/// branch and a scaffold and signs nothing.
fn signs_with_a_human_key(new: &New) -> bool {
    !matches!(new, New::Create { .. })
}

pub fn run(new: &New) -> ExitCode {
    if signs_with_a_human_key(new) && std::env::var_os("SPINE_AGENT").is_some_and(|v| v == "1") {
        eprintln!(
            "spine new: this invocation signs under a human key and refuses under \
             SPINE_AGENT=1 (PB §7.1)"
        );
        return ExitCode::from(exit::REFUSED);
    }

    let owed = match new {
        New::Create { from: Some(_), .. } => "--from <quick-branch>",
        New::Create { variant, .. } => {
            // Named so that the diagnostic says which of PB §3.5's three
            // templates was asked for, rather than reporting one gap for
            // three commands.
            eprintln!(
                "spine new: the scaffold would be `{}@<n>` from the manifest's `templates`",
                variant.template_name()
            );
            "the interview and the scaffold"
        }
        New::Sign { .. } => "--sign",
        New::Reopen { .. } => "--reopen",
        New::Withdraw { .. } => "--withdraw",
    };
    eprintln!("spine new: {owed} is not yet implemented");
    ExitCode::from(exit::ERROR)
}
