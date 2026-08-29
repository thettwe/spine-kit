//! G12's measurement — `red=k/n` and the `Spine-Test` lines.
//!
//! PB §6.3: "`red=k/n` recorded at `--approve`, **measured with the intent's
//! `expected` paths restored to base**". `spine-init` builds that tree; this
//! runs the frozen tests against it and counts.
//!
//! **This is not `--collect`.** The collector writes RF §4's result file over
//! `B` and `T` and is bound by RF §7's ten steps; G12 needs one run against a
//! tree no commit holds, contributes no result file, and is raised "by
//! `--approve` and never by `--land`". They share the host and the adapters and
//! nothing else.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use spine_collect::collector::{Checkout, Host, RunnerAdapter};
use spine_init::Repo;
use spine_runner::LocalHost;
use spine_runner::pytest::Pytest;

/// What the measurement produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Measured {
    /// `Spine-Test` payloads: `<runner> <fn>`.
    ///
    /// PB §4.3: "`Spine-Test` ids are collected *function* ids **without
    /// parametrization suffixes**, qualified by the runner that collected them,
    /// since a repository may run several (`params.langs`); the pair is the
    /// identity, and G1 requires every collected parametrization of each to
    /// pass."
    ///
    /// A set, because the pair is the identity: two parametrizations of one
    /// function are one `Spine-Test` line.
    pub tests: BTreeSet<String>,
    /// `k` — "how many frozen tests failed at approval".
    pub red: usize,
    /// `n` — the frozen tests the run collected.
    pub total: usize,
    /// Frozen paths for which the run collected nothing. Reported rather than
    /// folded into `n`: a test file the runner did not collect is a fact a
    /// human needs before signing, and counting it as neither red nor green
    /// would hide it.
    pub uncollected: BTreeSet<String>,
}

#[derive(Debug)]
pub enum MeasureError {
    Checkout(String),
    NoAdapter,
}

impl core::fmt::Display for MeasureError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MeasureError::Checkout(e) => {
                write!(f, "the base-restored tree would not check out: {e}")
            }
            MeasureError::NoAdapter => f.write_str(
                "no runner adapter for this repository's languages, so the frozen tests \
                 cannot be measured",
            ),
        }
    }
}

impl core::error::Error for MeasureError {}

/// Run the frozen tests against the base-restored tree and count.
///
/// `frozen` is the closure's paths. Only ids whose `path` is in it are counted:
/// PB §4.3's number is over "how many **frozen** tests failed", and a run
/// against a whole repository collects far more than the closure.
pub fn measure(
    repo: &Repo,
    restored_tree: &str,
    frozen: &BTreeSet<String>,
    langs: &[String],
    timeout_secs: u64,
) -> Result<Measured, MeasureError> {
    let work = repo.root().join(".spine/cache/approve");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| MeasureError::Checkout(e.to_string()))?;
    let tree_dir = work.join("tree");
    checkout_tree(repo.root(), restored_tree, &tree_dir)
        .map_err(|e| MeasureError::Checkout(e.to_string()))?;

    let mut adapters = adapters_for(langs);
    if adapters.is_empty() {
        return Err(MeasureError::NoAdapter);
    }

    // One checkout serves as both: this run has no `B` half. `LocalHost` is
    // RF §7.1's `profile=none`, which is honest — G12's number is not sealed
    // as evidence about a boundary, and PB §6.3 gives it no `profile=`.
    let mut host = LocalHost::new(tree_dir.clone(), tree_dir, work.join("scratch"))
        .map_err(|e| MeasureError::Checkout(e.to_string()))?;
    host.checkout(Checkout::Candidate);
    host.restore(Checkout::Candidate, timeout_secs);

    let mut measured = Measured::default();
    let mut collected_paths: BTreeSet<String> = BTreeSet::new();
    // `(runner, fn)` → did any parametrization fail.
    let mut functions: std::collections::BTreeMap<String, bool> = std::collections::BTreeMap::new();

    for adapter in adapters.iter_mut() {
        let run = adapter.run_candidate(&mut host, timeout_secs);
        let runner = adapter.token().as_str().to_string();
        for item in &run.items {
            if !frozen.contains(&item.path) {
                continue;
            }
            collected_paths.insert(item.path.clone());
            // "the pair is the identity, and G1 requires **every** collected
            // parametrization of each to pass" — so one failing
            // parametrization makes the function red.
            let key = format!("{runner} {}", item.function);
            let failed = !item.out.is_pass();
            functions
                .entry(key)
                .and_modify(|red| *red |= failed)
                .or_insert(failed);
        }
    }
    host.reap_all();

    measured.total = functions.len();
    measured.red = functions.values().filter(|red| **red).count();
    measured.tests = functions.into_keys().collect();
    measured.uncollected = frozen
        .iter()
        .filter(|p| !collected_paths.contains(*p))
        .cloned()
        .collect();

    let _ = std::fs::remove_dir_all(&work);
    Ok(measured)
}

/// Which adapters this release ships for the declared languages.
fn adapters_for(langs: &[String]) -> Vec<Box<dyn RunnerAdapter>> {
    let mut out: Vec<Box<dyn RunnerAdapter>> = Vec::new();
    for lang in langs {
        if lang == "python" {
            out.push(Box::new(Pytest::new()));
        }
    }
    out
}

/// A **tree**, not a commit: `merge-tree`'s and `write-tree`'s output is a tree
/// no commit holds, so `git worktree` has nothing to check out. `read-tree`
/// into a private index and `checkout-index` out of it is the plumbing that
/// takes one.
fn checkout_tree(root: &Path, tree: &str, into: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(into)?;
    let index = into.join(".spine-index");
    let run = |args: &[&str]| -> std::io::Result<()> {
        let out = std::process::Command::new("git")
            .current_dir(root)
            .env("GIT_INDEX_FILE", &index)
            .args(args)
            .output()?;
        if out.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(
                String::from_utf8_lossy(&out.stderr).to_string(),
            ))
        }
    };
    run(&["read-tree", tree])?;
    let prefix = format!("{}/", into.display());
    run(&["checkout-index", "-a", "-f", "--prefix", &prefix])?;
    let _ = std::fs::remove_file(&index);
    Ok(())
}

#[cfg(test)]
mod tests {

    /// PB §4.3: "the pair is the identity, and G1 requires **every** collected
    /// parametrization of each to pass" — so one failing parametrization makes
    /// the function red, and two parametrizations are one `Spine-Test` line.
    ///
    /// The folding is the part worth pinning; the run around it needs a
    /// repository and is exercised end to end by `--approve`'s own test.
    #[test]
    fn parametrizations_fold_into_one_function_and_one_failure_reddens_it() {
        let mut functions: std::collections::BTreeMap<String, bool> = Default::default();
        for (key, failed) in [
            ("pytest t.py::test_a", false),
            ("pytest t.py::test_a", true),
            ("pytest t.py::test_b", false),
        ] {
            functions
                .entry(key.to_string())
                .and_modify(|red| *red |= failed)
                .or_insert(failed);
        }
        assert_eq!(functions.len(), 2, "two parametrizations are one line");
        assert!(functions["pytest t.py::test_a"], "one failure reddens it");
        assert_eq!(functions.values().filter(|r| **r).count(), 1);
    }
}
