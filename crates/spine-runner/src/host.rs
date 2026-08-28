//! A [`Host`] that runs runners as ordinary children — RF §7.1's `profile=none`.
//!
//! **This is disposition 2, not a lesser boundary.** RF §7.1's profile table:
//! `none` is "**no boundary is attempted**", licensed by "nothing to test —
//! `none` asserts the *absence* of a boundary, and an absence needs no
//! evidence." It is what a host that cannot build M1 records — "not Linux
//! (`ci.md` §5.5 ships a Darwin target)" is prerequisite 1's first entry — and
//! what the solo path records by construction (RF §7.4).
//!
//! Everything here is the half M1 shares: the transport pipe, the deadline, the
//! process group, the restore phase. M1 adds namespaces, an overlay root, a
//! mapped identity and the probe, and is `spine-isolate`'s.

use std::io::Read;
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use spine_collect::collector::{Checkout, Host, Spawn, Spawned};
use spine_collect::header::Profile;

/// The environment variable the transport's fd is published in, kept here
/// rather than in the adapter because the *number* is the host's: the adapter
/// names the channel, the host holds the pipe.
pub const CHANNEL_VARIABLE: &str = "SPINE_TRANSPORT_FD";

/// Where the collector materialises the release's runner plugins.
///
/// **Outside the tree under test**, for the reason the restore script is:
/// a plugin inside the checkout is a file the candidate can rewrite between the
/// write and the exec, and RF §6.6 requires the stream to be "not supplied by
/// the candidate's environment".
pub const PLUGIN_DIRECTORY: &str = "spine-plugins";

/// The pytest transport, shipped in the binary.
///
/// RF §4.4 puts the adapter "in the pinned release", and this is the half of it
/// that is not Rust. Materialised per run rather than read from anywhere on the
/// host, so two collectors of one version run the same bytes.
pub const PYTEST_PLUGIN: &str = include_str!("../plugins/spine_pytest_transport.py");

#[derive(Debug)]
pub struct LocalHost {
    base: PathBuf,
    candidate: PathBuf,
    scratch: PathBuf,
    standing_on: Option<Checkout>,
}

impl LocalHost {
    /// `base` and `candidate` are the two checkouts RF §7.1 steps 7 and 8 run
    /// against; `scratch` is a directory outside both.
    pub fn new(base: PathBuf, candidate: PathBuf, scratch: PathBuf) -> std::io::Result<Self> {
        let plugins = scratch.join(PLUGIN_DIRECTORY);
        std::fs::create_dir_all(&plugins)?;
        std::fs::write(
            plugins.join("spine_pytest_transport.py"),
            PYTEST_PLUGIN.as_bytes(),
        )?;
        Ok(LocalHost {
            base,
            candidate,
            scratch,
            standing_on: None,
        })
    }

    fn root(&self, which: Checkout) -> &Path {
        match which {
            Checkout::Base => &self.base,
            Checkout::Candidate => &self.candidate,
        }
    }
}

impl Host for LocalHost {
    /// RF §7.1: "the achieved profile is `none`", and the collector "never
    /// silently upgrades the recorded profile".
    fn profile(&self) -> Profile {
        Profile::None
    }

    fn checkout(&mut self, which: Checkout) {
        self.standing_on = Some(which);
    }

    /// RF §7.1 *The restore phase*, delegated to `spine-isolate`, which owns
    /// the rule that it is `sh` over a file outside the checkout with stdin
    /// closed.
    fn restore(&mut self, which: Checkout, timeout_secs: u64) {
        let script = self.root(which).join(".spine/restore.sh");
        let bytes = std::fs::read(&script).ok();
        let script = match bytes {
            Some(bytes) => spine_isolate::RestoreScript::Present(bytes),
            None => spine_isolate::RestoreScript::Absent,
        };
        // The two crates each name the checkout in their own vocabulary and
        // neither depends on the other; this is the one place they meet.
        let isolate_side = match which {
            Checkout::Base => spine_isolate::Checkout::Base,
            Checkout::Candidate => spine_isolate::Checkout::Tree,
        };
        if let Some(line) = script.diagnostic(isolate_side) {
            eprintln!("{line}");
        }
        let _ = spine_isolate::m1::run_restore(
            &script,
            self.root(which),
            &self.scratch,
            Duration::from_secs(timeout_secs),
        );
    }

    /// RF §7.1 step 9: "Reap every process group."
    ///
    /// Every spawn below waits for its own child before returning — the
    /// deadline is enforced per invocation, not per run — so by the time this
    /// is called there is nothing outstanding. It is kept because RF §9 turns
    /// the step into an ordering guarantee the collector relies on, and a host
    /// that later spawns concurrently must have somewhere to honour it.
    fn reap_all(&mut self) {}

    fn spawn(&mut self, spec: &Spawn<'_>) -> Spawned {
        if self.standing_on != Some(spec.checkout) {
            // The collector checks out before it spawns (RF §7.1 steps 7, 8),
            // and a spawn against the other tree would be the interleaving
            // rule 3 forbids — "no candidate can make a landed test
            // uncollectable" — so it is refused rather than run.
            return Spawned::SpawnFailed;
        }
        let Some((program, arguments)) = spec.argv.split_first() else {
            return Spawned::SpawnFailed;
        };

        // RF §6.6: "read over a pipe **the collector holds**". The child gets
        // the write end and is told its number; the collector keeps the read
        // end and never lets the runner's stdout near it.
        let (read, write) = match pipe() {
            Ok(pair) => pair,
            Err(_) => return Spawned::SpawnFailed,
        };

        let mut command = Command::new(program);
        command
            .args(arguments)
            .current_dir(self.root(spec.checkout))
            // IR §11.1: "Every invocation runs at the **repository root**".
            .stdin(Stdio::null())
            // ci.md §5.1 puts every diagnostic on stderr; a runner's own
            // stdout is not the transport and is not read.
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());

        for (name, value) in spec.env {
            if *name == CHANNEL_VARIABLE {
                command.env(name, write.as_raw_fd().to_string());
            } else {
                command.env(name, value);
            }
        }
        // The plugin directory the constructor materialised, prepended so the
        // candidate's own `PYTHONPATH` cannot shadow the transport.
        let plugins = self.scratch.join(PLUGIN_DIRECTORY);
        let inherited = std::env::var("PYTHONPATH").unwrap_or_default();
        command.env(
            "PYTHONPATH",
            match inherited.is_empty() {
                true => plugins.display().to_string(),
                false => format!("{}:{inherited}", plugins.display()),
            },
        );

        let child = unsafe {
            use std::os::unix::process::CommandExt;
            let raw = write.as_raw_fd();
            command
                // The write end must survive `exec`, or the plugin has nothing
                // to open. Everything else the collector holds must not.
                .pre_exec(move || {
                    clear_cloexec(raw)?;
                    Ok(())
                })
                // Its own group, so the deadline can kill the group rather
                // than the leader: a runner that backgrounds a worker leaves
                // the worker running otherwise.
                .process_group(0)
                .spawn()
        };
        let mut child = match child {
            Ok(child) => child,
            Err(_) => return Spawned::SpawnFailed,
        };
        // The collector's copy of the write end goes, or the read below never
        // sees EOF.
        drop(write);

        read_until_exit(&mut child, read, Duration::from_secs(spec.timeout_secs))
    }
}

/// Read the transport while waiting for the child, enforcing the deadline.
fn read_until_exit(child: &mut std::process::Child, read: OwnedFd, deadline: Duration) -> Spawned {
    // The reader runs on its own thread: a runner that fills the pipe blocks
    // until someone drains it, and a collector that waited for exit first would
    // deadlock on exactly the runs that produce the most output.
    let reader = std::thread::spawn(move || {
        let mut file = std::fs::File::from(read);
        let mut bytes = Vec::new();
        let _ = file.read_to_end(&mut bytes);
        bytes
    });

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(_) => break None,
        }
        if started.elapsed() >= deadline {
            // "on expiry the collector kills its process group and reaps it".
            kill_group(child.id());
            let _ = child.wait();
            return Spawned::TimedOut;
        }
        std::thread::sleep(Duration::from_millis(10));
    };

    let bytes = reader.join().unwrap_or_default();
    // RF §7.3: `complete` requires "that no member of its process group was
    // terminated by a signal", and the exit **code** is never consulted.
    let signalled = status.is_some_and(|status| {
        use std::os::unix::process::ExitStatusExt;
        status.signal().is_some()
    });
    Spawned::Stream { bytes, signalled }
}

fn pipe() -> std::io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0i32; 2];
    // SAFETY: `pipe(2)` writes exactly two descriptors into the array.
    let rc = unsafe { libc_pipe(fds.as_mut_ptr()) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: both are fresh descriptors this process now owns.
    unsafe { Ok((OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1]))) }
}

unsafe extern "C" {
    #[link_name = "pipe"]
    fn libc_pipe(fds: *mut i32) -> i32;
    fn fcntl(fd: i32, cmd: i32, ...) -> i32;
    fn kill(pid: i32, sig: i32) -> i32;
}

const F_SETFD: i32 = 2;
const SIGKILL: i32 = 9;

/// Rust sets `FD_CLOEXEC` on every descriptor it creates, so the write end
/// would close on `exec` and the plugin would find nothing to write to.
fn clear_cloexec(fd: i32) -> std::io::Result<()> {
    // SAFETY: a variadic `fcntl` with an integer argument, on a descriptor
    // this process owns.
    if unsafe { fcntl(fd, F_SETFD, 0) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// "kills the whole process group of that invocation and reaps it" — the
/// negative pid is the group, which is why the child was given one.
fn kill_group(pid: u32) {
    // SAFETY: a signal to a process group this collector created.
    unsafe {
        kill(-(pid as i32), SIGKILL);
    }
}
