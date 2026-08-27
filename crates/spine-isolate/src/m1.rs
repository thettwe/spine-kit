//! **M1** — the one mechanism v1 ships (RF §7.1): the namespace set, the mount
//! sequence, the two network dispositions, and the restore phase.
//!
//! *"The collector spawns each runner as a child in a new **mount**, **PID**,
//! **IPC**, **network** and **user** namespace over an **overlay of the job's
//! own root filesystem**"* — and *"**No image is pulled and none is named.** The
//! boundary is made out of the filesystem the job already has … that is why
//! `params` needs no image key and `.spine/ci.sh` passes the collector no
//! isolation argument."*
//!
//! PB §7.4 rule 3's bullet calls `container` *"a container the collector
//! created"*, which reads like an OCI runtime. It is the profile's **name**;
//! PB's own next paragraph defers the mechanism here, and RF §12 makes the
//! runtime choice explicitly unspecified. Nothing in this module pulls, names or
//! talks to an image, a registry or a daemon.

use core::fmt;
use core::time::Duration;
use std::path::{Path, PathBuf};

/// D4 — M1's namespace set. **Exactly five**, all created for the child:
/// mount, PID, IPC, network, user (RF §7.1 *M1 — the shipped mechanism*, §13
/// R33/R34).
///
/// The numbers are Linux's `<linux/sched.h>` and not this implementation's: the
/// corpus names the namespaces and never their `CLONE_*` bits. They live here
/// rather than beside the syscall because *which five* is the normative fact and
/// `unshare(2)` is the replaceable one (RF §12).
pub mod clone {
    pub const NEWNS: u32 = 0x0002_0000;
    pub const NEWIPC: u32 = 0x0800_0000;
    pub const NEWUSER: u32 = 0x1000_0000;
    pub const NEWPID: u32 = 0x2000_0000;
    pub const NEWNET: u32 = 0x4000_0000;
}

/// RF §7.1 *M1's two network dispositions* — *"two dispositions of one
/// boundary — identical in mount, PID, IPC and user namespace, in root, in
/// writable tree, in masked result directory, in mapped identity and in pipes,
/// and **differing in exactly one thing, the network namespace**"*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// *"does not unshare the network namespace and keeps the job's own.
    /// `SPINE_ALLOWED_HOSTS`, `SPINE_REGISTRY_PROXY` and the client variables
    /// `ci.md` §5.6 sets are in its environment, for whatever the host puts in
    /// front of the socket to read"*.
    ///
    /// **Exactly one phase per checkout** runs in this one, and *"which is which
    /// is fixed by the collector rather than chosen per runner"*.
    Restore,
    /// *"a fresh network namespace holding only loopback. It is what **every
    /// runner invocation** is spawned under, without exception, and it is the
    /// configuration the probe below is built from and P4 is measured
    /// against."*
    Runner,
}

impl Disposition {
    /// The `unshare(2)` flag set. Five namespaces for the runner disposition,
    /// four for the restore one — the difference is `CLONE_NEWNET` and nothing
    /// else.
    pub fn namespaces(self) -> u32 {
        let common = clone::NEWNS | clone::NEWPID | clone::NEWIPC | clone::NEWUSER;
        match self {
            Disposition::Restore => common,
            Disposition::Runner => common | clone::NEWNET,
        }
    }
}

// ---------------------------------------------------------------------------
// M1's root
// ---------------------------------------------------------------------------

/// Where the result directory sits relative to the child's view. RF §7.1:
/// *"`.spine/cache/` is absent from that view: masked where the runtime can mask
/// a subpath, and otherwise outside the mounted root, with the file moved into
/// place after the process group is reaped (§3). Either arrangement satisfies
/// `container`; **a mounted, writable result directory does not, whatever the
/// configuration claims — which is what P1 measures.**"*
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultDirectoryArrangement {
    /// (a) The subpath is masked inside the child's mount namespace.
    Masked { at: PathBuf },
    /// (b) The directory lives outside the mounted root and the file is moved
    /// into place after the process group is reaped.
    ///
    /// Note what this costs under M1 specifically: the overlay's first lower
    /// layer is the job's **whole root**, so "outside the mounted root" cannot
    /// mean "elsewhere on the filesystem" — it means a directory the collector
    /// mounts in a mount namespace the child does not share. Arrangement (a) is
    /// the one M1 reaches for; (b) is kept because §3 admits it and P1 measures
    /// *"whichever path the collector is actually using"*.
    OutsideTheMountedRoot { staging: PathBuf, publish_to: PathBuf },
}

/// The inputs to M1's mount sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootSpec {
    /// Lower layer 1: *"the job's own root filesystem"*.
    pub job_root: PathBuf,
    /// Lower layer 2: *"one **empty** directory on a filesystem of its own — a
    /// `tmpfs` the collector mounts for the purpose, before the overlay"*.
    pub empty_lower: PathBuf,
    /// Where the overlay is mounted; `pivot_root`'s new root.
    pub new_root: PathBuf,
    /// *"The tree under test is the only writable path that is not scratch"* —
    /// the detached checkout of `T` on a `T` run, the collector's checkout of
    /// `B` on a `B` invocation. For the **probe** it is *"an empty directory of
    /// the collector's own making"*.
    pub writable_tree: PathBuf,
    /// Where the writable tree is bind-mounted inside `new_root`.
    pub writable_tree_at: PathBuf,
    /// *"one private temporary directory"*, a `tmpfs`.
    pub scratch_at: PathBuf,
    /// `pivot_root`'s `put_old`, beneath `new_root`.
    pub put_old: PathBuf,
    pub result_directory: ResultDirectoryArrangement,
}

/// One step of the sequence RF §7.1 fixes. The **order** is the normative part,
/// so it is data a test can read rather than statements a test cannot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MountStep {
    /// DERIVED. The corpus does not mention it, and `pivot_root(2)` refuses a
    /// new root whose mount propagation is shared — on a systemd host `/` is
    /// `shared` by default, so without this the last step of the sequence fails
    /// with `EINVAL` and the whole boundary is disposition 2 on most Linux
    /// distributions. It also stops every mount below from propagating back
    /// into the job's own namespace, which is what "the boundary" means.
    MakeRootPrivate,
    /// *"a `tmpfs` the collector mounts for the purpose, **before the
    /// overlay**"* (RF §7.1, as amended by §13 R36).
    TmpfsForEmptyLowerLayer { at: PathBuf },
    /// *"an **overlay** with **no upper layer** over two **lower** layers"*.
    OverlayRoot {
        lower: (PathBuf, PathBuf),
        at: PathBuf,
    },
    /// *"the writable tree is bind-mounted over the overlay **afterwards**"*.
    BindWritableTree { from: PathBuf, to: PathBuf },
    /// *"the private temporary directory is a `tmpfs` mounted the same way"*.
    TmpfsScratch { at: PathBuf },
    /// *"`.spine/cache/` is absent from that view"*, arrangement (a).
    MaskResultDirectory { at: PathBuf },
    /// A fresh `procfs`, mounted over the new root's `/proc` **before**
    /// `pivot_root`.
    ///
    /// DERIVED, and P3 cannot be measured without it. The corpus names the PID
    /// namespace and makes P3 read the probe's "own process table", but never
    /// says where that table comes from. It comes from `procfs` — and after
    /// `pivot_root` into an upper-less overlay whose lower layer is the job's
    /// root, the job's own `/proc` is **not** in the child's view: submounts of
    /// `/` are not part of an overlay over `/`, so the child sees the bare,
    /// empty mountpoint directory. P3's process-table limb would then enumerate
    /// nothing on every host, for every configuration, forever — the third
    /// instance of the shape RF §13 R36 exists to eliminate.
    ///
    /// A *fresh* mount and not a bind: `procfs` shows the PID namespace of the
    /// task that mounted it, so binding the job's `/proc` would show the job's
    /// pids and P3's limb would find the collector's own — inverting the test
    /// rather than emptying it.
    ProcFs { at: PathBuf },
    /// *"the child is `pivot_root`ed into the result"*.
    PivotRoot { new_root: PathBuf, put_old: PathBuf },
}

/// RF §7.1's sequence, in its order.
pub fn mount_sequence(spec: &RootSpec) -> Vec<MountStep> {
    let mut steps = vec![
        MountStep::MakeRootPrivate,
        MountStep::TmpfsForEmptyLowerLayer {
            at: spec.empty_lower.clone(),
        },
        MountStep::OverlayRoot {
            lower: (spec.job_root.clone(), spec.empty_lower.clone()),
            at: spec.new_root.clone(),
        },
        MountStep::BindWritableTree {
            from: spec.writable_tree.clone(),
            to: spec.writable_tree_at.clone(),
        },
        MountStep::TmpfsScratch {
            at: spec.scratch_at.clone(),
        },
    ];
    if let ResultDirectoryArrangement::Masked { at } = &spec.result_directory {
        steps.push(MountStep::MaskResultDirectory { at: at.clone() });
    }
    // Before `pivot_root`, and after the overlay it is mounted over: the
    // mountpoint has to exist, and `/proc` exists in the job's root, so it
    // exists in the overlay's lower layer. Mounting over a read-only overlay is
    // legal — a mount writes nothing to the filesystem beneath it.
    steps.push(MountStep::ProcFs {
        at: spec.new_root.join("proc"),
    });
    steps.push(MountStep::PivotRoot {
        new_root: spec.new_root.clone(),
        put_old: spec.put_old.clone(),
    });
    steps
}

/// Why an overlay option string could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayError {
    /// `lowerdir=` is a colon-separated list and the kernel's parser splits on
    /// `,` between options. A path holding either would silently become two
    /// layers, or half an option — a mis-mounted root that P3 would then happily
    /// pass, because it *is* a distinct filesystem. Refuse instead.
    PathHoldsASeparator(PathBuf),
    /// Every mount path must be nameable to `mount(2)`.
    PathNotUtf8(PathBuf),
}

impl fmt::Display for OverlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OverlayError::PathHoldsASeparator(p) => write!(
                f,
                "{} holds a ':' or ',' and cannot appear in an overlay lowerdir list",
                p.display()
            ),
            OverlayError::PathNotUtf8(p) => {
                write!(f, "{} is not UTF-8 and cannot be a mount option", p.display())
            }
        }
    }
}

impl core::error::Error for OverlayError {}

/// The `-o` string for M1's root.
///
/// RF §7.1's measured table, on Linux 6.11.11 as uid 0 with no user namespace
/// involved — so none of it is the unprivileged-overlayfs case prerequisite 3
/// already names:
///
/// ```text
/// | lower set                                  | result                          |
/// | a single directory                         | EINVAL                          |
/// | two directories on one filesystem          | mounts                          |
/// | / alone                                    | EINVAL                          |
/// | / and an ordinary directory beneath it     | ELOOP                           |
/// | / and an empty directory on a tmpfs        | mounts, read-only, own (dev,ino)|
/// ```
///
/// There is **no `upperdir=` and no `workdir=`**: *"An overlay with no upper
/// layer is read-only by construction, which is the second bullet's rule and not
/// an exception to it."*
pub fn overlay_options(lower_a: &Path, lower_b: &Path) -> Result<String, OverlayError> {
    let one = option_path(lower_a)?;
    let two = option_path(lower_b)?;
    Ok(format!("lowerdir={one}:{two}"))
}

fn option_path(path: &Path) -> Result<&str, OverlayError> {
    let text = path
        .to_str()
        .ok_or_else(|| OverlayError::PathNotUtf8(path.to_path_buf()))?;
    if text.contains(':') || text.contains(',') {
        return Err(OverlayError::PathHoldsASeparator(path.to_path_buf()));
    }
    Ok(text)
}

// ---------------------------------------------------------------------------
// The restore phase
// ---------------------------------------------------------------------------

/// RF §7.1, §11 item 16a, CI §5.6 — verbatim, and it is an address in trunk and
/// never a path in a checkout: *"A candidate therefore cannot introduce, edit or
/// delete the one phase that holds egress, and the script that runs against its
/// tree is trunk's."*
pub const RESTORE_SCRIPT_ADDRESS: &str = "origin/<trunk>:.spine/restore.sh";

/// *"It is `sh` over those bytes, at the root of that checkout."*
pub const RESTORE_INTERPRETER: &str = "sh";

/// *"**Two per run, never one per runner**, whatever the invocation set
/// holds."* This is also the `+2` in the worst-case wall time (RF §7.1 *The
/// deadline*).
pub const RESTORE_PHASES_PER_RUN: usize = 2;

/// Which checkout a phase belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checkout {
    /// Step 7. The collector's own checkout of `B`.
    Base,
    /// Step 8. The detached checkout of `T`. Its restore phase is *"the last
    /// phase of the run that holds the job's own network"*.
    Tree,
}

impl Checkout {
    fn name(self) -> &'static str {
        match self {
            Checkout::Base => "B",
            Checkout::Tree => "T",
        }
    }
}

/// The bytes, read from trunk. `Absent` is not an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreScript {
    /// *"Absent on trunk, the phase is empty, no process runs, and the collector
    /// writes one diagnostic to stderr saying so — which is what puts the
    /// remedy in the job log of the first landing that needs it. **It is not a
    /// prerequisite, not a failure and not a downgrade**: a repository whose
    /// toolchains are already provisioned wants exactly this."*
    Absent,
    Present(Vec<u8>),
}

impl RestoreScript {
    /// The one stderr line the empty case owes. DERIVED wording: the corpus
    /// fixes that there is *"one diagnostic to stderr saying so"* and never its
    /// bytes.
    pub fn diagnostic(&self, checkout: Checkout) -> Option<String> {
        match self {
            RestoreScript::Absent => Some(format!(
                "restore: no {RESTORE_SCRIPT_ADDRESS}; the restore phase for {} is empty \
                 and no process runs",
                checkout.name()
            )),
            RestoreScript::Present(_) => None,
        }
    }
}

/// What happened. **None of these reaches the file**: RF §7.1, *"It is not a
/// runner. It is in no invocation set (§6.2), contributes no `base` record, no
/// `result` record, no id and no `status` contribution (§7.3), and **nothing
/// reads its exit code**."*
///
/// That is why no variant carries an exit status. A `Ran { code }` would be an
/// exit code something could read, and *"a new token for it would put a value in
/// the file for a phase the file does not describe"*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreOutcome {
    /// No script on trunk; no process ran.
    Empty,
    /// A process ran and was reaped. Whether it succeeded is deliberately not
    /// representable: *"a restore that failed reappears as the suite that fails
    /// without it"*.
    Ran,
    /// *"on expiry the collector kills its process group and reaps it"*, and the
    /// run proceeds.
    TimedOut,
    /// The collector could not start `sh` at all. Still contributes nothing;
    /// still a diagnostic and a run that proceeds.
    SpawnFailed(String),
}

/// Run one restore phase.
///
/// - **bytes** from trunk, handed to `sh` **on stdin**. DERIVED: the corpus
///   fixes *"`sh` over those bytes"* and not how they reach it. Stdin is chosen
///   because the two alternatives both give something away — writing the script
///   into the checkout puts trunk's bytes at a path the candidate's tree
///   controls (and a `T` checkout is the candidate's tree), and `sh -c <string>`
///   publishes them in the process table and cannot carry a NUL. The cost is
///   named rather than hidden: a restore script cannot read its own stdin.
/// - **cwd** *"at the root of that checkout"*.
/// - **environment** *"the collector's own, unchanged"* — so nothing is removed
///   and nothing is added, which is what keeps §4.2's `keys_visible` predicate
///   already covering it by its first conjunct.
/// - **deadline** `params.timeout`; on expiry *"kill its process group and reap
///   it"*.
#[cfg(unix)]
pub fn run_restore(script: &RestoreScript, cwd: &Path, deadline: Duration) -> RestoreOutcome {
    use std::io::Write;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let RestoreScript::Present(bytes) = script else {
        return RestoreOutcome::Empty;
    };

    let mut command = Command::new(RESTORE_INTERPRETER);
    command
        .current_dir(cwd)
        .stdin(Stdio::piped())
        // The collector's own stderr and stdout: CI §5.1 puts every diagnostic
        // on stderr, and the restore phase's output is a diagnostic. It is not a
        // runner, so it has no stream to read (RF §7.1).
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    // Its own process group, so the deadline can kill *the group* — "kills the
    // whole process group of that invocation and reaps it". Without this a
    // restore that backgrounds a downloader leaves the downloader running.
    command.process_group(0);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => return RestoreOutcome::SpawnFailed(e.to_string()),
    };

    // The script is fed from a **separate thread**, and the deadline clock
    // starts before it.
    //
    // A blocking `write_all` on this thread bypassed the deadline entirely: a
    // `.spine/restore.sh` larger than the pipe buffer (64 KiB) whose head
    // blocks — `sleep 3600`, a `curl` at a dead socket — left `write_all`
    // waiting for a reader that never drains, so `started` was never taken and
    // `try_wait` was never called. The collector wedged forever, on a path
    // whose whole purpose is to be bounded: this is the one phase that keeps
    // the job's network, and `params.timeout` is what bounds it.
    //
    // The thread is detached deliberately. When the deadline expires the
    // process group is killed, the pipe breaks, `write_all` fails, and the
    // thread ends; joining it would reintroduce the wait this removes.
    if let Some(mut stdin) = child.stdin.take() {
        let script = bytes.clone();
        std::thread::spawn(move || {
            // A script that closes stdin early makes this a broken pipe, which
            // is the script's business and not a failure of the phase.
            let _ = stdin.write_all(&script);
            // Dropping `stdin` closes the pipe, which is how the script sees
            // EOF and can exit at all.
        });
    }

    let pgid = child.id();
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                // The phase is over, so its process group ends with it.
                //
                // RF §7.1 names the kill on expiry and is silent on normal
                // exit, and silence here is not the safe reading: this is the
                // ONE phase that keeps the job's network, and a script that
                // backgrounds a downloader and exits leaves that downloader
                // running — an egress window outliving the phase whose bound
                // is the only thing bounding it. The runner invocations that
                // follow are loopback-only, so the survivor would be the only
                // process in the job with a route off the host.
                //
                // Killing a group whose leader has already exited is harmless:
                // there is either nothing left to signal or exactly the
                // survivors this exists to end.
                let _ = crate::sys::kill_process_group(pgid, crate::sys::SIGKILL);
                return RestoreOutcome::Ran;
            }
            Ok(None) => {}
            Err(e) => return RestoreOutcome::SpawnFailed(e.to_string()),
        }
        if started.elapsed() >= deadline {
            let _ = crate::sys::kill_process_group(pgid, crate::sys::SIGKILL);
            // Reap it. The wait cannot block for long: the group has just been
            // SIGKILLed and SIGKILL is not catchable.
            let _ = child.wait();
            return RestoreOutcome::TimedOut;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// A busy-wait is the wrong tool at a 1800-second scale, and a `waitid`-based
/// wait with a timeout needs either a signal handler or a self-pipe — a second
/// mechanism inside the one component whose job is to be auditable. Ten
/// milliseconds costs microseconds of CPU per second of deadline.
#[cfg(unix)]
const POLL_INTERVAL: Duration = Duration::from_millis(10);

// ---------------------------------------------------------------------------
// The schedule: which phase runs under which disposition, and in what order
// ---------------------------------------------------------------------------

/// One runner in the invocation set (§6.2). Whether an adapter needs a separate
/// `B` outcome run is `import-resolver.md` §11.1's to say, not this crate's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerPlan {
    pub runner: String,
    pub separate_base_outcome_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationKind {
    /// Collecting the id set on `B` — *"an enumeration that stops early shrinks
    /// the floor"*.
    BaseEnumeration,
    /// *"the separate `B` outcome run where an adapter has one"* (IR §11.1).
    BaseOutcomeRun,
    TreeRun,
}

/// A phase of the run, and the disposition it is spawned under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// Step 6. *"a **probe boundary** from exactly the **runner disposition**
    /// M1 will use for a runner invocation, differing only in that its writable
    /// tree is an empty directory of the collector's own making"*.
    Probe,
    Restore { checkout: Checkout },
    Invocation {
        checkout: Checkout,
        runner: String,
        kind: InvocationKind,
    },
}

impl Phase {
    /// RF §7.1: every runner invocation is the runner disposition *"without
    /// exception"*, the probe is built from the runner disposition, and exactly
    /// one phase per checkout is the restore disposition.
    pub fn disposition(&self) -> Disposition {
        match self {
            Phase::Restore { .. } => Disposition::Restore,
            Phase::Probe | Phase::Invocation { .. } => Disposition::Runner,
        }
    }

    /// Which checkout this phase belongs to. `None` for the probe, which
    /// runs at step 6 — before either checkout exists.
    pub fn checkout(&self) -> Option<Checkout> {
        match self {
            Phase::Probe => None,
            Phase::Restore { checkout } => Some(*checkout),
            Phase::Invocation { checkout, .. } => Some(*checkout),
        }
    }
}

/// The order of §7.1's steps 6 to 8, expanded.
///
/// Two orderings here are normative and one is free:
///
/// - **Step 6 precedes step 7.** *"trunk's own tests are code that runs in the
///   job too, and a floor enumerated by an uncontained process is a floor the
///   job's other processes had a write path to."*
/// - **Every `B` collection precedes every `T` execution, without exception.**
///   *"interleaving — collect on `B` with pytest, run pytest on `T`, then
///   collect on `B` with vitest — would let code the candidate ran under the
///   first runner reach the second runner's collection of the floor, which is
///   exactly the attack rule 3 forbids."*
/// - Order *within* a checkout is free: *"Invocation order and concurrency are
///   an implementation choice and cannot affect the file's bytes (§4.5)."*
pub fn schedule(runners: &[RunnerPlan]) -> Vec<Phase> {
    let mut phases = vec![Phase::Probe, Phase::Restore { checkout: Checkout::Base }];
    for plan in runners {
        phases.push(Phase::Invocation {
            checkout: Checkout::Base,
            runner: plan.runner.clone(),
            kind: InvocationKind::BaseEnumeration,
        });
        if plan.separate_base_outcome_run {
            phases.push(Phase::Invocation {
                checkout: Checkout::Base,
                runner: plan.runner.clone(),
                kind: InvocationKind::BaseOutcomeRun,
            });
        }
    }
    phases.push(Phase::Restore { checkout: Checkout::Tree });
    for plan in runners {
        phases.push(Phase::Invocation {
            checkout: Checkout::Tree,
            runner: plan.runner.clone(),
            kind: InvocationKind::TreeRun,
        });
    }
    phases
}

/// How many invocations the deadline is paid for — *"the invocations are two or
/// three per runner depending on the adapter, not one per runner"*. The probe
/// and the two restore phases are not invocations.
pub fn invocation_count(runners: &[RunnerPlan]) -> u32 {
    runners
        .iter()
        .map(|p| if p.separate_base_outcome_run { 3 } else { 2 })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::clone;

    fn two_runners() -> Vec<RunnerPlan> {
        vec![
            RunnerPlan {
                runner: "pytest".into(),
                separate_base_outcome_run: true,
            },
            RunnerPlan {
                runner: "vitest".into(),
                separate_base_outcome_run: false,
            },
        ]
    }

    /// RF §7.1 / §13 R33/R34: *"a new **mount**, **PID**, **IPC**, **network**
    /// and **user** namespace"* — five, and the runner disposition holds all of
    /// them.
    #[test]
    fn m1_unshares_exactly_five_namespaces_for_a_runner() {
        let runner = Disposition::Runner.namespaces();
        assert_eq!(runner.count_ones(), 5);
        for bit in [
            clone::NEWNS,
            clone::NEWPID,
            clone::NEWIPC,
            clone::NEWNET,
            clone::NEWUSER,
        ] {
            assert_ne!(runner & bit, 0);
        }
    }

    /// *"differing in exactly one thing, the network namespace"*.
    #[test]
    fn the_two_dispositions_differ_in_exactly_the_network_namespace() {
        let runner = Disposition::Runner.namespaces();
        let restore = Disposition::Restore.namespaces();
        assert_eq!(runner ^ restore, clone::NEWNET);
        assert_eq!(restore & clone::NEWNET, 0, "restore keeps the job's own");
        assert_eq!(restore.count_ones(), 4);
    }

    /// RF §7.1, §11 item 16a: *"Every runner invocation is spawned under the
    /// **runner disposition** … without exception"*, and the probe is built
    /// from it too.
    #[test]
    fn every_runner_invocation_and_the_probe_are_the_runner_disposition() {
        for phase in schedule(&two_runners()) {
            match phase {
                Phase::Restore { .. } => {
                    assert_eq!(phase.disposition(), Disposition::Restore)
                }
                _ => assert_eq!(
                    phase.disposition(),
                    Disposition::Runner,
                    "{phase:?} must have no egress"
                ),
            }
        }
    }

    /// *"**Two per run, never one per runner**, whatever the invocation set
    /// holds."* Three runners, four runners — still two.
    #[test]
    fn exactly_one_restore_phase_per_checkout_never_one_per_runner() {
        for extra in 0..4 {
            let mut runners = two_runners();
            for i in 0..extra {
                runners.push(RunnerPlan {
                    runner: format!("runner{i}"),
                    separate_base_outcome_run: i % 2 == 0,
                });
            }
            let phases = schedule(&runners);
            let restores: Vec<Checkout> = phases
                .iter()
                .filter_map(|p| match p {
                    Phase::Restore { checkout } => Some(*checkout),
                    _ => None,
                })
                .collect();
            assert_eq!(restores, vec![Checkout::Base, Checkout::Tree]);
            assert_eq!(restores.len(), RESTORE_PHASES_PER_RUN);
        }
    }

    /// RF §7.1 step 7: *"Every `B` collection precedes every `T` execution,
    /// without exception."* Interleaving is the attack rule 3 forbids.
    #[test]
    fn every_base_invocation_precedes_every_tree_execution() {
        let phases = schedule(&two_runners());
        let first_tree = phases
            .iter()
            .position(|p| p.checkout() == Some(Checkout::Tree))
            .expect("a T phase");
        assert!(
            phases[first_tree..]
                .iter()
                .all(|p| p.checkout() != Some(Checkout::Base)),
            "no B phase may follow the first T phase"
        );
    }

    /// RF §7.1: step 6 precedes step 7 — *"a floor enumerated by an uncontained
    /// process is a floor the job's other processes had a write path to"*.
    #[test]
    fn the_probe_precedes_the_base_checkout() {
        let phases = schedule(&two_runners());
        assert_eq!(phases[0], Phase::Probe);
    }

    /// *"After each checkout and **before the first runner invocation against
    /// it**."*
    #[test]
    fn each_restore_phase_precedes_the_first_invocation_against_its_checkout() {
        let phases = schedule(&two_runners());
        for checkout in [Checkout::Base, Checkout::Tree] {
            let restore = phases
                .iter()
                .position(|p| matches!(p, Phase::Restore { checkout: c } if *c == checkout))
                .expect("a restore phase");
            let first_invocation = phases
                .iter()
                .position(|p| matches!(p, Phase::Invocation { checkout: c, .. } if *c == checkout))
                .expect("an invocation");
            assert!(restore < first_invocation, "{checkout:?}");
        }
    }

    /// *"the invocations are two or three per runner depending on the adapter,
    /// not one per runner"* (IR §11.1).
    #[test]
    fn an_adapter_with_a_separate_base_outcome_run_costs_three_invocations() {
        assert_eq!(invocation_count(&two_runners()), 5);
        let phases = schedule(&two_runners());
        let invocations = phases
            .iter()
            .filter(|p| matches!(p, Phase::Invocation { .. }))
            .count();
        assert_eq!(invocations, 5);
    }

    /// RF §13 R36(i). The lower set is `/` **and one empty directory on a
    /// `tmpfs`**, mounted before the overlay — because the kernel refuses an
    /// upper-less overlay with one lower layer (`EINVAL`) and refuses a second
    /// lower layer reachable through the first (`ELOOP`).
    #[test]
    fn the_empty_lower_layer_is_a_tmpfs_mounted_before_the_overlay() {
        let spec = spec();
        let steps = mount_sequence(&spec);
        let tmpfs = steps
            .iter()
            .position(|s| matches!(s, MountStep::TmpfsForEmptyLowerLayer { .. }))
            .expect("the second lower layer's tmpfs");
        let overlay = steps
            .iter()
            .position(|s| matches!(s, MountStep::OverlayRoot { .. }))
            .expect("the overlay");
        assert!(tmpfs < overlay);

        // And the second lower layer is exactly the path that tmpfs was mounted
        // at — which is the whole of the ELOOP guard: "an empty directory made
        // under `/` does not get a weaker boundary, it gets ELOOP".
        let (MountStep::TmpfsForEmptyLowerLayer { at }, MountStep::OverlayRoot { lower, .. }) =
            (&steps[tmpfs], &steps[overlay])
        else {
            unreachable!()
        };
        assert_eq!(&lower.1, at);
        assert_eq!(lower.0, spec.job_root);
    }

    /// *"An overlay with **no upper layer** is read-only by construction."*
    #[test]
    fn the_overlay_options_carry_two_lower_layers_and_no_upper() {
        let options = overlay_options(Path::new("/"), Path::new("/run/spine/empty")).unwrap();
        assert_eq!(options, "lowerdir=/:/run/spine/empty");
        assert!(!options.contains("upperdir"));
        assert!(!options.contains("workdir"));
        assert_eq!(options.matches(':').count(), 1, "exactly two lower layers");
    }

    /// A path holding a `:` would silently become two lower layers, and a `,`
    /// would end the option. The mis-mounted root would still pass P3 — it is
    /// still a distinct filesystem — so nothing downstream would catch it.
    #[test]
    fn a_path_holding_an_option_separator_is_refused_rather_than_mis_mounted() {
        for bad in ["/run/spine:empty", "/run/spine,empty"] {
            assert_eq!(
                overlay_options(Path::new("/"), Path::new(bad)),
                Err(OverlayError::PathHoldsASeparator(PathBuf::from(bad)))
            );
        }
    }

    /// RF §7.1's sequence, verbatim: *"the writable tree is bind-mounted over
    /// the overlay **afterwards**, the private temporary directory is a `tmpfs`
    /// mounted the same way, and the child is **`pivot_root`ed into the
    /// result**."*
    #[test]
    fn the_mount_sequence_is_in_the_order_the_spec_fixes() {
        let steps = mount_sequence(&spec());
        let kinds: Vec<&str> = steps.iter().map(step_name).collect();
        assert_eq!(
            kinds,
            [
                "MakeRootPrivate",
                "TmpfsForEmptyLowerLayer",
                "OverlayRoot",
                "BindWritableTree",
                "TmpfsScratch",
                "MaskResultDirectory",
                // Before PivotRoot, and after the overlay it mounts over.
                // Without it P3's process-table limb enumerates nothing.
                "ProcFs",
                "PivotRoot",
            ]
        );
    }

    /// §3's other arrangement. Both satisfy `container`; what does not is a
    /// mounted, writable result directory, "which is what P1 measures".
    #[test]
    fn the_second_result_directory_arrangement_masks_nothing_and_still_pivots() {
        let mut spec = spec();
        spec.result_directory = ResultDirectoryArrangement::OutsideTheMountedRoot {
            staging: PathBuf::from("/run/spine/staging"),
            publish_to: PathBuf::from("/repo/.spine/cache/results"),
        };
        let steps = mount_sequence(&spec);
        assert!(
            !steps
                .iter()
                .any(|s| matches!(s, MountStep::MaskResultDirectory { .. }))
        );
        assert!(matches!(steps.last(), Some(MountStep::PivotRoot { .. })));
    }

    /// RF §7.1, §11 item 16a: the address is in **trunk**, and *"never from a
    /// checkout, `T`'s included"*.
    #[test]
    fn the_restore_script_is_addressed_in_trunk_and_never_in_a_checkout() {
        assert_eq!(RESTORE_SCRIPT_ADDRESS, "origin/<trunk>:.spine/restore.sh");
        assert!(!RESTORE_SCRIPT_ADDRESS.starts_with('.'));
        assert!(RESTORE_SCRIPT_ADDRESS.starts_with("origin/"));
    }

    /// *"Absent on trunk, the phase is empty, no process runs, and the collector
    /// writes **one diagnostic** to stderr saying so."* Not a prerequisite
    /// failure, not a failure, not a downgrade.
    #[test]
    fn a_missing_restore_script_is_an_empty_phase_and_one_diagnostic() {
        let absent = RestoreScript::Absent;
        for checkout in [Checkout::Base, Checkout::Tree] {
            let line = absent.diagnostic(checkout).expect("one diagnostic");
            assert!(line.contains(RESTORE_SCRIPT_ADDRESS), "{line}");
            assert!(!line.contains("prerequisite"), "{line}");
        }
        assert_eq!(RestoreScript::Present(b"true\n".to_vec()).diagnostic(Checkout::Base), None);

        let outcome = run_restore(&absent, Path::new("."), Duration::from_secs(1));
        assert_eq!(outcome, RestoreOutcome::Empty);
    }

    /// *"**nothing reads its exit code**. A non-zero exit is a diagnostic on
    /// stderr and the run proceeds."* So a script exiting 7 and a script exiting
    /// 0 are the same outcome, and there is no accessor that could tell them
    /// apart.
    #[test]
    fn nothing_reads_the_restore_phases_exit_code() {
        let cwd = std::env::temp_dir();
        let zero = run_restore(
            &RestoreScript::Present(b"exit 0\n".to_vec()),
            &cwd,
            Duration::from_secs(30),
        );
        let seven = run_restore(
            &RestoreScript::Present(b"exit 7\n".to_vec()),
            &cwd,
            Duration::from_secs(30),
        );
        assert_eq!(zero, RestoreOutcome::Ran);
        assert_eq!(seven, RestoreOutcome::Ran);
    }

    /// *"It is bounded by `params.timeout` like an invocation; on expiry the
    /// collector kills its process group and reaps it"* — and the run proceeds,
    /// contributing no `status`.
    #[test]
    fn a_restore_phase_that_overruns_is_killed_reaped_and_contributes_nothing() {
        let cwd = std::env::temp_dir();
        let outcome = run_restore(
            &RestoreScript::Present(b"sleep 30\n".to_vec()),
            &cwd,
            Duration::from_millis(200),
        );
        assert_eq!(outcome, RestoreOutcome::TimedOut);
    }

    /// *"at the root of that checkout"*.
    #[test]
    fn the_restore_phase_runs_at_the_root_of_its_checkout() {
        let dir = std::env::temp_dir().join(format!("spine-restore-cwd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let outcome = run_restore(
            &RestoreScript::Present(b"pwd > cwd.txt\n".to_vec()),
            &dir,
            Duration::from_secs(30),
        );
        assert_eq!(outcome, RestoreOutcome::Ran);
        let seen = std::fs::read_to_string(dir.join("cwd.txt")).unwrap();
        // macOS resolves /var to /private/var, so compare the tails.
        assert!(
            seen.trim().ends_with(dir.file_name().unwrap().to_str().unwrap()),
            "{seen}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RF §7.1 *The phase is not conditioned on the profile*: *"Under
    /// `params.isolation: "none"`, and on the solo path where no boundary is
    /// attempted at all (§7.4), the restore phase still runs — same bytes, same
    /// source, same place in the order, same deadline — simply with no boundary
    /// around it."*
    ///
    /// [`schedule`] takes no profile and [`run_restore`] takes no boundary, so
    /// the rule is structural. What this test adds is the demonstration: the two
    /// restore phases are in the schedule unconditionally, and the phase above
    /// ran to completion on **this** host — which has no namespaces at all, and
    /// is therefore the disposition-2 and solo case (RF §7.4).
    #[test]
    fn the_restore_phase_is_not_conditioned_on_the_profile() {
        let phases = schedule(&two_runners());
        assert_eq!(
            phases
                .iter()
                .filter(|p| matches!(p, Phase::Restore { .. }))
                .count(),
            RESTORE_PHASES_PER_RUN
        );
        let outcome = run_restore(
            &RestoreScript::Present(b"true\n".to_vec()),
            &std::env::temp_dir(),
            Duration::from_secs(30),
        );
        assert_eq!(outcome, RestoreOutcome::Ran, "no boundary is still a phase");
    }

    fn step_name(step: &MountStep) -> &'static str {
        match step {
            MountStep::MakeRootPrivate => "MakeRootPrivate",
            MountStep::TmpfsForEmptyLowerLayer { .. } => "TmpfsForEmptyLowerLayer",
            MountStep::OverlayRoot { .. } => "OverlayRoot",
            MountStep::BindWritableTree { .. } => "BindWritableTree",
            MountStep::TmpfsScratch { .. } => "TmpfsScratch",
            MountStep::MaskResultDirectory { .. } => "MaskResultDirectory",
            MountStep::ProcFs { .. } => "ProcFs",
            MountStep::PivotRoot { .. } => "PivotRoot",
        }
    }

    fn spec() -> RootSpec {
        RootSpec {
            job_root: PathBuf::from("/"),
            empty_lower: PathBuf::from("/run/spine/empty"),
            new_root: PathBuf::from("/run/spine/root"),
            writable_tree: PathBuf::from("/work/probe-tree"),
            writable_tree_at: PathBuf::from("/run/spine/root/work/probe-tree"),
            scratch_at: PathBuf::from("/run/spine/root/tmp"),
            put_old: PathBuf::from("/run/spine/root/.old"),
            result_directory: ResultDirectoryArrangement::Masked {
                at: PathBuf::from("/run/spine/root/repo/.spine/cache"),
            },
        }
    }
}
