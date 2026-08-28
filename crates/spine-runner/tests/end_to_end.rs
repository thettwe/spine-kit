//! The whole collector path against **real pytest**: prepare's outputs, a real
//! host, real processes, and the file RF §3 publishes.
//!
//! Skipped where pytest is absent, because the point is that it is real. The
//! venv is found through `SPINE_TEST_PYTEST`, which the runner sets; there is
//! no search of the host's `PATH`, since a different pytest would make the
//! vectors mean something else.

use std::path::{Path, PathBuf};

use spine_collect::collector::{Host, Mode, Run, RunnerAdapter, collect};
use spine_collect::header::Profile;
use spine_collect::outcome::Outcome;
use spine_collect::record::Status;
use spine_manifest::Isolation;
use spine_runner::LocalHost;
use spine_runner::pytest::Pytest;

fn pytest_binary() -> Option<PathBuf> {
    let path = std::env::var_os("SPINE_TEST_PYTEST").map(PathBuf::from)?;
    path.exists().then_some(path)
}

fn project(root: &Path, extra: &str) {
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(
        root.join("tests/test_suite.py"),
        format!(
            "import pytest\n\n\
             def test_alpha():\n    assert True\n\n\
             def test_beta():\n    assert True\n\n\
             {extra}"
        ),
    )
    .unwrap();
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("spine-runner-e2e-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// RF §7.1 steps 6-10, over two real checkouts and three real pytest
/// invocations, ending in the file RF §3 publishes.
#[test]
fn a_whole_collection_runs_pytest_and_publishes_a_file() {
    let Some(pytest) = pytest_binary() else {
        return;
    };
    let root = scratch("whole");
    let base = root.join("base");
    let candidate = root.join("candidate");
    let work = root.join("work");
    std::fs::create_dir_all(&work).unwrap();

    // `B` has two tests. `T` adds one and breaks nothing, which is the
    // ordinary green landing.
    project(&base, "");
    project(&candidate, "def test_gamma():\n    assert True\n");

    // The adapter runs whatever argv `spine-resolve` ratified, and the host
    // runs it at the checkout root — so `pytest` has to be reachable by name.
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::os::unix::fs::symlink(&pytest, bin.join("pytest")).unwrap();
    let inherited = std::env::var("PATH").unwrap_or_default();
    unsafe { std::env::set_var("PATH", format!("{}:{inherited}", bin.display())) };

    let mut host = LocalHost::new(base.clone(), candidate.clone(), work).expect("a host");
    assert_eq!(host.profile(), Profile::None, "no boundary is attempted");

    let run = Run {
        tree: "3e91c7a2d0f46b58e19d73c0a5b284fd61e0c9a7".into(),
        base: "5f8e2a10cd47b6390e5a2c8db14f70369ba2e0d7".into(),
        tool: "1.4.0+sha256:0000000000000000000000000000000000000000000000000000000000000000"
            .into(),
        keys_visible: true,
    };
    let mut adapters: Vec<Box<dyn RunnerAdapter>> = vec![Box::new(Pytest::new())];
    let policy = spine_collect::collector::Policy {
        cli_version: "1.4.0".into(),
        dist_hash: "sha256:00".into(),
        isolation: Isolation::None,
        langs: vec!["python".into()],
        timeout_secs: 120,
        object_format: spine_canon::ObjectFormat::Sha1,
    };

    let file = collect(&run, &policy, Mode::Solo, &mut host, &mut adapters);

    // RF §7.3: a green run that reached its terminal event is `complete`, and
    // the exit code was never consulted.
    assert_eq!(file.status, Status::Complete, "{:?}", file.status);

    // `B`'s floor is two ids, and both were collected on the base checkout.
    let base_ids: Vec<&str> = file.base.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(base_ids.len(), 2, "{base_ids:?}");
    assert!(
        base_ids
            .iter()
            .all(|id| id.starts_with("tests/test_suite.py::"))
    );

    // `T` ran three, including the one the candidate added.
    let result_ids: Vec<&str> = file.results.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(result_ids.len(), 3, "{result_ids:?}");
    assert!(
        file.results.iter().all(|r| r.out == Outcome::Passed),
        "a green suite"
    );

    // RF §3's publish, into the directory the probe may have removed.
    let published = file
        .publish(&root, &run.tree)
        .expect("the file is written by temp-and-rename");
    assert_eq!(
        published,
        root.join(".spine/cache/results/")
            .join(format!("{}.jsonl", run.tree))
    );
    assert_eq!(std::fs::read(&published).unwrap(), file.to_bytes());

    let _ = std::fs::remove_dir_all(&root);
}

/// RF §7.4 rule 3's property, over real processes: "**before any process has
/// run against `T`'s content**". A candidate that deletes a test from its own
/// checkout cannot shrink `B`'s floor, because the floor was enumerated on `B`
/// before `T` existed as a checkout.
#[test]
fn a_candidate_that_deletes_a_landed_test_cannot_shrink_the_floor() {
    let Some(pytest) = pytest_binary() else {
        return;
    };
    let root = scratch("floor");
    let base = root.join("base");
    let candidate = root.join("candidate");
    let work = root.join("work");
    std::fs::create_dir_all(&work).unwrap();

    project(&base, "def test_landed():\n    assert True\n");
    // The candidate simply does not have it.
    project(&candidate, "");

    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::os::unix::fs::symlink(&pytest, bin.join("pytest")).unwrap();
    let inherited = std::env::var("PATH").unwrap_or_default();
    unsafe { std::env::set_var("PATH", format!("{}:{inherited}", bin.display())) };

    let mut host = LocalHost::new(base, candidate, work).expect("a host");
    let run = Run {
        tree: "aaaa".into(),
        base: "bbbb".into(),
        tool: "1.4.0+sha256:00".into(),
        keys_visible: true,
    };
    let mut adapters: Vec<Box<dyn RunnerAdapter>> = vec![Box::new(Pytest::new())];
    let policy = spine_collect::collector::Policy {
        cli_version: "1.4.0".into(),
        dist_hash: "sha256:00".into(),
        isolation: Isolation::None,
        langs: vec!["python".into()],
        timeout_secs: 120,
        object_format: spine_canon::ObjectFormat::Sha1,
    };

    let file = collect(&run, &policy, Mode::Solo, &mut host, &mut adapters);

    // The landed id is in the floor, and its `T` outcome is `absent` — which
    // is not a pass, and is what G1 fails on.
    let landed = file
        .base
        .iter()
        .find(|r| r.id.ends_with("test_landed"))
        .expect("the floor remembers what B collected");
    assert!(
        !file.results.iter().any(|r| r.id == landed.id),
        "the candidate never ran it, so there is no result record"
    );

    let _ = std::fs::remove_dir_all(&root);
}
