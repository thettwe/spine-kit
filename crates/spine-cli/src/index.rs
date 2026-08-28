//! `spine index` — the derivation, and the dump that is a projection of it.
//!
//! PB §11: it "builds the traceability graph (§6.2) from in-flight branches and
//! every sealed envelope reachable from trunk".
//!
//! **There is no cache, and this says so rather than pretending.** PB §11 also
//! describes "rebuilding from scratch whenever the cache's schema or builder
//! hash differs", and `.spine/cache/graph.sqlite` is the file PB §6.7 step 6
//! deletes on upgrade. No such file is written here: `spine-graph` derives from
//! git objects and holds no persistence, so every run is a fresh derivation and
//! `--fresh` names what already happens.
//!
//! Being wrong in this direction costs time and nothing else. PB §7.4 rule 3
//! requires exactly this behaviour in CI — "The graph is rebuilt from git
//! objects, every run … no SQLite file is fetched, cached or trusted from
//! anywhere" — so the cache is an optimisation for a laptop, and a cache that
//! does not exist can never be stale, poisoned or read under the wrong schema.
//! `--fresh` is accepted and is a no-op; it is in PB §11's signature and a
//! script that passes it is asking for what it gets.

use std::io::Write;
use std::process::ExitCode;

use spine_graph::git::Repo;
use spine_graph::verify::{OpenSsh, Unverified, ssh_keygen_available};
use spine_graph::{Indexer, Options, serialize};

use crate::exit;

pub fn run(fresh: bool, dump: bool) -> ExitCode {
    let _ = fresh;

    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("spine index: {e}");
            return ExitCode::from(exit::ERROR);
        }
    };
    let repo = match Repo::open(&cwd) {
        Ok(repo) => repo,
        Err(e) => {
            eprintln!("spine index: {e}");
            return ExitCode::from(exit::ERROR);
        }
    };

    // `verify.rs` is emphatic that the fallback is "fail-closed, not neutral":
    // with no `ssh-keygen`, every seal reads unverified and every landing
    // indexes `unattested`, which PB §6.3 says is "reported and counted".
    // Saying so on stderr is what keeps that visible rather than silent — and
    // it goes to stderr because DM §2.2 gives stdout to the dump alone.
    let openssh = OpenSsh;
    let unverified = Unverified;
    let verifier: &dyn spine_graph::verify::Verifier = if ssh_keygen_available() {
        &openssh
    } else {
        eprintln!(
            "spine index: ssh-keygen is not available, so no signature verifies: \
             every landing indexes `unattested` (PB §6.3)"
        );
        &unverified
    };

    let indexed = match Indexer::new(&repo, verifier).index(&Options::default()) {
        Ok(indexed) => indexed,
        Err(e) => {
            eprintln!("spine index: {e}");
            return ExitCode::from(exit::REFUSED);
        }
    };

    if dump {
        // DM §2.2: "`spine index --dump` writes exactly these bytes to
        // **stdout** and nothing else to stdout." Written through the raw
        // handle rather than `println!`, because the bytes are bytes — a dump
        // carries `esc`-encoded paths and is not required to be UTF-8 for a
        // `String` to be the right carrier — and because a trailing newline
        // `println!` would add is already the last byte of the artifact.
        let serialized = match serialize(&indexed.header, &indexed.graph) {
            Ok(dump) => dump,
            Err(e) => {
                eprintln!("spine index: {e}");
                return ExitCode::from(exit::REFUSED);
            }
        };
        let mut stdout = std::io::stdout().lock();
        if let Err(e) = stdout
            .write_all(serialized.bytes())
            .and_then(|()| stdout.flush())
        {
            eprintln!("spine index: {e}");
            return ExitCode::from(exit::ERROR);
        }
        return ExitCode::from(exit::OK);
    }

    // Without `--dump` the graph is derived and reported. It is not written
    // anywhere: nothing in v1 reads a stored graph, `spine check` derives its
    // own, and DM §1 puts the store's extra content — in-flight intents,
    // provisional changesets, volatile results — out of the dump's scope
    // rather than into a file.
    let head = indexed
        .header
        .head
        .as_deref()
        .map(|h| &h[..12.min(h.len())])
        .unwrap_or("(no trunk)");
    eprintln!(
        "indexed {} at {head}: {} node(s), {} edge(s)",
        String::from_utf8_lossy(&indexed.header.trunk),
        indexed.graph.nodes().len(),
        indexed.graph.edges().len()
    );
    if indexed.header.trust_root.is_none() {
        eprintln!(
            "spine index: no `spine.trustRoot` is configured, so the chain walk has no root \
             (PB §7.5). Set it with `spine init --trust-root <sha>`."
        );
    }
    ExitCode::from(exit::OK)
}
