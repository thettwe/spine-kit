//! The Linux half of M1 — the calls that actually create the boundary, and the
//! measurements P2, P3 and P4 take from inside it.
//!
//! Nothing here decides anything. Every function returns an observation for
//! [`crate::probe`]'s deciders, or performs one step of
//! [`crate::m1::mount_sequence`]. That is the seam described in the crate doc,
//! and it is why the verdicts are tested on a host with no namespaces at all.

use crate::m1::{Disposition, MountStep, RootSpec, overlay_options};
use crate::netlink;
use crate::prereq::IdentitySource;
use crate::probe::{DevIno, Enumeration, P3Report, P4Report, attempt_egress};
use crate::sys;
use std::io;
use std::path::Path;

/// DERIVED. RF §7.1's identity arrangement 2 says the root collector *"maps the
/// child to a **fixed unprivileged id**"* and never says which. It must lie
/// outside `{0, U, Ug}` for P2, and under this arrangement `U` and `Ug` are 0.
///
/// `65534` (`nobody`) is refused deliberately: on most hosts it is a real
/// account, and a file owned by `nobody` is also what an *unmapped* id looks
/// like through an idmapped mount — so P2 would pass on a mapping that had in
/// fact collapsed. `100000` is the first id of the conventional subordinate
/// range and belongs to nothing.
pub const FIXED_UNPRIVILEGED_ID: u32 = 100_000;

/// `unshare(2)` this task into one disposition's namespace set.
///
/// The user namespace must come with the rest: RF §7.1's identity source is
/// what supplies the mapping afterwards, and *"an unprivileged user namespace
/// with no delegated subordinate range … maps exactly one host uid, `U`
/// itself"*, which P2 fails on every host forever.
pub fn unshare_for(disposition: Disposition) -> io::Result<()> {
    sys::unshare_namespaces(disposition.namespaces())
}

/// Write a child's `uid_map`/`gid_map` from RF §7.1's arrangement 2 — the root
/// collector. Arrangement 1 is [`map_via_helpers`].
///
/// `setgroups` must be denied before `gid_map` may be written by anything but a
/// fully privileged process; writing `deny` unconditionally costs a root
/// collector nothing and is what makes the same code path work under both
/// arrangements' kernels. DERIVED — the corpus names the arrangements, not the
/// kernel's write protocol.
pub fn map_as_root_collector(pid: u32, child_id: u32) -> io::Result<()> {
    let _ = std::fs::write(format!("/proc/{pid}/setgroups"), b"deny");
    // A map line is `<id-inside-ns> <id-outside-ns> <count>`, and BOTH columns
    // are load-bearing here because P2 has two limbs that read different ones:
    // the probe reports its **inside** id, and the collector `stat`s the
    // probe's file to see its **outside** id. P2 passes only if neither is `0`
    // or `U`, and for a root collector `U` is `0`.
    //
    // So the identity column must be `child_id` on **both** sides. Writing
    // `"{child_id} 0 1"` — inside `child_id`, outside 0 — makes the host see
    // every file the child creates owned by root, and P2's host limb then fails
    // on every host, for every configuration, forever. That is the failure
    // shape RF §13 R36 exists to eliminate, reintroduced through a column
    // order.
    //
    // The first line maps the collector's own root through unchanged so the
    // child begins with a valid identity and the capabilities it needs to
    // `setuid` down; it then drops to `child_id`, which is what "a root
    // collector that **drops privilege**" means (RF §7.1, arrangement 2).
    //
    // **That line is also a door, and closing it is [`sys::drop_privileges_to`]'s
    // job, not the map's.** Inside-uid 0 stays mapped to host uid 0, so a child
    // that can `setuid(0)` is host root — and **P2 cannot see it**: P2 reads
    // the ids *after* the drop, so such a child still reports non-zero ids and
    // still creates a file the host sees owned by `child_id`. The drop must
    // therefore be irreversible before `exec`, with all three of real,
    // effective and saved set, and it is verified rather than assumed. A
    // caller that writes this map and does not drop leaves a boundary that
    // measures as `container` and is not one.
    let map = format!("0 0 1\n{child_id} {child_id} 1\n");
    std::fs::write(format!("/proc/{pid}/uid_map"), &map)?;
    std::fs::write(format!("/proc/{pid}/gid_map"), &map)?;
    Ok(())
}

/// RF §7.1's arrangement 1: *"the host … supplies `newuidmap`/`newgidmap` to
/// write the child's `uid_map`/`gid_map`"*. The helpers are setuid and must be
/// run from **outside** the child, which is why this takes the child's pid.
///
/// `newuidmap <pid> <inside> <outside> <count>`.
///
/// **Both columns must be non-zero.** The child sees `inside_id` and the host
/// sees `first_subordinate_id`, and P2 excludes `0` and `U` from *each* — the
/// probe reports the inside id and the collector `stat`s for the outside one.
/// Mapping inside `0` makes the probe report uid 0, which P2 refuses outright,
/// however unprivileged the host-side id is.
///
/// `first_subordinate_id` is the start of the range the host actually delegated
/// to this collector, read from `/etc/subuid`. A subordinate range never
/// contains `U` by construction, which is what makes the host limb pass.
pub fn map_via_helpers(
    pid: u32,
    inside_id: u32,
    first_subordinate_id: u32,
    count: u32,
) -> io::Result<()> {
    for helper in ["newuidmap", "newgidmap"] {
        let status = std::process::Command::new(helper)
            .arg(pid.to_string())
            .arg(inside_id.to_string())
            .arg(first_subordinate_id.to_string())
            .arg(count.to_string())
            .status()?;
        if !status.success() {
            return Err(io::Error::other(format!("{helper} refused the mapping")));
        }
    }
    Ok(())
}

/// Apply whichever arrangement the host supplied. *"Which of the two a host
/// supplies never reaches the file"* — so this returns `()` and not the
/// arrangement it used.
pub fn install_identity(pid: u32, source: IdentitySource) -> io::Result<()> {
    match source {
        IdentitySource::RootCollectorDroppingPrivilege => {
            map_as_root_collector(pid, FIXED_UNPRIVILEGED_ID)
        }
        // One id is enough for M1: the child runs as exactly one id. A wider
        // range would be needed only by a nested user namespace, which M1 does
        // not create.
        IdentitySource::DelegatedSubordinateRange { first_id } => {
            // Inside: a fixed non-zero id. Outside: the range the host actually
            // delegated — not a constant. A hard-coded outside id is a mapping
            // `newuidmap` refuses whenever the host delegated a different range,
            // which is most hosts.
            map_via_helpers(pid, FIXED_UNPRIVILEGED_ID, first_id, 1)
        }
    }
}

/// Perform [`crate::m1::mount_sequence`], step by step, inside the mount
/// namespace this task has already unshared.
pub fn apply(spec: &RootSpec) -> io::Result<()> {
    use sys::ms;
    for step in crate::m1::mount_sequence(spec) {
        match step {
            MountStep::MakeRootPrivate => {
                sys::mount_at("none", Path::new("/"), None, ms::REC | ms::PRIVATE, None)?
            }
            MountStep::TmpfsForEmptyLowerLayer { at } => {
                std::fs::create_dir_all(&at)?;
                // `nosuid,nodev` on every tmpfs M1 mounts: the child is an
                // unprivileged id and has no business finding a setuid binary
                // or a device node on a filesystem the collector made for it.
                // `size=4k` and not `size=0`: in `shmem`, `max_blocks == 0`
                // means *no limit*, so `size=0k` would ask for the opposite of
                // what it reads like. Nothing is ever written here — the layer
                // is required to be **empty**, because "an overlay's lower
                // layers are searched in order and a non-empty second layer
                // would put files in the child's root that the job's root does
                // not have".
                sys::mount_at(
                    "tmpfs",
                    &at,
                    Some("tmpfs"),
                    ms::NOSUID | ms::NODEV | ms::RDONLY,
                    Some("size=4k,mode=0555"),
                )?;
            }
            MountStep::OverlayRoot { lower, at } => {
                std::fs::create_dir_all(&at)?;
                let options = overlay_options(&lower.0, &lower.1)
                    .map_err(|e| io::Error::other(e.to_string()))?;
                sys::mount_at("overlay", &at, Some("overlay"), 0, Some(&options))?;
            }
            MountStep::BindWritableTree { from, to } => {
                std::fs::create_dir_all(&to)?;
                sys::mount_at(
                    from.to_str()
                        .ok_or_else(|| io::Error::other("writable tree path is not UTF-8"))?,
                    &to,
                    None,
                    ms::BIND | ms::REC,
                    None,
                )?;
            }
            MountStep::TmpfsScratch { at } => {
                std::fs::create_dir_all(&at)?;
                sys::mount_at("tmpfs", &at, Some("tmpfs"), ms::NOSUID | ms::NODEV, None)?;
            }
            MountStep::MaskResultDirectory { at } => {
                std::fs::create_dir_all(&at)?;
                // An empty, read-only tmpfs over the subpath: `.spine/cache/` is
                // "absent from that view", and P1(c) and P1(d) then measure the
                // arrangement rather than trust it.
                sys::mount_at(
                    "tmpfs",
                    &at,
                    Some("tmpfs"),
                    ms::RDONLY | ms::NOSUID | ms::NODEV,
                    Some("size=4k,mode=0555"),
                )?;
            }
            MountStep::ProcFs { at } => {
                std::fs::create_dir_all(&at)?;
                // **A fresh mount, never a bind.** `m1.rs`'s own note is the
                // reason: "`procfs` shows the PID namespace of the task that
                // mounted it, so binding the job's `/proc` would show the
                // job's pids and P3's limb would find the collector's own —
                // inverting the test rather than emptying it."
                //
                // `nosuid,nodev,noexec` for the same reason every other mount
                // here carries them: the child is an unprivileged id and has
                // no business finding a setuid binary under `/proc`.
                sys::mount_at(
                    "proc",
                    &at,
                    Some("proc"),
                    ms::NOSUID | ms::NODEV | ms::NOEXEC,
                    None,
                )?;
            }
            MountStep::PivotRoot { new_root, put_old } => {
                std::fs::create_dir_all(&put_old)?;
                sys::pivot_root(&new_root, &put_old)?;
                std::env::set_current_dir("/")?;
                // The old root must go, or the child holds a path back to the
                // job's own filesystem and P1 measures a boundary that has a
                // door in it. `MNT_DETACH` because the child's cwd may still be
                // resolving through it.
                // `put_old` was named on the host side, beneath `new_root`;
                // after the pivot it is reachable at that same path relative to
                // the new `/`. Deriving it by `strip_prefix` rather than by
                // `file_name` keeps a nested `put_old` correct.
                let relative = put_old
                    .strip_prefix(&new_root)
                    .map_err(|_| io::Error::other("put_old must live beneath new_root"))?;
                let put_old_inside = Path::new("/").join(relative);
                sys::umount_detach(&put_old_inside)?;
                let _ = std::fs::remove_dir(&put_old_inside);
            }
        }
    }
    Ok(())
}

/// Prerequisite 5's second half: *"a network namespace … with a loopback device
/// it can bring up"*. Run inside the fresh network namespace, before the probe.
pub fn bring_up_loopback() -> io::Result<()> {
    let socket = sys::NetlinkSocket::open()?;
    let dump = socket.dump(&netlink::get_link_request(1))?;
    let links = netlink::parse_links(&dump).map_err(|e| io::Error::other(e.to_string()))?;
    let lo = links
        .iter()
        .find(|l| l.is_loopback())
        .ok_or_else(|| io::Error::other("the fresh namespace holds no loopback device"))?;
    if lo.is_up() {
        return Ok(());
    }
    // The kernel answers a `NLM_F_ACK`ed request with an `NLMSG_ERROR` whose
    // errno is 0 on success. Parsing the reply rather than discarding it is what
    // turns "we sent the request" into "loopback is up" — prerequisite 5 is
    // about the second, and P4(a) would otherwise be measuring a namespace whose
    // `lo` the collector only *asked* to raise.
    let ack = socket.dump(&netlink::set_link_up_request(lo.index, 2))?;
    netlink::parse_links(&ack).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(())
}

/// P3's report. Both limbs are the probe's own; see [`crate::probe::decide_p3`]
/// for what that costs.
///
/// The table is read from `procfs`, which — unlike `sysfs` — *is* re-derived
/// per PID namespace, so a `/proc` mounted inside the boundary answers for the
/// boundary. RF §7.1 makes the same point for the net files.
pub fn measure_separation() -> io::Result<P3Report> {
    let mut pids = Vec::new();
    for entry in std::fs::read_dir("/proc")? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str()
            && let Ok(pid) = name.parse::<u32>()
        {
            pids.push(pid);
        }
    }
    Ok(P3Report {
        pids,
        own_pid: std::process::id(),
        root: DevIno::of(Path::new("/"))?,
    })
}

/// P4's report — **both** limbs, because *"the two limbs check each other and
/// neither alone would do"*.
///
/// (a) is read over netlink and never over `sysfs`, which is normative
/// (RF §7.1, §12 as narrowed by §13 R36): *"`unshare(2)` does not re-derive an
/// already-mounted `sysfs`, so `/sys/class/net` inside a correctly isolated
/// namespace continues to list the job's own interfaces."*
pub fn measure_egress() -> P4Report {
    let (links, addrs) = match sys::NetlinkSocket::open() {
        Err(e) => {
            let why = format!("netlink socket: {e}");
            (
                Enumeration::Unavailable(why.clone()),
                Enumeration::Unavailable(why),
            )
        }
        Ok(socket) => (
            dump(&socket, netlink::get_link_request(1), netlink::parse_links),
            dump(&socket, netlink::get_addr_request(2), netlink::parse_addrs),
        ),
    };
    P4Report {
        links,
        addrs,
        connect: attempt_egress(),
    }
}

fn dump<T>(
    socket: &sys::NetlinkSocket,
    request: Vec<u8>,
    parse: fn(&[u8]) -> Result<Vec<T>, netlink::NetlinkError>,
) -> Enumeration<T> {
    match socket.dump(&request).map_err(|e| e.to_string()) {
        Err(why) => Enumeration::Unavailable(why),
        Ok(bytes) => match parse(&bytes) {
            Ok(items) => Enumeration::Read(items),
            Err(e) => Enumeration::Unavailable(e.to_string()),
        },
    }
}
