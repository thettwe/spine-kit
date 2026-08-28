//! PB §6.3's G12 tree: the approval tree with the intent's `expected` paths
//! restored to base.
//!
//! > "`red=k/n` recorded at `--approve`, measured with the intent's `expected`
//! > paths restored to base"
//!
//! This is what makes the number mean anything: the frozen tests run against
//! trunk's code, so `red` counts tests that fail *because the feature is
//! absent*, not tests that pass because the implementation is already there.

use std::path::{Path, PathBuf};
use std::process::Command;

use spine_init::Repo;

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn available() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Option<Self> {
        if !available() {
            return None;
        }
        let dir = std::env::temp_dir().join(format!("spine-g12-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok()?;
        git(&dir, &["init", "-q", "-b", "main", "."])?;
        git(&dir, &["config", "user.email", "t@example.invalid"])?;
        git(&dir, &["config", "user.name", "Test"])?;
        Some(Scratch(dir))
    }

    fn write(&self, path: &str, content: &str) {
        let full = self.0.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }

    fn remove(&self, path: &str) {
        let _ = std::fs::remove_file(self.0.join(path));
    }

    fn commit(&self, message: &str) -> String {
        git(&self.0, &["add", "-A"]).unwrap();
        git(&self.0, &["commit", "-q", "-m", message]).unwrap();
        git(&self.0, &["rev-parse", "HEAD"]).unwrap()
    }

    /// The paths and blobs of a written tree, for comparison.
    fn tree_paths(&self, tree: &str) -> Vec<String> {
        git(&self.0, &["ls-tree", "-r", "--name-only", tree])
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn blob_in(&self, tree: &str, path: &str) -> Option<String> {
        git(&self.0, &["rev-parse", &format!("{tree}:{path}")])
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The two directions, in one repository:
///
/// - a path the branch **changed** under `expected` goes back to base's bytes;
/// - a path the branch **added** under `expected` is **removed**, which is the
///   direction an implementation forgets — a new module the frozen tests import
///   is exactly the implementation G12 must not see.
///
/// And the third case, which is the control: everything outside `expected`,
/// including the tests themselves, is left exactly as the branch has it.
#[test]
fn expected_paths_go_back_to_base_and_added_ones_disappear() {
    let Some(scratch) = Scratch::new("both-directions") else {
        return;
    };

    scratch.write("src/billing.py", "TAX = 0\n");
    scratch.write("src/untouched.py", "KEEP = 1\n");
    scratch.write("tests/test_billing.py", "# old test\n");
    scratch.write("README.md", "base\n");
    let base = scratch.commit("base");

    // The branch: changes code under test, adds a module, rewrites the test,
    // and edits something outside `expected`.
    scratch.write("src/billing.py", "TAX = 1\n");
    scratch.write("src/brand_new.py", "def helper(): pass\n");
    scratch.write("tests/test_billing.py", "# the new, red test\n");
    scratch.write("README.md", "candidate\n");
    let approval = scratch.commit("the candidate");

    let repo = Repo::discover(&scratch.0).unwrap();
    let is_expected = |p: &str| p.starts_with("src/");
    let tree = repo
        .restored_base_tree(&approval, &base, &is_expected)
        .expect("a restored tree");

    // Changed under `expected`: base's bytes.
    assert_eq!(
        scratch.blob_in(&tree, "src/billing.py"),
        scratch.blob_in(&base, "src/billing.py"),
        "a changed expected path goes back to base"
    );
    // Added under `expected`: gone.
    assert!(
        !scratch
            .tree_paths(&tree)
            .contains(&"src/brand_new.py".to_string()),
        "a module the branch added under expected must be invisible to G12"
    );
    // Untouched under `expected`: still there, unchanged.
    assert_eq!(
        scratch.blob_in(&tree, "src/untouched.py"),
        scratch.blob_in(&base, "src/untouched.py")
    );
    // Outside `expected`: the branch's, including the tests G12 is about to
    // run — restoring those would be measuring base against base.
    assert_eq!(
        scratch.blob_in(&tree, "tests/test_billing.py"),
        scratch.blob_in(&approval, "tests/test_billing.py"),
        "the frozen tests are the branch's, or there is nothing to measure"
    );
    assert_eq!(
        scratch.blob_in(&tree, "README.md"),
        scratch.blob_in(&approval, "README.md")
    );
}

/// A path the branch **deleted** under `expected` comes back: the restoration
/// is to base's tree, not a merge of the two.
#[test]
fn a_deleted_expected_path_is_restored() {
    let Some(scratch) = Scratch::new("deleted") else {
        return;
    };
    scratch.write("src/keep.py", "A = 1\n");
    scratch.write("src/gone.py", "B = 2\n");
    let base = scratch.commit("base");

    scratch.remove("src/gone.py");
    let approval = scratch.commit("the branch deleted it");

    let repo = Repo::discover(&scratch.0).unwrap();
    let tree = repo
        .restored_base_tree(&approval, &base, &|p| p.starts_with("src/"))
        .unwrap();
    assert_eq!(
        scratch.blob_in(&tree, "src/gone.py"),
        scratch.blob_in(&base, "src/gone.py"),
        "restoration is to base's tree, not a merge"
    );
}

/// The tree is written and nothing else moves: no ref, no index, no checkout.
/// `--approve` computes a number from it and the repository must be as it was.
#[test]
fn writing_the_tree_moves_no_ref_and_leaves_no_index() {
    let Some(scratch) = Scratch::new("no-side-effects") else {
        return;
    };
    scratch.write("src/a.py", "A = 1\n");
    let base = scratch.commit("base");
    scratch.write("src/a.py", "A = 2\n");
    let approval = scratch.commit("candidate");

    let head_before = git(&scratch.0, &["rev-parse", "HEAD"]).unwrap();
    let status_before = git(&scratch.0, &["status", "--porcelain"]).unwrap();

    let repo = Repo::discover(&scratch.0).unwrap();
    let tree = repo
        .restored_base_tree(&approval, &base, &|p| p.starts_with("src/"))
        .unwrap();
    assert_eq!(tree.len(), head_before.len(), "an oid");

    assert_eq!(
        git(&scratch.0, &["rev-parse", "HEAD"]).unwrap(),
        head_before
    );
    assert_eq!(
        git(&scratch.0, &["status", "--porcelain"]).unwrap(),
        status_before
    );
    assert!(
        !scratch.0.join(".git/spine-g12-index").exists(),
        "the scratch index is removed"
    );
}

/// With nothing under `expected`, the restored tree **is** the approval tree —
/// which is the honest answer and not a special case.
#[test]
fn an_intent_that_expects_nothing_restores_nothing() {
    let Some(scratch) = Scratch::new("empty-expected") else {
        return;
    };
    scratch.write("src/a.py", "A = 1\n");
    let base = scratch.commit("base");
    scratch.write("src/a.py", "A = 2\n");
    let approval = scratch.commit("candidate");

    let repo = Repo::discover(&scratch.0).unwrap();
    let tree = repo
        .restored_base_tree(&approval, &base, &|_| false)
        .unwrap();
    assert_eq!(
        tree,
        git(&scratch.0, &["rev-parse", &format!("{approval}^{{tree}}")]).unwrap()
    );
}
