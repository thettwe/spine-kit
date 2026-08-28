//! PB §7.1's rule about who may reach a human signing key.
//!
//! > "any invocation that produces a `-Sig` line with a key that is not the
//! > `--ci` pipeline secret — `--sign`, `--reopen`, `--withdraw`, `--approve`,
//! > `--review`, `--break-glass`, and `--land` outside `--ci` — is **TTY-only**
//! > and refuses under `SPINE_AGENT=1`"
//!
//! Two conditions, and they are not the same test. `SPINE_AGENT=1` is an agent
//! **declaring itself**, and is honoured because an honest agent should be
//! stopped cheaply. The TTY check is the one that costs an attacker something:
//! it refuses a signing invocation with no terminal at all, which is what a CI
//! job, a cron entry and a piped script look like.
//!
//! **The residual is the corpus's own, and is repeated here rather than
//! papered over.** PB §7.1: "an agent running in a terminal whose agent socket
//! holds an unlocked signing key can satisfy the TTY check with a pty … Nothing
//! in spine closes that; a hardware key or `ssh-add -c` (confirm on use) does."
//! So this is a speed bump against accident and an honest agent, not a control
//! against a determined one, and the code should not read as though it were.

/// Why a signing invocation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// `SPINE_AGENT=1`.
    DeclaredAgent,
    /// No terminal on stdin.
    NoTty,
}

impl core::fmt::Display for Refusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Refusal::DeclaredAgent => f.write_str(
                "this invocation signs under a human key and refuses under SPINE_AGENT=1 \
                 (PB §7.1): agents never hold key material",
            ),
            Refusal::NoTty => f.write_str(
                "this invocation signs under a human key and is TTY-only (PB §7.1): \
                 there is no terminal to confirm the key touch on",
            ),
        }
    }
}

impl core::error::Error for Refusal {}

/// The check, over an environment and a terminal answer.
///
/// Both are arguments so the rule is testable: a test binary's own environment
/// is shared and racy, and its stdin is whatever the harness attached.
pub fn check(declares_agent: bool, has_tty: bool) -> Result<(), Refusal> {
    if declares_agent {
        return Err(Refusal::DeclaredAgent);
    }
    if !has_tty {
        return Err(Refusal::NoTty);
    }
    Ok(())
}

/// [`check`] over this process.
pub fn check_this_process() -> Result<(), Refusal> {
    check(declares_agent(), stdin_is_a_tty())
}

fn declares_agent() -> bool {
    std::env::var_os("SPINE_AGENT").is_some_and(|v| v == "1")
}

/// `isatty(0)`.
///
/// **stdin, not stdout.** A signing invocation's terminal is the one the key
/// touch is confirmed on, and a run whose stdout is piped — `ID=$(spine new
/// --sign …)` — still has a human at the keyboard. Testing stdout would refuse
/// the ordinary shell idiom and, worse, would pass a cron job that happened to
/// write to a terminal.
fn stdin_is_a_tty() -> bool {
    // SAFETY: `isatty` reads a descriptor number and touches no memory.
    unsafe { isatty(0) == 1 }
}

unsafe extern "C" {
    fn isatty(fd: i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ordinary human case.
    #[test]
    fn a_terminal_and_no_declaration_signs() {
        assert_eq!(check(false, true), Ok(()));
    }

    /// "refuses under `SPINE_AGENT=1`" — and it refuses **even with a
    /// terminal**, which is the whole point: an agent with a pty is exactly
    /// what the declaration is for.
    #[test]
    fn a_declared_agent_refuses_even_on_a_terminal() {
        assert_eq!(check(true, true), Err(Refusal::DeclaredAgent));
        assert_eq!(check(true, false), Err(Refusal::DeclaredAgent));
    }

    /// "is TTY-only" — a CI job, a cron entry and a piped script have no
    /// terminal and none of them may sign under a human key.
    #[test]
    fn no_terminal_refuses() {
        assert_eq!(check(false, false), Err(Refusal::NoTty));
    }

    /// The declaration is checked first, so an honest agent gets the message
    /// that tells it what it did rather than one about terminals.
    #[test]
    fn the_declaration_answers_before_the_terminal_does() {
        let refusal = check(true, false).unwrap_err();
        assert_eq!(refusal, Refusal::DeclaredAgent);
        assert!(refusal.to_string().contains("SPINE_AGENT=1"));
    }
}
