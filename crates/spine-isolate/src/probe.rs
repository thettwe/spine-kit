//! The probe, and the four tests (RF §7.1).
//!
//! *"Establishing a boundary is not creating one. It is creating one **and
//! passing a test the collector performs and can fail** — the property §7.4
//! rule 5 rests on, since a header alone proves nothing and a collector that
//! merely *configured* a boundary has measured nothing."*
//!
//! Step 6, in the order §7.1 fixes:
//!
//! 1. create the result directory (§3) and write a **canary** into it,
//!    `O_CREAT|O_EXCL`, under a name no other process can predict, holding its
//!    bytes in memory;
//! 2. create a **probe boundary** from exactly the **runner disposition** M1
//!    will use for a runner invocation, differing only in that its writable tree
//!    is an empty directory of the collector's own making;
//! 3. run P1, P2, P3 and P4 inside it;
//! 4. reap it, tear it down, and remove the canary. **No probe artifact
//!    survives step 6**, whatever the outcome.
//!
//! # What is a measurement and what is a claim
//!
//! This is the crate's only real subtlety, and RF §7.1 states it for P2 alone:
//! *"The host's view decides: a uid the probe reports is a claim, and an
//! identity mapping that does not reach the host is exactly the forgery a test
//! trusting the report would miss."*
//!
//! The same asymmetry runs through all four, and the deciders below are
//! arranged around it:
//!
//! - **P1** — the four attempts are the probe's claims; the **canary re-read on
//!   the host side after the probe is reaped** is the collector's own
//!   measurement, and is the limb that cannot be forged.
//! - **P2** — the four reported ids are claims; the `stat` of the created file
//!   is the collector's.
//! - **P3** — both limbs are the probe's report, and nothing the collector
//!   measures corroborates them.
//! - **P4** — both limbs are the probe's report. What makes (a) evidence rather
//!   than a claim is *where it was read from*: a netlink socket *"is answered by
//!   the namespace the calling task belongs to and cannot be pointed at
//!   another"* (RF §7.1, normative).
//!
//! So P3 and P4 are worth exactly the integrity of the probe process, which is
//! why prerequisite 4 names *"the binary the probe re-execs"*: the probe is the
//! collector's own hash-verified binary (PB §7.4 rule 2), re-exec'd inside the
//! boundary. A probe that is anything else turns every limb of P1–P4 into a
//! configuration claim, which is the thing §7.4 rule 3 refuses.

use crate::netlink::{Addr, Link};
use core::fmt;
use core::time::Duration;
use std::path::{Path, PathBuf};

/// The `(device, inode)` pair P3's separation limb is decided by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevIno {
    pub dev: u64,
    pub ino: u64,
}

impl DevIno {
    #[cfg(unix)]
    pub fn of(path: &Path) -> std::io::Result<Self> {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(path)?;
        Ok(DevIno {
            dev: meta.dev(),
            ino: meta.ino(),
        })
    }
}

/// Which of the four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Test {
    P1,
    P2,
    P3,
    P4,
}

impl Test {
    /// What the stderr diagnostic prints. RF §7.1 requires it to name *"which
    /// of P1, P2, P3 and P4"* failed.
    pub fn name(self) -> &'static str {
        match self {
            Test::P1 => "P1",
            Test::P2 => "P2",
            Test::P3 => "P3",
            Test::P4 => "P4",
        }
    }

    pub fn subject(self) -> &'static str {
        match self {
            Test::P1 => "containment",
            Test::P2 => "identity",
            Test::P3 => "separation",
            Test::P4 => "egress",
        }
    }
}

/// One test's verdict, with every limb that failed.
///
/// The reasons are kept because RF §7.1's diagnostic must let the human reading
/// the `G11` wire tell one failure from another, and *"P4 failed"* alone does
/// not distinguish an inherited bridge from a completed connect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestOutcome {
    pub test: Test,
    pub failures: Vec<String>,
}

impl TestOutcome {
    pub fn passing(test: Test) -> Self {
        TestOutcome {
            test,
            failures: Vec::new(),
        }
    }

    pub fn failing(test: Test, why: impl Into<String>) -> Self {
        TestOutcome {
            test,
            failures: vec![why.into()],
        }
    }

    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }

    fn fail(&mut self, why: impl Into<String>) {
        self.failures.push(why.into());
    }
}

impl fmt::Display for TestOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.passed() {
            write!(f, "{} ({}) passed", self.test.name(), self.test.subject())
        } else {
            write!(
                f,
                "{} ({}) failed: {}",
                self.test.name(),
                self.test.subject(),
                self.failures.join("; ")
            )
        }
    }
}

/// All four, and the only thing that licenses `profile=container`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub p1: TestOutcome,
    pub p2: TestOutcome,
    pub p3: TestOutcome,
    pub p4: TestOutcome,
}

impl ProbeReport {
    pub fn new(p1: TestOutcome, p2: TestOutcome, p3: TestOutcome, p4: TestOutcome) -> Self {
        ProbeReport { p1, p2, p3, p4 }
    }

    fn all(&self) -> [&TestOutcome; 4] {
        [&self.p1, &self.p2, &self.p3, &self.p4]
    }

    /// **P1 ∧ P2 ∧ P3 ∧ P4.** RF §7.1: *"There is no third outcome and no
    /// partial one: three tests out of four is `none`."*
    pub fn all_passed(&self) -> bool {
        self.all().iter().all(|o| o.passed())
    }

    pub fn failed(&self) -> Vec<Test> {
        self.all()
            .iter()
            .filter(|o| !o.passed())
            .map(|o| o.test)
            .collect()
    }

    pub fn failure_reasons(&self) -> Vec<String> {
        self.all()
            .iter()
            .filter(|o| !o.passed())
            .map(|o| o.to_string())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// The canary
// ---------------------------------------------------------------------------

/// RF §7.1 probe step 1, verbatim: the collector *"writes a **canary** into it,
/// `O_CREAT|O_EXCL`, **under a name no other process can predict**, holding its
/// bytes in memory"*.
///
/// Every clause is load-bearing. `O_EXCL` so the collector is the creator and
/// not the adopter of somebody else's file; the unpredictable name so a process
/// that already had the directory cannot have pre-opened it; and **in memory**
/// so the comparison at step 4 is against bytes the boundary never had a path
/// to. A canary whose expected bytes were re-read from disk would compare the
/// tampered file with itself.
#[derive(Debug)]
pub struct Canary {
    path: PathBuf,
    bytes: Vec<u8>,
}

impl Canary {
    /// Create the canary inside the result directory, which the collector has
    /// just created itself (§3).
    pub fn create(result_dir: &Path) -> std::io::Result<Self> {
        use std::io::Write;
        std::fs::create_dir_all(result_dir)?;
        let bytes = unpredictable_bytes()?;
        let name = format!("canary-{}", hex(&bytes));
        let path = result_dir.join(&name);
        // `create_new(true)` is `O_CREAT|O_EXCL` (std documents it as exactly
        // that). `create(true)` would silently adopt an existing file.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(Canary { path, bytes })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The bytes held **in memory**. Test-only and never public: a caller that
    /// could read these could also write them back over the file, which is the
    /// one thing the canary exists to detect.
    #[cfg(test)]
    fn expected(&self) -> &[u8] {
        &self.bytes
    }

    /// The host-side re-read of RF §7.1's P1: *"the canary's bytes, read back on
    /// the host side after the probe is reaped, are unchanged"*.
    ///
    /// A canary the probe **deleted** is also changed. `read` failing is
    /// therefore a failure and never a skipped limb.
    pub fn reread_unchanged(&self) -> bool {
        std::fs::read(&self.path)
            .map(|got| got == self.bytes)
            .unwrap_or(false)
    }

    /// Probe step 4: *"removes the canary. **No probe artifact survives step
    /// 6**, whatever the outcome."*
    pub fn remove(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            // The probe having already removed it is a P1 failure, not a
            // teardown failure: what step 4 owes is that nothing survives.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// 32 bytes from the kernel CSPRNG. *"A name no other process can predict"* is a
/// property of the entropy, not of the encoding — the hex below only makes it a
/// filename.
fn unpredictable_bytes() -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut buf = vec![0u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut buf)?;
    Ok(buf)
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from_digit((b >> 4) as u32, 16).expect("nibble"));
        out.push(char::from_digit((b & 0xf) as u32, 16).expect("nibble"));
    }
    out
}

// ---------------------------------------------------------------------------
// The collector's own facts, measured on the host side
// ---------------------------------------------------------------------------

/// `U`, `Ug`, the collector's pid, and the collector's root — RF §7.1: *"`U` and
/// `Ug` below are the collector's own real uid and gid."*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostView {
    pub u: u32,
    pub ug: u32,
    pub pid: u32,
    pub root: DevIno,
}

impl HostView {
    #[cfg(unix)]
    pub fn current() -> std::io::Result<Self> {
        Ok(HostView {
            u: crate::sys::getuid_real(),
            ug: crate::sys::getgid_real(),
            pid: std::process::id(),
            root: DevIno::of(Path::new("/"))?,
        })
    }
}

// ---------------------------------------------------------------------------
// P1 — Containment
// ---------------------------------------------------------------------------

/// One of P1's four attempts. `Failed` is the **passing** value: *"One success
/// is a failed test."*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attempt {
    Failed,
    Succeeded,
}

impl Attempt {
    fn from_result<T, E>(r: Result<T, E>) -> Self {
        match r {
            Ok(_) => Attempt::Succeeded,
            Err(_) => Attempt::Failed,
        }
    }
}

/// RF §7.1 P1: *"By absolute path, the probe attempts four things: (a) read the
/// canary, (b) write to it, (c) create a file at
/// `.spine/cache/results/<T>.jsonl`, (d) remove the result directory — at
/// whichever path the collector is actually using it, since §3 admits two
/// arrangements for `container`."*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct P1Attempts {
    pub read_canary: Attempt,
    pub write_canary: Attempt,
    pub create_result_file: Attempt,
    pub remove_result_dir: Attempt,
}

/// P1's decision.
///
/// The canary limb is **re-read here**, by the collector, rather than taken as a
/// parameter — *"the canary's bytes, read back on the host side after the probe
/// is reaped, are unchanged"*. A `bool` argument would let a caller hand P1 the
/// answer, which is the shape of passing a test without performing it. Call this
/// only after the probe's process group has been reaped.
pub fn decide_p1(canary: &Canary, attempts: &P1Attempts) -> TestOutcome {
    let mut outcome = TestOutcome::passing(Test::P1);
    let limbs: [(&str, Attempt); 4] = [
        ("(a) read the canary", attempts.read_canary),
        ("(b) write to the canary", attempts.write_canary),
        (
            "(c) create the result file at its absolute path",
            attempts.create_result_file,
        ),
        (
            "(d) remove the result directory",
            attempts.remove_result_dir,
        ),
    ];
    for (name, attempt) in limbs {
        if attempt == Attempt::Succeeded {
            // "a boundary the result file can be written from is not a
            // boundary, and this is the single relation `container` has ever
            // asserted".
            outcome.fail(format!("{name} succeeded"));
        }
    }
    if !canary.reread_unchanged() {
        outcome.fail("the canary's bytes changed, or it was removed");
    }
    outcome
}

/// The probe's side of P1, run **inside** the boundary, by absolute path.
///
/// **Limb (d) is `remove_dir_all` and not `rmdir`, and that is not a
/// liberty.** The result directory holds the canary, so a bare `rmdir` fails
/// with `ENOTEMPTY` on every host, inside a boundary or not — a limb that
/// passes without measuring anything, which is exactly the shape RF §13 R36 was
/// written against. `remove_dir_all` asks the question the limb means: *can a
/// process in the boundary destroy the result directory*. It runs last so that
/// a success does not disturb limbs (a) to (c).
pub fn attempt_containment(canary: &Path, result_file: &Path, result_dir: &Path) -> P1Attempts {
    // **Not** `debug_assert!`. P1 is specified "By absolute path", and a
    // release binary with the assertion compiled out would silently measure the
    // probe's cwd instead of the result directory — a limb that answers a
    // different question and cannot fail. The collector this ships in is a
    // release build.
    assert!(
        canary.is_absolute() && result_file.is_absolute() && result_dir.is_absolute(),
        "P1's four attempts are made by absolute path (RF §7.1)"
    );
    let read_canary = Attempt::from_result(std::fs::read(canary));
    let write_canary = Attempt::from_result(append_a_byte(canary));
    // `create(true)` and not `create_new(true)`: the attack the limb models is
    // a process in the boundary writing the file the trusted stage will ingest,
    // and that process would happily overwrite.
    let create_result_file = Attempt::from_result(std::fs::write(result_file, b"forged\n"));
    let remove_result_dir = Attempt::from_result(std::fs::remove_dir_all(result_dir));

    // Probe step 4: "No probe artifact survives step 6, whatever the outcome."
    //
    // Limb (c) SUCCEEDING is the case that matters, and it was the case with no
    // cleanup: a boundary that failed P1 left `forged\n` sitting at
    // `.spine/cache/results/<T>.jsonl` — the exact path RF §8.1 makes the
    // untrusted job's one artifact and the trusted stage ingests. The run
    // continues under `profile=none` after a failed probe, so the collector
    // would then have written its real file over a forgery, or worse, not
    // written one and left the forgery to be ingested.
    //
    // Removal is unconditional and its own result is discarded: this is
    // cleanup, not a fifth measurement, and a failure to clean up must not
    // change what P1 decided.
    if create_result_file == Attempt::Succeeded {
        let _ = std::fs::remove_file(result_file);
    }

    P1Attempts {
        read_canary,
        write_canary,
        create_result_file,
        remove_result_dir,
    }
}

fn append_a_byte(path: &Path) -> std::io::Result<()> {
    use std::io::Write;
    // Opening for write is not writing: a read-only bind mount can refuse at
    // `open`, but an overlay can refuse at the first page fault instead, so the
    // limb is not answered until bytes have actually been flushed.
    let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
    file.write_all(b"x")?;
    file.flush()
}

// ---------------------------------------------------------------------------
// P2 — Identity
// ---------------------------------------------------------------------------

/// What the probe **claims** about itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct P2Report {
    pub real_uid: u32,
    pub effective_uid: u32,
    pub real_gid: u32,
    pub effective_gid: u32,
}

/// What the **host** sees of the file the probe created in its writable tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ownership {
    pub uid: u32,
    pub gid: u32,
}

/// RF §7.1 P2: *"The reported ids all lie outside `{0, U, Ug}`, **and** the
/// created file's owner as *the host* sees it is neither `U` nor 0."*
///
/// `host_stat` is `None` where the collector could not `stat` the file — the
/// probe created nothing, or created it somewhere the collector cannot see. That
/// is a failure and not a skipped limb: the whole point of the limb is that the
/// host confirms it.
///
/// Note what the spec does **not** ask for: the host limb reads the file's
/// **owner**, and says nothing about its group. That is implemented as written.
pub fn decide_p2(host: &HostView, report: &P2Report, host_stat: Option<Ownership>) -> TestOutcome {
    let mut outcome = TestOutcome::passing(Test::P2);
    let excluded = [0u32, host.u, host.ug];
    let reported: [(&str, u32); 4] = [
        ("real uid", report.real_uid),
        ("effective uid", report.effective_uid),
        ("real gid", report.real_gid),
        ("effective gid", report.effective_gid),
    ];
    for (what, id) in reported {
        if excluded.contains(&id) {
            outcome.fail(format!(
                "the probe's reported {what} is {id}, inside {{0, U={}, Ug={}}}",
                host.u, host.ug
            ));
        }
    }
    match host_stat {
        None => outcome.fail(
            "the collector could not stat the file the probe claims to have created \
             — the host's view is what decides, and there was none",
        ),
        Some(owner) => {
            if owner.uid == host.u || owner.uid == 0 {
                outcome.fail(format!(
                    "the host sees the probe's file owned by uid {}, which is U or 0",
                    owner.uid
                ));
            }
        }
    }
    outcome
}

/// The probe's side of P2: report the real and effective pair, and create a file
/// in the writable tree for the collector to `stat`.
#[cfg(unix)]
pub fn measure_identity(writable_tree: &Path) -> (P2Report, std::io::Result<PathBuf>) {
    let report = P2Report {
        real_uid: crate::sys::getuid_real(),
        effective_uid: crate::sys::geteuid_eff(),
        real_gid: crate::sys::getgid_real(),
        effective_gid: crate::sys::getegid_eff(),
    };
    let path = writable_tree.join("p2-owner-probe");
    let created = std::fs::write(&path, b"p2\n").map(|()| path);
    (report, created)
}

// ---------------------------------------------------------------------------
// P3 — Separation
// ---------------------------------------------------------------------------

/// What the probe reports about its own process table and root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct P3Report {
    /// Every pid visible to the probe, **numbered in the probe's own PID
    /// namespace**.
    pub pids: Vec<u32>,
    /// The probe's own pid in that same numbering — 1 under a fresh PID
    /// namespace.
    pub own_pid: u32,
    pub root: DevIno,
}

/// RF §7.1 P3: *"The collector's own pid is **absent** from that table, **and**
/// the probe's root is a different `(device, inode)` pair from the collector's
/// root."*
///
/// **The first limb compares numbers drawn from two different namespaces**, and
/// that is what the spec says to do. Under M1 the probe's table is renumbered
/// from 1, so the collector's host pid is absent by construction and the limb
/// measures nothing — except in the one case where it produces a *false fail*,
/// a collector whose host pid happens to collide with a pid inside the child's
/// namespace (pid 1 on a runner whose collector is the container's init). The
/// limb that actually decides is the second: the `(device, inode)` pair, which
/// differs *"because of something the collector built"*. Reported as a corpus
/// defect; implemented as written, because the spec is normative and a
/// unilateral strengthening here would make two conforming collectors disagree.
///
/// The `own_pid`-present check is this implementation's, not the spec's: an
/// empty or self-less process table is an enumeration that did not happen, and
/// a limb over an empty set passes vacuously.
pub fn decide_p3(host: &HostView, report: &P3Report) -> TestOutcome {
    let mut outcome = TestOutcome::passing(Test::P3);
    if !report.pids.contains(&report.own_pid) {
        // DERIVED: the corpus fixes the two limbs (RF §7.1 P3) and says nothing
        // about an enumeration that failed. Treating an empty table as a pass
        // is the way to pass P3 without measuring anything.
        outcome
            .fail("the probe's process table does not contain the probe — nothing was enumerated");
    }
    if report.pids.contains(&host.pid) {
        outcome.fail(format!(
            "the collector's own pid {} is visible in the probe's process table",
            host.pid
        ));
    }
    if report.root == host.root {
        outcome.fail(format!(
            "the probe's root is the collector's root, (dev {}, ino {}) — \
             a mount namespace over the job's root *is* the job's root",
            host.root.dev, host.root.ino
        ));
    }
    outcome
}

// ---------------------------------------------------------------------------
// P4 — Egress
// ---------------------------------------------------------------------------

/// An enumeration the probe either performed or could not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Enumeration<T> {
    Read(Vec<T>),
    /// The netlink socket could not be opened, the dump could not be read, or
    /// the dump did not parse. **Not an empty set** — see [`decide_p4`].
    Unavailable(String),
}

/// RF §7.1 P4(b), verbatim: *"A connect that completes is a failed test, and a
/// connect still pending when the bound expires is **also** a failed test:
/// pending means a route existed."*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectOutcome {
    /// The kernel refused it — no route, unreachable, refused. The **passing**
    /// value.
    Failed,
    Completed,
    PendingAtBound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct P4Report {
    pub links: Enumeration<Link>,
    pub addrs: Enumeration<Addr>,
    pub connect: ConnectOutcome,
}

/// RF §7.1 P4, as amended by §13 R36:
///
/// **(a)** *"no interface other than loopback is `IFF_UP`, and the namespace
/// holds no address of any family other than `127.0.0.1/8` and `::1/128`"* —
/// the **reachable** set, not the device count. The original *"exactly one
/// device, loopback"* failed on every host with the tunnel modules loaded,
/// which is the distribution default: *"a fresh namespace is not an empty
/// one"*.
///
/// **(b)** the non-blocking connect to `192.0.2.1:443`, bounded at one second,
/// fails.
///
/// *"The two limbs check each other and neither alone would do."* — so neither
/// is droppable and both are evaluated here.
pub fn decide_p4(report: &P4Report) -> TestOutcome {
    let mut outcome = TestOutcome::passing(Test::P4);

    match &report.links {
        // DERIVED: the corpus does not say what an enumeration that failed
        // means. It cannot mean "pass": P4(a) passes on an absence, so an
        // unavailable dump and a clean namespace would be indistinguishable,
        // and `profile=container` would be licensed by a socket that never
        // opened.
        Enumeration::Unavailable(why) => {
            outcome.fail(format!("P4(a) could not enumerate interfaces: {why}"))
        }
        Enumeration::Read(links) if links.is_empty() => {
            // Likewise DERIVED. A live network namespace always holds `lo`; a
            // dump of zero links is a dump that did not happen.
            outcome.fail("P4(a) enumerated no interface at all, not even loopback")
        }
        Enumeration::Read(links) => {
            for link in links {
                if link.is_up() && !link.is_loopback() {
                    outcome.fail(format!(
                        "interface {} is IFF_UP and is not loopback",
                        link.name
                    ));
                }
            }
            // The third clause, and it is not symmetry with the second.
            //
            // A fresh network namespace creates loopback **down** —
            // `IFF_LOOPBACK` alone, measured — so a test written only as
            // *nothing but loopback is up* passes a namespace in which the
            // runner cannot reach `127.0.0.1` either. RF §7.1's prerequisite 5
            // is "a network namespace … **with a loopback device it can bring
            // up**", and the mechanism paragraph makes bringing it up a step
            // the collector performs; this is the test of that step.
            if !links.iter().any(|link| link.is_loopback() && link.is_up()) {
                outcome.fail(
                    "P4(a): loopback is not IFF_UP, so the namespace has no usable                      interface — RF §7.1 promises a runner reaches 127.0.0.1 and ::1",
                );
            }
        }
    }

    match &report.addrs {
        Enumeration::Unavailable(why) => {
            outcome.fail(format!("P4(a) could not enumerate addresses: {why}"))
        }
        Enumeration::Read(addrs) => {
            for addr in addrs {
                if !addr.is_the_loopback_v4() && !addr.is_the_loopback_v6() {
                    outcome.fail(format!(
                        "the namespace holds address family {} prefix /{} — \
                         the only two admitted are 127.0.0.1/8 and ::1/128",
                        addr.family, addr.prefix_len
                    ));
                }
            }
        }
    }

    match report.connect {
        ConnectOutcome::Failed => {}
        ConnectOutcome::Completed => outcome.fail("P4(b) the connect to 192.0.2.1:443 completed"),
        ConnectOutcome::PendingAtBound => outcome.fail(
            "P4(b) the connect to 192.0.2.1:443 was still pending at one second — \
             pending means a route existed",
        ),
    }

    outcome
}

/// P4(b)'s target, RF §7.1 verbatim: *"`192.0.2.1:443` — RFC 5737 TEST-NET-1,
/// an address that is unroutable on the public internet by definition, so no
/// packet the limb emits can ever reach a third party"*.
pub const EGRESS_TARGET: ([u8; 4], u16) = ([192, 0, 2, 1], 443);

/// *"the collector bounds the attempt at **one second**"*.
pub const EGRESS_BOUND: Duration = Duration::from_secs(1);

/// The probe's side of P4(b).
///
/// `TcpStream::connect_timeout` is exactly the spec's construction: it puts the
/// socket in non-blocking mode, calls `connect(2)`, and polls to the bound. A
/// plain blocking `connect` would not distinguish *pending at one second* from
/// *failed*, and the spec fails the test on both — but for different reasons a
/// human reading the diagnostic needs to tell apart.
pub fn attempt_egress() -> ConnectOutcome {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
    let (octets, port) = EGRESS_TARGET;
    let target = SocketAddr::new(IpAddr::V4(Ipv4Addr::from(octets)), port);
    match TcpStream::connect_timeout(&target, EGRESS_BOUND) {
        Ok(_stream) => ConnectOutcome::Completed,
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => ConnectOutcome::PendingAtBound,
        Err(_) => ConnectOutcome::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netlink::{self, AF_INET, AF_INET6};

    fn host() -> HostView {
        HostView {
            u: 1001,
            ug: 1001,
            pid: 4242,
            root: DevIno {
                dev: 79,
                ino: 34252584,
            },
        }
    }

    fn all_attempts_failed() -> P1Attempts {
        P1Attempts {
            read_canary: Attempt::Failed,
            write_canary: Attempt::Failed,
            create_result_file: Attempt::Failed,
            remove_result_dir: Attempt::Failed,
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("spine-isolate-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    /// RF §7.1 probe step 1: *"under a name no other process can predict"*. Two
    /// canaries in one directory must not collide, and neither name may be a
    /// function of the directory.
    #[test]
    fn a_canary_name_is_not_predictable_from_the_directory() {
        let dir = scratch("canary-names");
        let a = Canary::create(&dir).unwrap();
        let b = Canary::create(&dir).unwrap();
        assert_ne!(a.path(), b.path());
        assert_ne!(a.expected(), b.expected());
        assert_eq!(a.expected().len(), 32);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// *"holding its bytes in memory"*. The comparison must be against the
    /// held bytes: re-reading the expectation from disk would compare the
    /// tampered file with itself and pass every tamper.
    #[test]
    fn the_canary_is_compared_against_bytes_the_boundary_never_had_a_path_to() {
        let dir = scratch("canary-tamper");
        let canary = Canary::create(&dir).unwrap();
        assert!(canary.reread_unchanged());

        std::fs::write(canary.path(), b"forged").unwrap();
        assert!(!canary.reread_unchanged(), "a rewritten canary is changed");

        std::fs::remove_file(canary.path()).unwrap();
        assert!(!canary.reread_unchanged(), "a removed canary is changed");

        // Probe step 4 must leave nothing behind whatever the outcome — an
        // already-removed canary included.
        canary.remove().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RF §7.1 P1: *"All four fail, **and** the canary's bytes … are unchanged.
    /// **One success is a failed test.**"*
    #[test]
    fn one_p1_limb_succeeding_is_a_failed_test() {
        let dir = scratch("p1");
        let canary = Canary::create(&dir).unwrap();

        assert!(decide_p1(&canary, &all_attempts_failed()).passed());

        type Set = fn(&mut P1Attempts);
        let setters: [(&str, Set); 4] = [
            ("read", |a| a.read_canary = Attempt::Succeeded),
            ("write", |a| a.write_canary = Attempt::Succeeded),
            ("create", |a| a.create_result_file = Attempt::Succeeded),
            ("remove", |a| a.remove_result_dir = Attempt::Succeeded),
        ];
        for (which, set) in setters {
            let mut attempts = all_attempts_failed();
            set(&mut attempts);
            assert!(
                !decide_p1(&canary, &attempts).passed(),
                "{which} succeeding must fail P1"
            );
        }

        // The host-side limb is independent of all four: a probe can honestly
        // report four failures and still have changed the canary, which is the
        // forgery the re-read exists to catch.
        std::fs::write(canary.path(), b"forged").unwrap();
        assert!(!decide_p1(&canary, &all_attempts_failed()).passed());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RF §7.1 P2: *"The reported ids all lie outside `{0, U, Ug}`."* Every one
    /// of the four, and every member of the set.
    #[test]
    fn every_reported_id_must_lie_outside_zero_u_and_ug() {
        let host = host();
        let good = P2Report {
            real_uid: 100000,
            effective_uid: 100000,
            real_gid: 100000,
            effective_gid: 100000,
        };
        let seen = Some(Ownership {
            uid: 100000,
            gid: 100000,
        });
        assert!(decide_p2(&host, &good, seen).passed());

        for forbidden in [0, host.u, host.ug] {
            for field in 0..4 {
                let mut report = good;
                match field {
                    0 => report.real_uid = forbidden,
                    1 => report.effective_uid = forbidden,
                    2 => report.real_gid = forbidden,
                    _ => report.effective_gid = forbidden,
                }
                assert!(
                    !decide_p2(&host, &report, seen).passed(),
                    "id {forbidden} in field {field} must fail P2"
                );
            }
        }
    }

    /// RF §7.1 P2: *"The host's view decides: a uid the probe reports is a
    /// claim, and an identity mapping that does not reach the host is exactly
    /// the forgery a test trusting the report would miss."*
    #[test]
    fn a_uid_the_probe_reports_is_a_claim_and_the_host_stat_decides() {
        let host = host();
        let honest_looking = P2Report {
            real_uid: 100000,
            effective_uid: 100000,
            real_gid: 100000,
            effective_gid: 100000,
        };
        // The forgery: perfect ids, and a file the host sees owned by `U`.
        let forged = Some(Ownership {
            uid: host.u,
            gid: host.ug,
        });
        assert!(!decide_p2(&host, &honest_looking, forged).passed());

        // And the same with 0, which is the other half of the host limb.
        let as_root = Some(Ownership { uid: 0, gid: 0 });
        assert!(!decide_p2(&host, &honest_looking, as_root).passed());

        // No file at all is a failure, never a skipped limb.
        assert!(!decide_p2(&host, &honest_looking, None).passed());
    }

    /// RF §7.1 P3, second limb — and RF §7.1's reason for the overlay: *"A
    /// mount namespace over the job's root *is* the job's root — `stat` on the
    /// child's `/` returns the collector's own `(device, inode)` pair, so P3's
    /// separation limb would fail on every host, for every configuration,
    /// forever."*
    #[test]
    fn a_probe_root_equal_to_the_collectors_root_fails_p3_forever() {
        let host = host();
        let bare_mount_namespace = P3Report {
            pids: vec![1],
            own_pid: 1,
            root: host.root,
        };
        assert!(!decide_p3(&host, &bare_mount_namespace).passed());

        // The overlay's measured pair, RF §7.1: "overlay dev=97 ino=2".
        let overlay = P3Report {
            pids: vec![1],
            own_pid: 1,
            root: DevIno { dev: 97, ino: 2 },
        };
        assert!(decide_p3(&host, &overlay).passed());
    }

    /// The collector must be invisible to the probe.
    #[test]
    fn the_collectors_pid_visible_to_the_probe_fails_p3() {
        let host = host();
        let leaky = P3Report {
            pids: vec![1, host.pid],
            own_pid: 1,
            root: DevIno { dev: 97, ino: 2 },
        };
        assert!(!decide_p3(&host, &leaky).passed());
    }

    /// DERIVED guard: a table that does not contain the probe is a table that
    /// was never read, and both of P3's limbs pass vacuously over an empty set.
    #[test]
    fn an_empty_process_table_is_an_enumeration_that_did_not_happen() {
        let host = host();
        let vacuous = P3Report {
            pids: vec![],
            own_pid: 1,
            root: DevIno { dev: 97, ino: 2 },
        };
        assert!(!decide_p3(&host, &vacuous).passed());
    }

    fn measured_namespace() -> P4Report {
        let links = netlink::parse_links(&netlink::build::the_runner_disposition()).unwrap();
        let mut v6 = [0u8; 16];
        v6[15] = 1;
        let mut dump = netlink::build::addr_message(1, AF_INET, 8, &[127, 0, 0, 1]);
        dump.extend_from_slice(&netlink::build::addr_message(1, AF_INET6, 128, &v6));
        dump.extend_from_slice(&netlink::build::done());
        P4Report {
            links: Enumeration::Read(links),
            addrs: Enumeration::Read(netlink::parse_addrs(&dump).unwrap()),
            connect: ConnectOutcome::Failed,
        }
    }

    /// RF §13 R36(ii), reproduced from netlink bytes. The namespace RF §7.1
    /// measured — nine down tunnel devices beside `lo` — **passes** P4(a) as
    /// amended, and would have failed the withdrawn *"exactly one device,
    /// loopback"* wording on every such host forever.
    /// The clause the 2026-08-27 re-measurement added, and the reason it is
    /// three clauses and not two.
    ///
    /// A **bare** `unshare(CLONE_NEWNET)` namespace — loopback present but
    /// **down** — must FAIL P4, because RF §7.1 promises a runner reaches
    /// `127.0.0.1` and `::1` and this namespace has no usable interface at all.
    /// Prerequisite 5 is "a network namespace … with a loopback device it can
    /// bring up"; bringing it up is a step the collector performs, and this is
    /// the test of that step.
    ///
    /// Without the third clause this fixture passes, which is why the clause
    /// exists: the boundary would be licensed as `container` while being
    /// unusable rather than merely isolated.
    #[test]
    fn a_namespace_whose_loopback_was_never_brought_up_fails_p4() {
        // The runner disposition, with the one bit that separates it from a
        // bare `unshare(CLONE_NEWNET)` put back the way the kernel leaves it.
        let mut report = measured_namespace();
        report.links = Enumeration::Read(
            netlink::parse_links(&netlink::build::the_measured_fresh_namespace()).unwrap(),
        );

        let outcome = decide_p4(&report);
        assert!(
            !outcome.passed(),
            "a namespace with loopback down has no usable interface"
        );
        assert!(
            outcome
                .failures
                .iter()
                .any(|r| r.contains("loopback is not IFF_UP")),
            "and the diagnostic must say which clause failed: {:?}",
            outcome.failures
        );
    }

    #[test]
    fn the_runner_disposition_passes_p4_as_amended() {
        let report = measured_namespace();
        let outcome = decide_p4(&report);
        assert!(outcome.passed(), "{outcome}");

        // The withdrawn reading, stated as the thing it would have done.
        let Enumeration::Read(links) = &report.links else {
            unreachable!()
        };
        assert_eq!(
            links.len(),
            10,
            "a device count of one was never the property"
        );
    }

    /// *"a veth pair moved in, a bridge, an inherited interface"* — the threat
    /// P4(a) exists to detect. Each must be up to carry traffic.
    #[test]
    fn an_inherited_interface_that_is_up_fails_p4() {
        let mut report = measured_namespace();
        let Enumeration::Read(links) = &mut report.links else {
            unreachable!()
        };
        links.push(Link {
            index: 11,
            name: "eth0".into(),
            flags: netlink::IFF_UP,
        });
        let outcome = decide_p4(&report);
        assert!(!outcome.passed());
        assert!(outcome.to_string().contains("eth0"), "{outcome}");
    }

    /// *"no address of **any family** other than `127.0.0.1/8` and
    /// `::1/128`"*. A routable address on a down device still fails.
    #[test]
    fn an_address_that_is_not_loopbacks_fails_p4() {
        let mut report = measured_namespace();
        let Enumeration::Read(addrs) = &mut report.addrs else {
            unreachable!()
        };
        addrs.push(Addr {
            index: 11,
            family: AF_INET,
            prefix_len: 24,
            bytes: vec![10, 0, 0, 5],
        });
        assert!(!decide_p4(&report).passed());
    }

    /// RF §7.1 P4(b): *"A connect that completes is a failed test, and a connect
    /// still pending when the bound expires is **also** a failed test: pending
    /// means a route existed."*
    #[test]
    fn a_completed_connect_and_a_pending_one_both_fail_p4() {
        for bad in [ConnectOutcome::Completed, ConnectOutcome::PendingAtBound] {
            let mut report = measured_namespace();
            report.connect = bad;
            assert!(!decide_p4(&report).passed(), "{bad:?}");
        }
    }

    /// *"The two limbs check each other and neither alone would do."* A clean
    /// interface set does not excuse a completed connect, and a failed connect
    /// does not excuse an inherited interface.
    #[test]
    fn neither_p4_limb_alone_licenses_the_boundary() {
        let mut only_a = measured_namespace();
        only_a.connect = ConnectOutcome::Completed;
        assert!(!decide_p4(&only_a).passed());

        let mut only_b = measured_namespace();
        let Enumeration::Read(links) = &mut only_b.links else {
            unreachable!()
        };
        links.push(Link {
            index: 11,
            name: "veth0".into(),
            flags: netlink::IFF_UP,
        });
        assert!(!decide_p4(&only_b).passed());
    }

    /// DERIVED: an enumeration that failed must not read as a clean namespace.
    /// P4(a) passes on an absence, so this is the one place where doing nothing
    /// would look like success.
    #[test]
    fn an_unavailable_enumeration_is_a_failed_test_and_not_a_clean_namespace() {
        let mut report = measured_namespace();
        report.links = Enumeration::Unavailable("no netlink socket".into());
        assert!(!decide_p4(&report).passed());

        let mut report = measured_namespace();
        report.addrs = Enumeration::Unavailable("EPERM".into());
        assert!(!decide_p4(&report).passed());

        let mut report = measured_namespace();
        report.links = Enumeration::Read(vec![]);
        assert!(
            !decide_p4(&report).passed(),
            "a live namespace always holds lo"
        );
    }

    /// RF §7.1: the target and the bound are fixed, and the target is
    /// deliberately unroutable *"so no packet the limb emits can ever reach a
    /// third party"*.
    #[test]
    fn p4bs_target_is_test_net_1_and_the_bound_is_one_second() {
        assert_eq!(EGRESS_TARGET, ([192, 0, 2, 1], 443));
        assert_eq!(EGRESS_BOUND, Duration::from_secs(1));
    }

    /// The four names the diagnostic prints.
    #[test]
    fn a_report_names_which_of_the_four_failed() {
        let report = ProbeReport::new(
            TestOutcome::passing(Test::P1),
            TestOutcome::failing(Test::P2, "the host sees uid 0"),
            TestOutcome::passing(Test::P3),
            TestOutcome::failing(Test::P4, "eth0 is IFF_UP"),
        );
        assert!(!report.all_passed());
        assert_eq!(report.failed(), vec![Test::P2, Test::P4]);
        assert_eq!(report.failure_reasons().len(), 2);
    }
}
