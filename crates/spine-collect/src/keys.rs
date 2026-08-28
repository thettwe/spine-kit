//! RF §4.2's `keys_visible` predicate — step 4 of RF §7.1.
//!
//! > "`keys_visible=false` asserts that **no signing key material of any kind**
//! > was reachable from the collector process or from any process group it
//! > spawned — *every* runner invocation included: no variable named
//! > `SPINE_PIPELINE_KEY` (§11, Files and refs) nor any provider-specific
//! > pipeline-key name that `ci.md` fixes, **and** no signing agent or private
//! > key — `SSH_AUTH_SOCK`, `GPG_AGENT_INFO`, a readable `~/.ssh` or
//! > `~/.gnupg`, the set §7.1 names when it says what a sandbox strips."
//!
//! **It observes and never sanitizes.** CI §14 R6 is explicit about the
//! sibling rule: "`ci.sh` **does not strip** those variables. Stripping would
//! launder a misconfigured pipeline into a passing assertion." A collector that
//! scrubbed its own environment to earn `false` would be doing exactly that —
//! the field is evidence about the job it ran in, not a property it can
//! arrange.
//!
//! **One assertion covers the whole job.** RF §4.2: "the field is not
//! per-runner, and a collector that strips key material for one runner and not
//! another writes `true`." So this reads the collector's own environment once,
//! at step 4, before any runner is spawned — which is honest only under an
//! invariant the corpus states for the restore phase and nowhere else: no
//! spawned environment may carry key material the probe did not read. RF §7.1
//! states it for that phase — "**Its environment is the collector's own**,
//! unchanged, which is what §4.2's `keys_visible` predicate already covers by
//! its first conjunct" — and [`Probe::spawned_environment_is_a_subset`] is the
//! name this crate gives the rule so an adapter author meets it.

use std::path::{Path, PathBuf};

/// The seven checks, named so a diagnostic can say which one answered.
///
/// `ci.md` closes RF's "any provider-specific pipeline-key name that `ci.md`
/// fixes" to a literal list — the three `SPINE_*` variables of CI §6.1's table,
/// which are also the three `.spine/ci.sh`'s own rule-0 loop probes — so there
/// is nothing here to guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMaterial {
    /// CI §6.1: "The OpenSSH **private key** of the `spine-seal@v1` principal,
    /// as key material, not a path."
    PipelineKey,
    /// CI §6.1: the credential the trusted job pushes trunk with.
    PushToken,
    /// CI §6.1: "The SSH alternative to `SPINE_PUSH_TOKEN`."
    PushKey,
    /// PB §7.1's sandbox-strips set: a running ssh-agent.
    SshAgent,
    /// PB §7.1's sandbox-strips set: a running gpg-agent.
    GpgAgent,
    /// PB §7.1: "a readable `~/.ssh`".
    SshDirectory,
    /// PB §7.1: "a readable `~/.gnupg`".
    GnupgDirectory,
}

impl KeyMaterial {
    pub fn name(self) -> &'static str {
        match self {
            KeyMaterial::PipelineKey => "SPINE_PIPELINE_KEY",
            KeyMaterial::PushToken => "SPINE_PUSH_TOKEN",
            KeyMaterial::PushKey => "SPINE_PUSH_KEY",
            KeyMaterial::SshAgent => "SSH_AUTH_SOCK",
            KeyMaterial::GpgAgent => "GPG_AGENT_INFO",
            KeyMaterial::SshDirectory => "~/.ssh",
            KeyMaterial::GnupgDirectory => "~/.gnupg",
        }
    }
}

/// The three environment variables CI §6.1's table marks **secret**.
pub const SECRET_VARIABLES: [(KeyMaterial, &str); 5] = [
    (KeyMaterial::PipelineKey, "SPINE_PIPELINE_KEY"),
    (KeyMaterial::PushToken, "SPINE_PUSH_TOKEN"),
    (KeyMaterial::PushKey, "SPINE_PUSH_KEY"),
    (KeyMaterial::SshAgent, "SSH_AUTH_SOCK"),
    (KeyMaterial::GpgAgent, "GPG_AGENT_INFO"),
];

/// What the probe observed. Empty is `keys_visible=false`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Probe {
    pub reachable: Vec<KeyMaterial>,
}

impl Probe {
    /// RF §4.2's field.
    ///
    /// "`true` is the honest negation, and the collector writes it rather than
    /// omitting the field."
    pub fn keys_visible(&self) -> bool {
        !self.reachable.is_empty()
    }

    /// The stderr line naming what was reachable, for the human reading a
    /// `keys_visible=true` on a run that expected `false`.
    pub fn diagnostic(&self) -> Option<String> {
        (!self.reachable.is_empty()).then(|| {
            let names: Vec<&str> = self.reachable.iter().map(|k| k.name()).collect();
            format!(
                "keys_visible=true: signing key material is reachable ({})",
                names.join(", ")
            )
        })
    }

    /// The invariant that makes a step-4 reading honest about "every runner
    /// invocation": no environment the collector spawns may carry key material
    /// this probe did not read.
    ///
    /// RF §7.1 states it for the restore phase alone — "**Its environment is
    /// the collector's own**, unchanged, which is what §4.2's `keys_visible`
    /// predicate already covers by its first conjunct — the phase adds no
    /// environment the predicate does not already read." An adapter may *add*
    /// environment (a runner plugin path, say); what it may not do is add a
    /// name in [`SECRET_VARIABLES`] or point `HOME` at a directory with keys
    /// in it.
    pub fn spawned_environment_is_a_subset(added: &[(&str, &str)]) -> bool {
        !added.iter().any(|(name, _)| {
            SECRET_VARIABLES.iter().any(|(_, secret)| name == secret) || *name == "HOME"
        })
    }
}

/// Read the predicate over an environment and a home directory.
///
/// Taken as arguments rather than read from the process so the seven checks are
/// testable without setting environment variables in a test binary — which is
/// shared, racy under `cargo test`'s threads, and would make this the one
/// module whose tests could not run in parallel.
pub fn probe<'a>(
    env: impl Fn(&str) -> Option<&'a str>,
    home: Option<&Path>,
    readable: &dyn Fn(&Path) -> bool,
) -> Probe {
    let mut reachable = Vec::new();

    for (material, name) in SECRET_VARIABLES {
        // **Set, not non-empty.** `.spine/ci.sh`'s rule-0 loop tests
        // `${$_v+set}` — presence, whatever the value — and an empty
        // `SPINE_PIPELINE_KEY` is still a pipeline that was configured to pass
        // one here.
        if env(name).is_some() {
            reachable.push(material);
        }
    }

    if let Some(home) = home {
        for (material, dir) in [
            (KeyMaterial::SshDirectory, ".ssh"),
            (KeyMaterial::GnupgDirectory, ".gnupg"),
        ] {
            if readable(&home.join(dir)) {
                reachable.push(material);
            }
        }
    }

    Probe { reachable }
}

/// [`probe`] over this process.
pub fn probe_this_process() -> Probe {
    let env = |name: &str| std::env::var(name).ok();
    let owned: Vec<(KeyMaterial, Option<String>)> =
        SECRET_VARIABLES.iter().map(|(m, n)| (*m, env(n))).collect();
    let home = std::env::var_os("HOME").map(PathBuf::from);

    let mut reachable: Vec<KeyMaterial> = owned
        .into_iter()
        .filter_map(|(m, v)| v.map(|_| m))
        .collect();
    if let Some(home) = home {
        for (material, dir) in [
            (KeyMaterial::SshDirectory, ".ssh"),
            (KeyMaterial::GnupgDirectory, ".gnupg"),
        ] {
            // "a **readable** `~/.ssh`" — existence is not the test; a
            // directory the collector cannot open holds nothing it can reach.
            if std::fs::read_dir(home.join(dir)).is_ok() {
                reachable.push(material);
            }
        }
    }
    Probe { reachable }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_env(_: &str) -> Option<&'static str> {
        None
    }

    fn unreadable(_: &Path) -> bool {
        false
    }

    /// PB §7.4 rule 0's job: nothing reachable, so `false`.
    #[test]
    fn an_untrusted_job_with_no_secrets_writes_false() {
        let seen = probe(no_env, Some(Path::new("/home/runner")), &unreadable);
        assert!(!seen.keys_visible());
        assert_eq!(seen.diagnostic(), None);
    }

    /// RF §4.2 and CI §6.1's three, plus PB §7.1's two agents. Each on its own,
    /// because a predicate that only fires on the combination fires on nothing.
    #[test]
    fn each_of_the_five_variables_answers_alone() {
        for (material, name) in SECRET_VARIABLES {
            let seen = probe(
                |asked| (asked == name).then_some("anything"),
                None,
                &unreadable,
            );
            assert!(seen.keys_visible(), "{name} did not answer");
            assert_eq!(seen.reachable, [material]);
            assert!(seen.diagnostic().unwrap().contains(name));
        }
    }

    /// `.spine/ci.sh`'s rule-0 loop tests `${$_v+set}` — presence, whatever the
    /// value. An empty `SPINE_PIPELINE_KEY` is still a pipeline configured to
    /// pass one to this job.
    #[test]
    fn an_empty_value_is_still_set() {
        let seen = probe(
            |asked| (asked == "SPINE_PIPELINE_KEY").then_some(""),
            None,
            &unreadable,
        );
        assert!(seen.keys_visible());
    }

    /// PB §7.1: "a **readable** `~/.ssh` or `~/.gnupg`".
    #[test]
    fn a_readable_key_directory_answers_and_an_unreadable_one_does_not() {
        let home = Path::new("/home/dev");
        let ssh = home.join(".ssh");
        let seen = probe(no_env, Some(home), &|p: &Path| p == ssh);
        assert_eq!(seen.reachable, [KeyMaterial::SshDirectory]);

        let gnupg = home.join(".gnupg");
        let seen = probe(no_env, Some(home), &|p: &Path| p == gnupg);
        assert_eq!(seen.reachable, [KeyMaterial::GnupgDirectory]);

        assert!(!probe(no_env, Some(home), &unreadable).keys_visible());
    }

    /// RF §4.2: "One assertion covers the whole job … `true` is the solo path's,
    /// where the operator's own signing key is reachable from the process tree
    /// that ran the tests."
    #[test]
    fn the_solo_path_writes_true_and_the_diagnostic_names_every_reason() {
        let home = Path::new("/home/dev");
        let seen = probe(
            |asked| (asked == "SSH_AUTH_SOCK").then_some("/tmp/agent.sock"),
            Some(home),
            &|p: &Path| p.ends_with(".ssh"),
        );
        assert!(seen.keys_visible());
        let diagnostic = seen.diagnostic().unwrap();
        assert!(diagnostic.contains("SSH_AUTH_SOCK"), "{diagnostic}");
        assert!(diagnostic.contains("~/.ssh"), "{diagnostic}");
    }

    /// CI §14 R6: "`ci.sh` **does not strip** those variables. Stripping would
    /// launder a misconfigured pipeline into a passing assertion." The
    /// invariant an adapter must meet, made checkable.
    #[test]
    fn an_adapter_may_add_environment_but_not_key_material() {
        assert!(Probe::spawned_environment_is_a_subset(&[
            ("PYTEST_PLUGINS", "spine_adapter"),
            ("PYTHONHASHSEED", "0"),
        ]));
        assert!(!Probe::spawned_environment_is_a_subset(&[(
            "SPINE_PIPELINE_KEY",
            "..."
        )]));
        // `HOME` too: repointing it moves `~/.ssh` and would launder the
        // directory half of the predicate.
        assert!(!Probe::spawned_environment_is_a_subset(&[(
            "HOME",
            "/tmp/empty"
        )]));
    }
}
