//! M1's five host prerequisites (RF §7.1).
//!
//! *"They are stated as prerequisites rather than assumed because a host that
//! lacks one is not a broken host — it is a host whose landings a human reads."*
//! Every absence is **disposition 2 and not a refusal**: the collector tears
//! down whatever it built, records `profile=none`, **names which prerequisite
//! failed on stderr**, and runs the suite unisolated.
//!
//! The count is **five**, not four. PB §12's change log says *"four host
//! prerequisites"* in one sentence and *"the container prerequisite stack stays
//! at five"* in another; RF §7.1 prints a five-row table and says *"M1 requires,
//! all five"*. Under RF §1's authority rule the spec is normative and PB's
//! change-log prose is not §11 Vocabulary, so it is five and the "four" is a
//! stale count from before the network namespace became prerequisite 5.

use std::path::{Path, PathBuf};

/// RF §7.1's prerequisite table, in its own order. The **number** is what the
/// stderr diagnostic names, so the discriminants are the table's rows and not
/// an implementation ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prerequisite {
    /// 1 — *"the four filesystem-and-process namespaces — mount, PID, IPC, user
    /// — creatable by this collector; the **network** namespace is prerequisite
    /// 5"*.
    Namespaces,
    /// 2 — *"an identity source — a delegated `subuid`/`subgid` range with
    /// `newuidmap`/`newgidmap`, or a collector running as uid 0"*.
    IdentitySource,
    /// 3 — *"a read-only overlay over the job's root, mountable inside the
    /// namespace, **and a filesystem the collector may mount its second, empty
    /// lower layer on**"*. The second clause is RF §13 R36's: without it the
    /// kernel refuses the mount and P3 fails on every host, forever.
    OverlayRoot,
    /// 4 — *"a filesystem the mapped id can traverse — the checkouts of `B` and
    /// `T`, and the binary the probe re-execs"*.
    TraversableFilesystem,
    /// 5 — *"a **network namespace**, creatable by this collector, with a
    /// loopback device it can bring up"*.
    NetworkNamespace,
}

impl Prerequisite {
    /// The five, in table order. Order is load-bearing: [`check`] reports the
    /// **first** absence, and the number it prints is what the human reading
    /// the `G11` wire looks up.
    pub const ALL: [Prerequisite; 5] = [
        Prerequisite::Namespaces,
        Prerequisite::IdentitySource,
        Prerequisite::OverlayRoot,
        Prerequisite::TraversableFilesystem,
        Prerequisite::NetworkNamespace,
    ];

    /// The row number in RF §7.1's table.
    pub fn number(self) -> u8 {
        match self {
            Prerequisite::Namespaces => 1,
            Prerequisite::IdentitySource => 2,
            Prerequisite::OverlayRoot => 3,
            Prerequisite::TraversableFilesystem => 4,
            Prerequisite::NetworkNamespace => 5,
        }
    }

    /// The table's *Prerequisite* column, condensed.
    pub fn summary(self) -> &'static str {
        match self {
            Prerequisite::Namespaces => {
                "the mount, PID, IPC and user namespaces, creatable by this collector"
            }
            Prerequisite::IdentitySource => {
                "an identity source — a delegated subuid/subgid range with newuidmap/newgidmap, \
                 or a collector running as uid 0"
            }
            Prerequisite::OverlayRoot => {
                "a read-only overlay over the job's root, and a filesystem to mount its second, \
                 empty lower layer on"
            }
            Prerequisite::TraversableFilesystem => "a filesystem the mapped id can traverse",
            Prerequisite::NetworkNamespace => {
                "a network namespace with a loopback device it can bring up"
            }
        }
    }
}

/// RF §7.1 *M1's identity source*: **exactly two** arrangements, one required.
///
/// *"Which of the two a host supplies never reaches the file. Both are M1, both
/// license `container`, neither is recorded, and the header carries no trace of
/// the difference — so two collectors on two hosts of different arrangements
/// write the same bytes."* Nothing in this crate serializes this type, and that
/// is the rule, not an oversight.
///
/// A bare unprivileged user namespace is **not** a third arrangement: *"it maps
/// exactly one host uid, `U` itself, so the file P2 `stat`s comes back owned by
/// `U` and the test fails on every host, for every configuration, forever."*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    /// `/etc/subuid` + `/etc/subgid` with `newuidmap`/`newgidmap` — equivalently
    /// `CAP_SETUID`/`CAP_SETGID` on the host.
    ///
    /// `first_id` is the **start of the range the host actually delegated**,
    /// not a constant. It is carried rather than rediscovered because it is the
    /// outside column of the child's `uid_map`, and a mapping outside the
    /// delegated range is one `newuidmap` refuses.
    DelegatedSubordinateRange { first_id: u32 },
    /// *"A root collector that drops privilege"* — `U` is 0, the collector maps
    /// the child to a fixed unprivileged id, *"and any non-zero id satisfies
    /// P2"*.
    RootCollectorDroppingPrivilege,
}

/// The seam. Every check is a host fact no git object records, so it is a trait
/// and the deciding is [`check`], which is pure.
pub trait HostFacts {
    fn namespaces_creatable(&self) -> bool;
    fn identity_source(&self) -> Option<IdentitySource>;
    fn overlay_root_available(&self) -> bool;
    fn mapped_id_can_traverse(&self) -> bool;
    fn network_namespace_available(&self) -> bool;
}

/// Check all five, in table order, and report the **first** absence.
///
/// Reporting the first rather than all five is deliberate: RF §7.1 requires the
/// diagnostic to name *"which prerequisite"*, singular, and a host missing
/// prerequisite 1 cannot meaningfully be asked about 2 through 5 — on Darwin
/// there is no namespace to look inside.
pub fn check(facts: &dyn HostFacts) -> Result<IdentitySource, Prerequisite> {
    if !facts.namespaces_creatable() {
        return Err(Prerequisite::Namespaces);
    }
    let Some(identity) = facts.identity_source() else {
        return Err(Prerequisite::IdentitySource);
    };
    if !facts.overlay_root_available() {
        return Err(Prerequisite::OverlayRoot);
    }
    if !facts.mapped_id_can_traverse() {
        return Err(Prerequisite::TraversableFilesystem);
    }
    if !facts.network_namespace_available() {
        return Err(Prerequisite::NetworkNamespace);
    }
    Ok(identity)
}

/// The real host.
///
/// **Every check here is necessary and none is sufficient**, and saying so is
/// the point. The sufficient test is building the boundary, whose failure is
/// *the same* disposition 2 by a different cause (RF §7.1: *"creation failed"*),
/// so a false positive here costs a clearer diagnostic and nothing else. A
/// false *negative* — reporting an absence a host does not have — would cost a
/// repository `profile=container` forever, which is the failure shape RF §13
/// R36 exists for, so each check below refuses only on evidence it actually
/// read.
#[derive(Debug, Clone)]
pub struct RealHost {
    /// A directory the collector may create mount points under — the second
    /// lower layer's `tmpfs`, the overlay root, the private scratch `tmpfs`.
    /// `.spine/ci.sh` `chmod 0700`s `$WORK` for exactly this (CI §5.4 item 1).
    scratch: PathBuf,
    /// Paths the **mapped id** must be able to traverse: at step 6 that is
    /// `$WORK` and `$BIN`, since prerequisite 4's *"checkouts of `B` and `T`"*
    /// do not exist yet — step 6 precedes step 7 (RF §7.1).
    traversal: Vec<PathBuf>,
}

impl RealHost {
    pub fn new(scratch: impl Into<PathBuf>, traversal: Vec<PathBuf>) -> Self {
        RealHost {
            scratch: scratch.into(),
            traversal,
        }
    }

    pub fn scratch(&self) -> &Path {
        &self.scratch
    }

    /// The paths prerequisite 4 is asked about.
    pub fn traversal(&self) -> &[PathBuf] {
        &self.traversal
    }
}

#[cfg(target_os = "linux")]
impl HostFacts for RealHost {
    fn namespaces_creatable(&self) -> bool {
        // The four namespace kinds must exist as concepts on this kernel: a
        // kernel built without `CONFIG_USER_NS` has no `/proc/self/ns/user`.
        // This does not prove `unshare` will succeed — a seccomp policy can
        // still refuse it — which is why creation failure is disposition 2 too.
        let present = ["mnt", "pid", "ipc", "user"]
            .iter()
            .all(|ns| Path::new(&format!("/proc/self/ns/{ns}")).exists());
        // The table's third absence, "a nested runner that already spent them",
        // is not observable here and is caught at creation.
        present && sysctl_positive("/proc/sys/user/max_user_namespaces")
    }

    fn identity_source(&self) -> Option<IdentitySource> {
        // Arrangement 2 first, because it is the one that needs no host file:
        // "the ordinary case on a container-based CI runner, where ci.md §7.2's
        // untrusted job is itself already a container".
        if crate::sys::getuid_real() == 0 {
            return Some(IdentitySource::RootCollectorDroppingPrivilege);
        }
        // Arrangement 1. Both halves are required: a delegated range with no
        // `newuidmap` cannot be written into the child's `uid_map` by an
        // unprivileged collector, and `newuidmap` with no range has nothing to
        // write.
        // Both halves are required and both ranges must exist: a uid range
        // with no matching gid range leaves `newgidmap` nothing to write, and
        // a child with an unmapped gid fails P2's gid limb.
        let uid_range = subordinate_range("/etc/subuid", crate::sys::getuid_real());
        let gid_range = subordinate_range("/etc/subgid", crate::sys::getgid_real());
        let has_helpers = which("newuidmap").is_some() && which("newgidmap").is_some();
        match (uid_range, gid_range, has_helpers) {
            (Some(first_id), Some(_), true) => {
                Some(IdentitySource::DelegatedSubordinateRange { first_id })
            }
            _ => None,
        }
    }

    fn overlay_root_available(&self) -> bool {
        // The kernel must know the filesystem at all...
        let Ok(filesystems) = std::fs::read_to_string("/proc/filesystems") else {
            return false;
        };
        if !filesystems.split_whitespace().any(|w| w == "overlay") {
            return false;
        }
        // ...and RF §13 R36 added the second clause: "no mount point available
        // for the `tmpfs` the second lower layer needs". Without a tmpfs the
        // lower set is a single layer, which the kernel refuses with EINVAL, or
        // a directory beneath `/`, which it refuses with ELOOP. Either way
        // `profile=container` would be unwritable on every host.
        self.scratch.is_dir()
    }

    fn mapped_id_can_traverse(&self) -> bool {
        // "the collector inherits a 0700 umask" — at which point every checkout
        // and every file under $INSTALL_DIR is unreachable to the mapped id and
        // M1 fails a prerequisite rather than a test (CI §5.4 item 1).
        //
        // The mapped id is in neither the owner nor the group of anything the
        // collector wrote, so what it needs is the *other* bits: `o+x` on every
        // directory on the path, and `o+r` (plus `o+x` for the binary) on the
        // leaf.
        //
        // **An empty traversal set is not a pass.** `all` over nothing is
        // `true`, so `RealHost::new(dir, vec![])` made prerequisite 4
        // unfalsifiable — a check that cannot fail, which is the named hazard
        // of this whole section. RF §7.1's prerequisite 4 names what must be
        // traversable: "the checkouts of `B` and `T`, and the binary the probe
        // re-execs". A collector that was handed none of them has not satisfied
        // the prerequisite; it has failed to ask.
        traversal_is_reachable(&self.traversal, other_can_reach)
    }

    fn network_namespace_available(&self) -> bool {
        Path::new("/proc/self/ns/net").exists()
            && sysctl_positive("/proc/sys/user/max_net_namespaces")
    }
}

/// Prerequisite 4's predicate, extracted so it is testable on every platform
/// and so its one subtlety is stated in one place.
///
/// The emptiness clause is the subtlety. RF §7.1 names what must be
/// traversable — "the checkouts of `B` and `T`, and the binary the probe
/// re-execs" — so a collector handed none of them has not satisfied the
/// prerequisite, it has failed to ask. `all` over nothing being `true` made
/// this the one prerequisite that could not fail.
pub fn traversal_is_reachable<F>(paths: &[PathBuf], reachable: F) -> bool
where
    F: Fn(&Path) -> bool,
{
    !paths.is_empty() && paths.iter().all(|p| reachable(p))
}

/// RF §7.1: *"M1 needs kernel namespaces and therefore exists on Linux only —
/// prerequisite 1. `ci.md` §5.5's platform table also ships a Darwin target,
/// where M1 cannot be created at all; **that is not a refusal but disposition
/// 2**."*
///
/// So the non-Linux host reports prerequisite 1 absent and every landing on it
/// records `profile=none`, loudly. It does not fail the job.
#[cfg(not(target_os = "linux"))]
impl HostFacts for RealHost {
    fn namespaces_creatable(&self) -> bool {
        false
    }
    fn identity_source(&self) -> Option<IdentitySource> {
        None
    }
    fn overlay_root_available(&self) -> bool {
        false
    }
    fn mapped_id_can_traverse(&self) -> bool {
        false
    }
    fn network_namespace_available(&self) -> bool {
        false
    }
}

/// A `/proc/sys` counter that must be greater than zero. **Absent is not zero**:
/// a kernel without the knob has no limit, and treating an unreadable file as a
/// refusal would deny the boundary on every host that predates the sysctl.
#[cfg(target_os = "linux")]
fn sysctl_positive(path: &str) -> bool {
    match std::fs::read_to_string(path) {
        Ok(text) => text.trim().parse::<u64>().map(|n| n > 0).unwrap_or(true),
        Err(_) => true,
    }
}

/// One `name:start:count` line in `/etc/subuid` or `/etc/subgid` whose first
/// field names this collector, by login name or by numeric id, and whose count
/// is non-zero. A zero-length range is a line that delegates nothing.
#[cfg(target_os = "linux")]
/// The **start** of the subordinate range delegated to `id`, or `None`.
///
/// Returning the start rather than a boolean is the point: it is the outside
/// column of the child's `uid_map`, so discarding it and mapping a constant
/// instead produces a mapping `newuidmap` refuses on every host whose delegated
/// range begins anywhere else — which is most of them.
fn subordinate_range(path: &str, id: u32) -> Option<u32> {
    let text = std::fs::read_to_string(path).ok()?;
    let login = login_name_for(id);
    text.lines().find_map(|line| {
        let mut fields = line.split(':');
        let (Some(who), Some(start), Some(count)) =
            (fields.next(), fields.next(), fields.next())
        else {
            return None;
        };
        let names_us = who == id.to_string() || login.as_deref() == Some(who);
        if !names_us || count.trim().parse::<u64>().unwrap_or(0) == 0 {
            return None;
        }
        let start: u32 = start.trim().parse().ok()?;
        // A range starting at 0 would map the child to host root and fail P2's
        // host limb; a real `/etc/subuid` never contains one, and refusing it
        // here keeps the guarantee local to the function that reads it.
        (start != 0).then_some(start)
    })
}

/// `getpwuid` without libc's `pwd.h`: `/etc/passwd` is the only source this
/// crate is allowed (no new dependencies), and a host using NSS for the
/// collector's own account will fall back to the numeric form above, which
/// `useradd` also writes.
#[cfg(target_os = "linux")]
fn login_name_for(id: u32) -> Option<String> {
    let text = std::fs::read_to_string("/etc/passwd").ok()?;
    text.lines().find_map(|line| {
        let mut fields = line.split(':');
        let name = fields.next()?;
        let _passwd = fields.next()?;
        let uid: u32 = fields.next()?.parse().ok()?;
        (uid == id).then(|| name.to_string())
    })
}

/// An executable of that name on `PATH`.
#[cfg(target_os = "linux")]
fn which(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let candidate = dir.join(program);
        is_executable(&candidate).then_some(candidate)
    })
}

#[cfg(target_os = "linux")]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// `o+x` on every directory above `path`, and `o+r` on `path` itself (plus
/// `o+x` where it is a directory or an executable). This is exactly what
/// `umask 022` buys and `umask 077` denies.
#[cfg(target_os = "linux")]
fn other_can_reach(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let mode = meta.permissions().mode();
    let leaf_ok = if meta.is_dir() {
        mode & 0o005 == 0o005
    } else {
        mode & 0o004 == 0o004
    };
    if !leaf_ok {
        return false;
    }
    let mut cursor = path.parent();
    while let Some(dir) = cursor {
        let Ok(meta) = std::fs::metadata(dir) else {
            return false;
        };
        if meta.permissions().mode() & 0o001 == 0 {
            return false;
        }
        cursor = dir.parent();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct Fake {
        namespaces: bool,
        identity: Option<IdentitySource>,
        overlay: bool,
        traverse: bool,
        netns: bool,
    }

    impl Fake {
        fn all_present() -> Self {
            Fake {
                namespaces: true,
                identity: Some(IdentitySource::RootCollectorDroppingPrivilege),
                overlay: true,
                traverse: true,
                netns: true,
            }
        }
    }

    impl HostFacts for Fake {
        fn namespaces_creatable(&self) -> bool {
            self.namespaces
        }
        fn identity_source(&self) -> Option<IdentitySource> {
            self.identity
        }
        fn overlay_root_available(&self) -> bool {
            self.overlay
        }
        fn mapped_id_can_traverse(&self) -> bool {
            self.traverse
        }
        fn network_namespace_available(&self) -> bool {
            self.netns
        }
    }

    /// PB §12's change log says "four host prerequisites" in one sentence and
    /// "the container prerequisite stack stays at five" in another; RF §7.1
    /// prints five rows and says "all five". Five wins (RF §1).
    #[test]
    fn the_prerequisite_stack_is_five_and_numbered_in_table_order() {
        assert_eq!(Prerequisite::ALL.len(), 5);
        for (i, p) in Prerequisite::ALL.iter().enumerate() {
            assert_eq!(p.number() as usize, i + 1);
        }
    }

    /// The diagnostic names *which* prerequisite, so [`check`] must report the
    /// first absence in table order rather than an arbitrary one.
    #[test]
    fn the_first_absence_in_table_order_is_the_one_reported() {
        let mut facts = Fake::all_present();
        facts.namespaces = false;
        facts.netns = false;
        assert_eq!(check(&facts), Err(Prerequisite::Namespaces));

        let mut facts = Fake::all_present();
        facts.overlay = false;
        facts.traverse = false;
        assert_eq!(check(&facts), Err(Prerequisite::OverlayRoot));
    }

    /// Each of the five, alone, is disposition 2.
    #[test]
    fn any_one_absence_denies_the_boundary() {
        type Break = fn(&mut Fake);
        let cases: [(Prerequisite, Break); 5] = [
            (Prerequisite::Namespaces, |f| f.namespaces = false),
            (Prerequisite::IdentitySource, |f| f.identity = None),
            (Prerequisite::OverlayRoot, |f| f.overlay = false),
            (Prerequisite::TraversableFilesystem, |f| f.traverse = false),
            (Prerequisite::NetworkNamespace, |f| f.netns = false),
        ];
        for (expected, break_it) in cases {
            let mut facts = Fake::all_present();
            break_it(&mut facts);
            assert_eq!(check(&facts), Err(expected));
        }
    }

    /// RF §7.1 *M1's identity source*: a bare unprivileged user namespace is
    /// **not** an arrangement — "it maps exactly one host uid, `U` itself".
    /// There are exactly two, and neither is recorded anywhere.
    #[test]
    fn there_are_exactly_two_identity_arrangements_and_both_license_container() {
        for identity in [
            IdentitySource::DelegatedSubordinateRange { first_id: 100_000 },
            IdentitySource::RootCollectorDroppingPrivilege,
        ] {
            let mut facts = Fake::all_present();
            facts.identity = Some(identity);
            assert_eq!(check(&facts), Ok(identity));
        }
    }

    /// RF §7.1: on a Darwin runner "M1 cannot be created at all; that is not a
    /// refusal but disposition 2". Prerequisite 1 is what says so — and this
    /// test runs on the host that proves it whenever that host is not Linux.
    #[test]
    #[cfg(not(target_os = "linux"))]
    fn off_linux_prerequisite_one_is_absent_and_that_is_disposition_two() {
        let host = RealHost::new(std::env::temp_dir(), vec![]);
        assert_eq!(check(&host), Err(Prerequisite::Namespaces));
    }

    /// The number the human looks up, and the words beside it.
    #[test]
    fn every_prerequisite_carries_the_words_the_diagnostic_prints() {
        for p in Prerequisite::ALL {
            assert!(!p.summary().is_empty());
            assert!((1..=5).contains(&p.number()));
        }
    }

    /// A check that cannot fail is the named hazard of this section, and
    /// prerequisite 4 was one: `all` over an empty traversal set is `true`, so
    /// a collector handed nothing to check satisfied it.
    #[test]
    fn an_empty_traversal_set_does_not_satisfy_prerequisite_four() {
        assert!(
            !traversal_is_reachable(&[], |_| true),
            "a collector handed nothing to check has not satisfied prerequisite 4"
        );
        let paths = [PathBuf::from("/a"), PathBuf::from("/b")];
        assert!(traversal_is_reachable(&paths, |_| true));
        assert!(!traversal_is_reachable(&paths, |p| p != Path::new("/b")));
    }
}
