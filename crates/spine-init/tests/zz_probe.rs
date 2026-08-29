//! TEMPORARY adversarial probe — delete after running.
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use spine_init::Repo;

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").current_dir(dir).args(args).output().ok()?;
    if !out.status.success() {
        eprintln!("git {:?} FAILED: {}", args, String::from_utf8_lossy(&out.stderr));
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("spine-probe-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main", "."]).unwrap();
    git(&dir, &["config", "user.email", "t@e.invalid"]).unwrap();
    git(&dir, &["config", "user.name", "T"]).unwrap();
    dir
}

fn commit(dir: &Path, msg: &str) -> String {
    git(dir, &["add", "-A"]).unwrap();
    git(dir, &["commit", "-q", "-m", msg]).unwrap();
    git(dir, &["rev-parse", "HEAD"]).unwrap()
}

fn mktree(dir: &Path, records: &[Vec<u8>]) -> String {
    let mut child = Command::new("git")
        .current_dir(dir)
        .args(["mktree", "-z"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        for r in records {
            stdin.write_all(r).unwrap();
            stdin.write_all(b"\0").unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "mktree: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim_end().to_string()
}

fn blob(dir: &Path, content: &str) -> String {
    let mut child = Command::new("git")
        .current_dir(dir)
        .args(["hash-object", "-w", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(content.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim_end().to_string()
}

fn commit_tree(dir: &Path, tree: &str, msg: &str) -> String {
    git(dir, &["commit-tree", tree, "-m", msg]).unwrap()
}

#[test]
fn probe_exec_bit() {
    use std::os::unix::fs::PermissionsExt;
    let d = scratch("execbit");
    std::fs::create_dir_all(d.join("src")).unwrap();
    std::fs::write(d.join("src/run.sh"), "echo hi\n").unwrap();
    std::fs::set_permissions(d.join("src/run.sh"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let base = commit(&d, "base");
    std::fs::set_permissions(d.join("src/run.sh"), std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::write(d.join("src/run.sh"), "echo changed\n").unwrap();
    let approval = commit(&d, "exec bit dropped");
    println!("base:     {}", git(&d, &["ls-tree", "-r", &base]).unwrap());
    println!("approval: {}", git(&d, &["ls-tree", "-r", &approval]).unwrap());
    let repo = Repo::discover(&d).unwrap();
    let tree = repo.restored_base_tree(&approval, &base, &|p| p.starts_with("src/")).unwrap();
    println!("restored: {}", git(&d, &["ls-tree", "-r", &tree]).unwrap());
}

#[test]
fn probe_submodule_gitlink() {
    let d = scratch("gitlink");
    let keep = blob(&d, "k\n");
    let fake_commit = "0123456789abcdef0123456789abcdef01234567";
    let sub = mktree(&d, &[
        format!("100644 blob {keep}\tkeep.py").into_bytes(),
        format!("160000 commit {fake_commit}\tvendor").into_bytes(),
    ]);
    let base_tree = mktree(&d, &[format!("040000 tree {sub}\tsrc").into_bytes()]);
    let base = commit_tree(&d, &base_tree, "base");
    let other = blob(&d, "o\n");
    let sub2 = mktree(&d, &[
        format!("100644 blob {keep}\tkeep.py").into_bytes(),
        format!("100644 blob {other}\tother.py").into_bytes(),
    ]);
    let appr_tree = mktree(&d, &[format!("040000 tree {sub2}\tsrc").into_bytes()]);
    let approval = commit_tree(&d, &appr_tree, "approval");
    println!("base:\n{}", git(&d, &["ls-tree", "-r", &base]).unwrap());
    println!("approval:\n{}", git(&d, &["ls-tree", "-r", &approval]).unwrap());
    let repo = Repo::discover(&d).unwrap();
    let got = repo.restored_base_tree(&approval, &base, &|p| p.starts_with("src/"));
    println!("GITLINK result: {got:?}");
    if let Ok(t) = &got {
        println!("restored:\n{}", git(&d, &["ls-tree", "-r", t]).unwrap_or_default());
        println!("base tree = {base_tree}");
    }
}

#[test]
fn probe_tab_newline_dash_paths() {
    let d = scratch("weird");
    let b1 = blob(&d, "base bytes\n");
    let b2 = blob(&d, "branch bytes\n");
    let names: Vec<&[u8]> = vec![b"a\tb.py", b"c\nd.py", b"-dash.py", b"plain.py"];
    let recs = |oid: &str| -> Vec<Vec<u8>> {
        names.iter().map(|n| {
            let mut r = format!("100644 blob {oid}\t").into_bytes();
            r.extend_from_slice(n);
            r
        }).collect()
    };
    let base_sub = mktree(&d, &recs(&b1));
    let appr_sub = mktree(&d, &recs(&b2));
    let base = commit_tree(&d, &mktree(&d, &[format!("040000 tree {base_sub}\tsrc").into_bytes()]), "base");
    let approval = commit_tree(&d, &mktree(&d, &[format!("040000 tree {appr_sub}\tsrc").into_bytes()]), "approval");
    let repo = Repo::discover(&d).unwrap();
    println!("ls_tree_all(base) = {:?}", repo.ls_tree_all(&base));
    let got = repo.restored_base_tree(&approval, &base, &|p| p.starts_with("src/"));
    println!("WEIRD result: {got:?}");
    if let Ok(t) = &got {
        let out = Command::new("git").current_dir(&d).args(["ls-tree", "-r", "-z", t]).output().unwrap();
        println!("restored: {:?}", String::from_utf8_lossy(&out.stdout));
        println!("base blob {b1} / branch blob {b2}");
    }
}

#[test]
fn probe_non_utf8_path() {
    let d = scratch("nonutf8");
    let b1 = blob(&d, "base bytes\n");
    let b2 = blob(&d, "branch bytes\n");
    let mut base_rec = format!("100644 blob {b1}\t").into_bytes();
    base_rec.extend_from_slice(b"caf\xe9.py");
    let mut appr_rec = format!("100644 blob {b2}\t").into_bytes();
    appr_rec.extend_from_slice(b"caf\xe9.py");
    let base_sub = mktree(&d, &[base_rec]);
    let appr_sub = mktree(&d, &[appr_rec]);
    let base = commit_tree(&d, &mktree(&d, &[format!("040000 tree {base_sub}\tsrc").into_bytes()]), "base");
    let approval = commit_tree(&d, &mktree(&d, &[format!("040000 tree {appr_sub}\tsrc").into_bytes()]), "approval");
    let repo = Repo::discover(&d).unwrap();
    println!("ls_tree_all(base) = {:?}", repo.ls_tree_all(&base));
    let got = repo.restored_base_tree(&approval, &base, &|p| p.starts_with("src/"));
    println!("NON-UTF8 result: {got:?}");
    if let Ok(t) = &got {
        let out = Command::new("git").current_dir(&d).args(["ls-tree", "-r", "-z", t]).output().unwrap();
        println!("restored raw: {:?}", out.stdout);
        println!("base blob {b1} / branch blob {b2}");
    }
}

#[test]
fn probe_case_rename() {
    let d = scratch("caserename");
    println!("core.ignorecase = {:?}", git(&d, &["config", "--get", "core.ignorecase"]));
    let b1 = blob(&d, "base\n");
    let b2 = blob(&d, "branch\n");
    let base_sub = mktree(&d, &[format!("100644 blob {b1}\tBilling.py").into_bytes()]);
    let appr_sub = mktree(&d, &[format!("100644 blob {b2}\tbilling.py").into_bytes()]);
    let base = commit_tree(&d, &mktree(&d, &[format!("040000 tree {base_sub}\tsrc").into_bytes()]), "base");
    let approval = commit_tree(&d, &mktree(&d, &[format!("040000 tree {appr_sub}\tsrc").into_bytes()]), "approval");
    let repo = Repo::discover(&d).unwrap();
    let got = repo.restored_base_tree(&approval, &base, &|p| p.starts_with("src/"));
    println!("CASE result: {got:?}");
    if let Ok(t) = &got {
        println!("restored:\n{}", git(&d, &["ls-tree", "-r", t]).unwrap_or_default());
        println!("want: src/Billing.py at {b1}, no src/billing.py");
    }
}

#[test]
fn probe_concurrent_index_collision() {
    let d = scratch("concurrent");
    std::fs::create_dir_all(d.join("src")).unwrap();
    for i in 0..40 {
        std::fs::write(d.join(format!("src/f{i}.py")), format!("A={i}\n")).unwrap();
    }
    let base = commit(&d, "base");
    for i in 0..40 {
        std::fs::write(d.join(format!("src/f{i}.py")), format!("B={i}\n")).unwrap();
    }
    let approval = commit(&d, "candidate");
    let base_tree = git(&d, &["rev-parse", &format!("{base}^{{tree}}")]).unwrap();
    let repo = std::sync::Arc::new(Repo::discover(&d).unwrap());
    let mut handles = Vec::new();
    for _ in 0..8 {
        let r = std::sync::Arc::clone(&repo);
        let a = approval.clone();
        let b = base.clone();
        handles.push(std::thread::spawn(move || r.restored_base_tree(&a, &b, &|p| p.starts_with("src/"))));
    }
    for h in handles {
        let got = h.join().unwrap();
        let ok = matches!(&got, Ok(t) if *t == base_tree);
        println!("concurrent: correct={ok} {got:?}");
    }
    println!("want every one = {base_tree}");
}
