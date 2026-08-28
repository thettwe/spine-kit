//! The syscalls, declared rather than depended on.
//!
//! **No new external dependencies**: these are `extern "C"` declarations
//! against the libc every `*-unknown-linux-musl` and `*-apple-darwin` target
//! already links (CI §5.5's platform table). A crate would buy nothing here and
//! would put a third party inside the one component whose whole job is to
//! measure the host.
//!
//! RF §12 leaves *"the particular system call, helper or runtime that creates
//! M1's namespaces"* to the implementation — *"`unshare(2)` used directly, a
//! rootless OCI runtime, and a sandbox helper are all conforming if the probe of
//! §7.1 passes P1-P4 under them"*. This module picks `unshare(2)` used
//! directly, and that choice is deliberately invisible in the file: it reaches
//! no header field.
//!
//! **The freedom is over creation and not over measurement** (RF §12, as R36
//! separated them). See [`crate::netlink`].

#![cfg(unix)]

#[cfg(target_os = "linux")]
use core::ffi::{c_char, c_long, c_ulong, c_void};
use core::ffi::{c_int, c_uint};

unsafe extern "C" {
    fn getuid() -> c_uint;
    fn geteuid() -> c_uint;
    fn getgid() -> c_uint;
    fn getegid() -> c_uint;
    fn kill(pid: c_int, sig: c_int) -> c_int;
}

/// `U` — *"the collector's own real uid"* (RF §7.1). P2's exclusion set is
/// `{0, U, Ug}` and this is where `U` comes from.
pub fn getuid_real() -> u32 {
    // SAFETY: `getuid` takes no arguments, reads process credentials, and
    // cannot fail (POSIX: "shall always be successful").
    unsafe { getuid() }
}

/// `Ug` — the collector's own real gid.
pub fn getgid_real() -> u32 {
    // SAFETY: as above.
    unsafe { getgid() }
}

pub fn geteuid_eff() -> u32 {
    // SAFETY: as above.
    unsafe { geteuid() }
}

pub fn getegid_eff() -> u32 {
    // SAFETY: as above.
    unsafe { getegid() }
}

/// `SIGKILL`. RF §7.1 *The deadline*: on expiry the collector *"kills the whole
/// process group of that invocation and reaps it"* — including each of the two
/// restore phases. `SIGTERM` first would be a grace period nothing in the
/// corpus grants, and a hung restore phase *"is the case that would otherwise
/// wedge a job forever"*.
pub const SIGKILL: i32 = 9;

/// Kill a whole **process group**, which is what the deadline names. A negative
/// pid is POSIX's spelling for "the group whose id is its absolute value"; a
/// positive one would kill the shell and orphan everything it spawned, which is
/// the failure the wording exists to prevent.
pub fn kill_process_group(pgid: u32, sig: i32) -> std::io::Result<()> {
    let target = -(pgid as i32);
    // SAFETY: `kill` takes two scalars and touches no memory this process owns.
    let rc = unsafe { kill(target, sig) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

// ---------------------------------------------------------------------------
// Linux only. M1 needs kernel namespaces "and therefore exists on Linux only —
// prerequisite 1" (RF §7.1).
// ---------------------------------------------------------------------------

/// `<sys/mount.h>` flags used by the mount sequence.
pub mod ms {
    pub const RDONLY: u64 = 1;
    pub const NOSUID: u64 = 2;
    pub const NODEV: u64 = 4;
    pub const BIND: u64 = 0x1000;
    pub const REC: u64 = 0x4000;
    pub const PRIVATE: u64 = 1 << 18;
}

/// `umount2(2)`'s `MNT_DETACH`: unmount the subtree even while it is busy. The
/// teardown must leave **no probe artifact** whatever the outcome (RF §7.1
/// probe step 4), so it cannot fail merely because something still holds a
/// reference.
pub const MNT_DETACH: c_int = 2;

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn unshare(flags: c_int) -> c_int;
    fn mount(
        source: *const c_char,
        target: *const c_char,
        fstype: *const c_char,
        flags: c_ulong,
        data: *const c_void,
    ) -> c_int;
    fn umount2(target: *const c_char, flags: c_int) -> c_int;
    fn syscall(number: c_long, ...) -> c_long;
    fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    fn bind(fd: c_int, addr: *const c_void, len: c_uint) -> c_int;
    fn send(fd: c_int, buf: *const c_void, len: usize, flags: c_int) -> isize;
    fn recv(fd: c_int, buf: *mut c_void, len: usize, flags: c_int) -> isize;
    fn close(fd: c_int) -> c_int;
}

/// `pivot_root(2)` has no libc wrapper on either shipped Linux target, so it is
/// reached through `syscall(2)`. The number is per-architecture kernel ABI;
/// CI §5.5 ships exactly two Linux targets, so exactly two numbers appear here
/// and an unlisted architecture is a compile error rather than a wrong syscall.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const SYS_PIVOT_ROOT: c_long = 155;
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const SYS_PIVOT_ROOT: c_long = 41;

#[cfg(target_os = "linux")]
fn cstr(path: &std::path::Path) -> std::io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::other("a path containing NUL is not a path"))
}

#[cfg(target_os = "linux")]
fn ok(rc: c_int) -> std::io::Result<()> {
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// `unshare(2)` with the flag set of one [`crate::m1::Disposition`].
#[cfg(target_os = "linux")]
pub fn unshare_namespaces(flags: u32) -> std::io::Result<()> {
    // SAFETY: `unshare` takes one scalar. It changes this task's namespace
    // memberships and touches no memory.
    ok(unsafe { unshare(flags as c_int) })
}

/// `mount(2)`, with `data` as an already-NUL-free option string.
#[cfg(target_os = "linux")]
pub fn mount_at(
    source: &str,
    target: &std::path::Path,
    fstype: Option<&str>,
    flags: u64,
    data: Option<&str>,
) -> std::io::Result<()> {
    let source = std::ffi::CString::new(source)
        .map_err(|_| std::io::Error::other("a source containing NUL is not a source"))?;
    let target = cstr(target)?;
    let fstype = fstype
        .map(std::ffi::CString::new)
        .transpose()
        .map_err(|_| std::io::Error::other("an fstype containing NUL is not an fstype"))?;
    let data = data
        .map(std::ffi::CString::new)
        .transpose()
        .map_err(|_| std::io::Error::other("mount options containing NUL are not options"))?;
    // SAFETY: every pointer is to a `CString` that outlives the call, and the
    // two optional pointers are null exactly where the kernel accepts null.
    let rc = unsafe {
        mount(
            source.as_ptr(),
            target.as_ptr(),
            fstype.as_ref().map_or(core::ptr::null(), |f| f.as_ptr()),
            flags as c_ulong,
            data.as_ref()
                .map_or(core::ptr::null(), |d| d.as_ptr().cast::<c_void>()),
        )
    };
    ok(rc)
}

/// `umount2(2)`. Teardown only.
#[cfg(target_os = "linux")]
pub fn umount_detach(target: &std::path::Path) -> std::io::Result<()> {
    let target = cstr(target)?;
    // SAFETY: the pointer is to a `CString` that outlives the call.
    ok(unsafe { umount2(target.as_ptr(), MNT_DETACH) })
}

/// `pivot_root(2)` — the last step of M1's mount sequence (RF §7.1: *"the child
/// is `pivot_root`ed into the result"*).
#[cfg(target_os = "linux")]
pub fn pivot_root(new_root: &std::path::Path, put_old: &std::path::Path) -> std::io::Result<()> {
    let new_root = cstr(new_root)?;
    let put_old = cstr(put_old)?;
    // SAFETY: both pointers are to `CString`s that outlive the call; the
    // syscall's contract is two path arguments and no output buffer.
    let rc = unsafe { syscall(SYS_PIVOT_ROOT, new_root.as_ptr(), put_old.as_ptr()) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// `AF_NETLINK`, `SOCK_RAW`, `NETLINK_ROUTE` — Linux's numbers.
#[cfg(target_os = "linux")]
pub mod netlink_abi {
    pub const AF_NETLINK: i32 = 16;
    pub const SOCK_RAW: i32 = 3;
    pub const NETLINK_ROUTE: i32 = 0;
}

/// An `AF_NETLINK`/`NETLINK_ROUTE` socket, bound to this task's own namespace.
///
/// This is the type RF §7.1 makes normative for P4(a): *"A netlink socket is
/// answered by the namespace the calling task belongs to and **cannot be
/// pointed at another**, which is the property P4(a) needs and the only one
/// that distinguishes it from a configuration claim."*
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct NetlinkSocket {
    fd: c_int,
}

#[cfg(target_os = "linux")]
impl NetlinkSocket {
    pub fn open() -> std::io::Result<Self> {
        use netlink_abi::*;
        // SAFETY: three scalars in, a file descriptor out.
        let fd = unsafe { socket(AF_NETLINK, SOCK_RAW, NETLINK_ROUTE) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // `struct sockaddr_nl { u16 family; u16 pad; u32 pid; u32 groups; }`.
        // `pid = 0` asks the kernel to assign, which is what a single-socket
        // dump wants; a fixed pid would collide with any other netlink user in
        // the same process.
        let mut addr = [0u8; 12];
        addr[0..2].copy_from_slice(&(AF_NETLINK as u16).to_ne_bytes());
        // SAFETY: `addr` is 12 bytes, exactly `sizeof(struct sockaddr_nl)`, and
        // lives across the call.
        let rc = unsafe { bind(fd, addr.as_ptr().cast::<c_void>(), addr.len() as c_uint) };
        if rc != 0 {
            let err = std::io::Error::last_os_error();
            // SAFETY: `fd` is a descriptor this function owns and has not
            // handed out.
            unsafe { close(fd) };
            return Err(err);
        }
        Ok(NetlinkSocket { fd })
    }

    /// Send one request and read the whole dump, up to `NLMSG_DONE`.
    pub fn dump(&self, request: &[u8]) -> std::io::Result<Vec<u8>> {
        // SAFETY: `request` is a live slice for the duration of the call.
        let sent = unsafe { send(self.fd, request.as_ptr().cast::<c_void>(), request.len(), 0) };
        if sent < 0 {
            return Err(std::io::Error::last_os_error());
        }

        let mut out = Vec::new();
        let mut buf = vec![0u8; 32 * 1024];
        loop {
            // SAFETY: `buf` is a live, uniquely-borrowed allocation of the
            // length passed.
            let got = unsafe { recv(self.fd, buf.as_mut_ptr().cast::<c_void>(), buf.len(), 0) };
            if got < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if got == 0 {
                break;
            }
            let chunk = &buf[..got as usize];
            out.extend_from_slice(chunk);
            if crate::netlink::chunk_ends_the_dump(chunk) {
                break;
            }
        }
        Ok(out)
    }
}

#[cfg(target_os = "linux")]
impl Drop for NetlinkSocket {
    fn drop(&mut self) {
        // SAFETY: `self.fd` is owned by this value and closed exactly once.
        unsafe { close(self.fd) };
    }
}
