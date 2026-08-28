//! `spine index` — the derivation.
//!
//! PB §6.1 fixes what this module is allowed to be: *"Spine-kit's job is not to
//! ask anyone to draw a graph; it is to **extract the graph that already
//! exists** in the artifacts."* Everything below reads git objects and produces
//! [`Node`]s and [`Edge`]s; nothing takes a hint from a human, a config file, a
//! PR description or a cache.
//!
//! **What it derives is fixed by two tables and one rule.** PB §6.2's
//! derivation table says where each element comes from; DM §8.2 and §8.3 say
//! which of them a *dump* may carry; and DM §8.1's generating rule decides
//! anything the tables left open:
//!
//! > *"A graph element is in the dump if and only if it is derived from git
//! > objects reachable from the trunk tip. An element derived from anything
//! > else — an intent branch, the collector's result file, a coverage report,
//! > the binary's own floor list, or a heuristic over the objects rather than
//! > the objects — is excluded."*
//!
//! This module builds **the projection**, not the store. DM §1 draws the line:
//! a store built for `spine check` holds in-flight intents, provisional
//! changesets, volatile results and the shipped floor; a dump holds only what a
//! fresh clone of trunk can rederive. Since [`crate::dump::serialize`] refuses
//! a non-projection, deriving one directly is the honest shape — and it is why
//! nothing here reads `refs/heads/intent/*`, `refs/notes/*`, the working tree,
//! the result file, or the release's floor list.
//!
//! **What it does not derive** is listed in the crate's report and repeated at
//! each site: `verified_by` edges and the pragma scan that seeds them (they
//! need IR §12's pragma grammar and the runner id→path map, which live in
//! `spine-resolve`), `implements` for in-flight branches (excluded from a dump
//! anyway), and the SQLite cache (PB §6.2's `.spine/cache/graph.sqlite` needs a
//! binding this crate may not add — PB §6.7 step 6 makes it disposable:
//! *"Schema migration is nothing: `spine index` rebuilds under the new
//! schema."*).

use crate::dump::Header;
use crate::schema::{Attrs, EdgeKind, NodeKind, Src, id, tool_version_from_seal};
use crate::status::{Refusal, Status};
use crate::store::{Edge, Graph, Node};
use crate::{git, verify::Verifier};
use spine_canon::sha256_prefixed;
use spine_manifest::Manifest;
use spine_manifest::keyring::Keyring;

/// What can stop a derivation.
///
/// Two arms because two things can go wrong and only one of them is a dump's
/// business: DM §4.4's refusals describe *"something this format cannot
/// represent"*, while a git failure is the repository refusing to be read at
/// all and has no token in that table.
#[derive(Debug)]
pub enum Error {
    Git(git::GitError),
    Refused(Refusal),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Git(e) => write!(f, "{e}"),
            Error::Refused(r) => write!(f, "{r}"),
        }
    }
}

impl core::error::Error for Error {}

impl From<git::GitError> for Error {
    fn from(e: git::GitError) -> Self {
        Error::Git(e)
    }
}

impl From<Refusal> for Error {
    fn from(r: Refusal) -> Self {
        Error::Refused(r)
    }
}

type Result<T> = core::result::Result<T, Error>;

/// The two inputs a caller may supply that are not in the repository.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// DM §4.2 step 1: *"an explicit `--trunk <name>`, when the CLI offers
    /// one"*. Bytes, because a ref name is bytes.
    pub trunk: Option<Vec<u8>>,
    /// The runner's pinned trust root, overriding `spine.trustRoot`.
    ///
    /// DM §11 step 3 writes it into **both** sides of a G10 comparison, and DM
    /// §3.1 records it in the header, *"since a side without a pin would trust
    /// on first use or refuse, either way diverging from the other on every
    /// landing (TOFU is for humans, never for G10)"*.
    pub trust_root: Option<String>,
}

/// A derived graph and the header that describes what produced it.
#[derive(Debug, Clone)]
pub struct Indexed {
    pub header: Header,
    pub graph: Graph,
}

/// One landing, as read from its envelope, before any status is decided.
///
/// The derivation is two passes because three intent statuses are facts about
/// *other* landings: `superseded` is named by a later envelope's
/// `Spine-Supersedes`, `reverted` by a later landing's patch id, and both are
/// unknowable while the landing that carries them has not been read. PB §6.2
/// makes `status` *"derived, never read from the file"*, and a derivation that
/// emitted a node before it knew the answer would have to mutate it afterwards
/// — which [`Graph::add_node`]'s DM §5.5 collapse deliberately cannot do.
struct Landing {
    sha: String,
    envelope: Envelope,
    /// The seal's payload fields, already split.
    seal: Payload,
    seal_verified: bool,
    /// This subset of PB §6.3's G9 that a serializer can compute: the seal's
    /// signature and the `envelope=` digest. See [`Indexer::read_landing`].
    unattested: bool,
    /// The intent doc, when the landing carries a fenced block.
    doc: Option<IntentDoc>,
    /// Paths of `git diff --name-only base..L`, which `reverts` restricts to.
    paths: Vec<Vec<u8>>,
}

impl Landing {
    fn event(&self) -> &str {
        self.envelope
            .trailer("Spine-Event")
            .map(|t| t.payload_str())
            .unwrap_or_default()
    }

    fn is_land(&self) -> bool {
        self.event() == "land"
    }

    /// The bare intent id (`INT-042`), from `Spine-Intent`. Absent on a quick,
    /// lifecycle or reseal landing, which PB §11 says *"have no intent id, and
    /// take their identity from the seal's first field"*.
    fn intent_id(&self) -> Option<String> {
        self.envelope
            .trailer("Spine-Intent")
            .map(|t| t.payload_str().to_string())
            .filter(|id| !id.is_empty())
    }
}

/// The derivation, over one repository.
pub struct Indexer<'a> {
    repo: &'a git::Repo,
    verifier: &'a dyn Verifier,
}

impl core::fmt::Debug for Indexer<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // A `dyn Verifier` has no `Debug`, and giving the trait one would put a
        // formatting requirement on every implementation for the sake of a
        // derive.
        f.debug_struct("Indexer").field("repo", &self.repo).finish_non_exhaustive()
    }
}

impl<'a> Indexer<'a> {
    pub fn new(repo: &'a git::Repo, verifier: &'a dyn Verifier) -> Self {
        Indexer { repo, verifier }
    }

    /// Build the graph a dump is a projection of.
    ///
    /// DM §4.1: *"A dump is a function of exactly four things: the trunk tip's
    /// oid, the git objects reachable from it, the trust root, and the pinned
    /// release. Nothing else may influence one byte."* The four appear here as
    /// the resolved head, every `cat-file` below it, [`Options::trust_root`],
    /// and this code.
    pub fn index(&self, options: &Options) -> Result<Indexed> {
        let format = self.repo.object_format();
        let (repo_name, trunk) = self.resolve_trunk(options)?;
        let trunk_ref = format!("refs/heads/{}", String::from_utf8_lossy(&trunk));
        let head = self.repo.rev_parse(&trunk_ref);
        let trust_root = options
            .trust_root
            .clone()
            .or_else(|| self.repo.config("spine.trustRoot"));

        let header = Header {
            object_format: format,
            repo: repo_name.clone(),
            trunk: trunk.clone(),
            head: head.clone(),
            trust_root: trust_root.clone(),
        };

        let mut graph = Graph::new();
        // DM §9 case 2: "a manifest resolves but `refs/heads/<trunk>` does not
        // … the walk has no tip, the derivation produces nothing, and the dump
        // is [the header alone]". Legal, exit 0, not an error.
        let Some(head) = head else {
            return Ok(Indexed { header, graph });
        };

        // Oldest first. Every rule below that says "the first landing at which"
        // is a rule about this order, and reversing it would silently move
        // every constitution and signer citation.
        let mut walk = self.repo.first_parent(&head)?;
        if let Some(root) = &trust_root
            && let Some(at) = walk.iter().position(|sha| sha == root)
        {
            // DM §8.2: a changeset "below the trust root" is excluded. A trust
            // root that is not on the walk is a broken chain, which PB §7.5
            // makes G13's to hard-fail — not this module's to hide by dumping
            // nothing, so the walk is left whole in that case.
            walk.truncate(at + 1);
        }
        walk.reverse();

        self.derive_signers(&repo_name, &walk, &mut graph)?;

        let mut landings = Vec::new();
        for sha in &walk {
            if let Some(landing) = self.read_landing(sha)? {
                landings.push(landing);
            }
        }

        self.derive_constitutions(&repo_name, &landings, &mut graph)?;
        self.derive_adrs(&repo_name, &head, &mut graph)?;

        // Computed once and used twice: `reverted` is a status *and* an edge,
        // and the two must agree by construction rather than by both running
        // the same O(n²) patch-id pipeline over the same landings.
        let reverts = self.revert_pairs(&landings)?;
        let statuses = statuses(&landings, &reverts);
        let resealed = self.resealed(&landings)?;
        for landing in &landings {
            self.emit_landing(&repo_name, landing, &statuses, &resealed, &mut graph)?;
        }
        derive_reverts(&repo_name, &landings, &reverts, &mut graph);

        Ok(Indexed { header, graph })
    }

    // -----------------------------------------------------------------
    // DM §4.2 — trunk resolution
    // -----------------------------------------------------------------

    /// The manifest's `repo` and the resolved trunk **branch name**.
    ///
    /// DM §4.2's order, steps 2–4 (step 1 is [`Options::trunk`], and it still
    /// needs a manifest for `repo`, which every node id is prefixed with):
    ///
    /// > 2. `params.trunk` in `.spine/manifest.json` in the tree of the commit
    /// >    `HEAD` resolves to;
    /// > 3. `params.trunk` in the manifest of the newest first-parent ancestor
    /// >    of `HEAD` whose tree carries one — this is the case for the range
    /// >    between an `--uninstall` landing and the next `init`;
    /// > 4. none: the repository is not a spine repository, and `--dump`
    /// >    refuses.
    ///
    /// *"Steps 2 and 3 read a **tree**, never the working directory, so a bare
    /// repository resolves identically to a checked-out one."*
    fn resolve_trunk(&self, options: &Options) -> Result<(String, Vec<u8>)> {
        let not_installed = || {
            Error::Refused(Refusal::new(
                Status::NotInstalled,
                "no manifest on HEAD or any first-parent ancestor (DM §4.2 step 4)",
            ))
        };
        let head = self.repo.rev_parse("HEAD").ok_or_else(not_installed)?;

        // The walk is HEAD first, so step 2 is step 3's first iteration and the
        // two need no separate code path.
        for sha in self.repo.first_parent(&head)? {
            let Some(bytes) = self.repo.blob_at(&sha, ".spine/manifest.json") else {
                continue;
            };
            let Ok(manifest) = Manifest::parse(&bytes, Some(self.repo.object_format())) else {
                // A manifest that does not parse is not a manifest. Skipping it
                // continues the search rather than refusing, which is what
                // makes step 3's uninstall range work at all: the range's tip
                // carries no usable manifest and its history still is a ledger.
                continue;
            };
            let trunk = options
                .trunk
                .clone()
                .unwrap_or_else(|| manifest.trunk().as_bytes().to_vec());
            return Ok((manifest.repo().to_string(), trunk));
        }
        Err(not_installed())
    }

    // -----------------------------------------------------------------
    // PB §6.2 — signer nodes
    // -----------------------------------------------------------------

    /// `signer` nodes, from *"`.spine/allowed_signers` at every trunk
    /// first-parent commit from the trust root, with `valid_from`/`valid_to`
    /// from the chain walk (§7.5)"* (PB §6.2).
    ///
    /// The identity is the **fingerprint**, not the principal, because MF §4.6
    /// says so: *"A line edited in place (same principal, new key) is a removal
    /// and an addition: the old fingerprint gets a `valid_to`, the new one a
    /// `valid_from`."* The node *id* is still the principal (DM §5.2), which is
    /// why MF §4.5 refuses a keyring listing two keys under one principal —
    /// *"they would be two signer nodes with one id"*.
    ///
    /// **Both are commits, not times** (MF §4.6): PB §7.5's *"the chain, not
    /// timestamps, is the authority"*, which is also DM §10 rule 1 — no wall
    /// clock reaches a dump.
    fn derive_signers(&self, repo: &str, walk: &[String], graph: &mut Graph) -> Result<()> {
        /// A key's life, while the walk is still running.
        struct Life {
            principal: String,
            namespaces: Vec<String>,
            line_no: usize,
            valid_from: String,
            valid_to: Option<String>,
        }
        let mut lives: Vec<Life> = Vec::new();

        for sha in walk {
            let bytes = self.repo.blob_at(sha, ".spine/allowed_signers");
            let entries = match &bytes {
                Some(bytes) => Keyring::parse(bytes).entries,
                None => Vec::new(),
            };
            for entry in &entries {
                match lives
                    .iter_mut()
                    .find(|l| l.principal == entry.principal && l.valid_to.is_none())
                {
                    Some(_) => {}
                    None => lives.push(Life {
                        principal: entry.principal.clone(),
                        // The namespaces as of the line that `src` cites. A
                        // later re-listing under more namespaces is a new line
                        // and would be a new fingerprint only if the key
                        // changed; MF §4.6 gives no rule for the same key
                        // gaining a namespace, so the first appearance is
                        // taken. **DERIVED.**
                        namespaces: entry.namespaces.clone(),
                        line_no: entry.line_no,
                        valid_from: sha.clone(),
                        valid_to: None,
                    }),
                }
            }
            for life in &mut lives {
                if life.valid_to.is_none() && !entries.iter().any(|e| e.principal == life.principal)
                {
                    life.valid_to = Some(sha.clone());
                }
            }
        }

        for life in lives {
            let mut attrs = Attrs::new()
                .arr("roles", life.namespaces)
                .str("valid_from", &life.valid_from);
            // DM §12.3 check 6: "`signer.valid_to` is absent, not null".
            if let Some(valid_to) = life.valid_to {
                attrs = attrs.str("valid_to", valid_to);
            }
            // The fingerprint is recomputed at the citing commit rather than
            // carried, so the attr and the citation cannot disagree.
            let Some(bytes) = self.repo.blob_at(&life.valid_from, ".spine/allowed_signers") else {
                continue;
            };
            let keyring = Keyring::parse(&bytes);
            let Some(entry) = keyring.entries.iter().find(|e| e.principal == life.principal) else {
                continue;
            };
            graph.add_node(Node::new(
                NodeKind::Signer,
                id::signer(repo, life.principal.as_bytes()),
                attrs.str("fingerprint", &entry.fingerprint),
                Src::FileLineAt {
                    sha: life.valid_from.clone(),
                    path: b".spine/allowed_signers".to_vec(),
                    line: life.line_no as u64,
                },
            ));
        }
        Ok(())
    }

    // -----------------------------------------------------------------
    // CN §9.6 — constitution nodes and the `C-A2` limb of `protects`
    // -----------------------------------------------------------------

    /// One `constitution` node per distinct version observed on the walk, and
    /// one `protects` edge per `effective(C-A2)` entry.
    ///
    /// CN §9.6: *"Id `<repo>/constitution:v<n>`, kind `constitution`, `attrs
    /// {}`, `src` `git:<sha>:<esc(path)>:2` — line 2, the header (§3.1).
    /// `<sha>` is the landing that introduced the version."* CN §3.1 fixes the
    /// header at line 2 *"not because that example happened to put it there"*,
    /// which is why the 2 below is a constant and not a search.
    ///
    /// **Only landings are consulted.** DM §12.1's trust root carries
    /// `CONSTITUTION.md` at v3 and the published dump cites `git:<L>:…`, not
    /// `git:<T0>:…`: the trust root is not a landing, so the first landing at
    /// which a version is observed is the one that introduced it.
    ///
    /// The shipped half of `protects` is **not** here, and its absence is the
    /// rule: DM §8.5 clause 2 excludes it because *"including it would make the
    /// dump a function of the release, which §3.4 forbids"*, and because the
    /// release has no node kind to hang it from. Every dumped `protects`
    /// therefore carries `floor: false`.
    fn derive_constitutions(
        &self,
        repo: &str,
        landings: &[Landing],
        graph: &mut Graph,
    ) -> Result<()> {
        let mut seen: Vec<u64> = Vec::new();
        for landing in landings {
            let path = self.constitution_path(&landing.sha);
            let Some(bytes) = self.repo.blob_at(&landing.sha, &path) else {
                continue;
            };
            let Some(version) = constitution_version(&bytes) else {
                continue;
            };
            if seen.contains(&version) {
                continue;
            }
            seen.push(version);
            let node = id::constitution(repo, version);
            graph.add_node(Node::new(
                NodeKind::Constitution,
                node.clone(),
                Attrs::new(),
                Src::FileLineAt {
                    sha: landing.sha.clone(),
                    path: path.as_bytes().to_vec(),
                    line: 2,
                },
            ));
            for (pattern, line) in effective_c_a2(&bytes) {
                let src = Src::FileLineAt {
                    sha: landing.sha.clone(),
                    path: path.as_bytes().to_vec(),
                    // CN §9.6: "**Every pattern on the line shares the line's
                    // number**", which is ID §6.6's rule for a touchpoint list
                    // "and is the only available answer when several patterns
                    // share one line".
                    line,
                };
                let unit = id::code_unit(repo, &pattern);
                graph.add_node(Node::new(
                    NodeKind::CodeUnit,
                    unit.clone(),
                    Attrs::new(),
                    src.clone(),
                ));
                graph.add_edge(Edge::new(
                    EdgeKind::Protects,
                    node.clone(),
                    unit,
                    Attrs::new().bool("floor", false),
                    src,
                ));
            }
        }
        Ok(())
    }

    /// The constitution's path at a commit: the manifest's `paths.constitution`
    /// (PB §6.7), or `CONSTITUTION.md` where the manifest does not name one.
    fn constitution_path(&self, sha: &str) -> String {
        let default = "CONSTITUTION.md".to_string();
        let Some(bytes) = self.repo.blob_at(sha, ".spine/manifest.json") else {
            return default;
        };
        let Ok(manifest) = Manifest::parse(&bytes, Some(self.repo.object_format())) else {
            return default;
        };
        manifest
            .value()
            .get("paths")
            .and_then(|paths| paths.get("constitution"))
            .and_then(spine_canon::Value::as_str)
            .map(str::to_string)
            .unwrap_or(default)
    }

    // -----------------------------------------------------------------
    // DM §8.2 — adr nodes
    // -----------------------------------------------------------------

    /// `adr` nodes: *"an ADR file present in `adr/` in the trunk tip's tree (PB
    /// §2.2 makes the folder append-only)"* (DM §8.2), cited at line 1 — the
    /// heading the id is read out of.
    ///
    /// **DERIVED: the id is the heading's leading id token.** DM §5.2 says only
    /// *"the ADR's own id, as its heading spells it"*, and no document gives
    /// the heading a grammar. The run of `[A-Za-z0-9-]` after any leading `#`
    /// and spaces is taken, which reads `ADR-007` out of
    /// `# ADR-007: Tax rounding`. A file whose first line yields nothing is not
    /// a node — an id that is not in the id grammar would refuse the whole dump
    /// at `check_node_id`, and one ADR with an unconventional heading must not
    /// cost the repository its ledger.
    fn derive_adrs(&self, repo: &str, tip: &str, graph: &mut Graph) -> Result<()> {
        let entries = match self.repo.ls_tree(tip, "adr/") {
            Ok(entries) => entries,
            // No `adr/` directory: `ls-tree` succeeds with nothing, but a
            // repository without the path is not an error either way.
            Err(_) => return Ok(()),
        };
        for entry in entries {
            if entry.kind != "blob" {
                continue;
            }
            let bytes = self.repo.blob(&entry.oid)?;
            let first = bytes.split(|&b| b == b'\n').next().unwrap_or_default();
            let Some(adr_id) = heading_id(first) else {
                continue;
            };
            graph.add_node(Node::new(
                NodeKind::Adr,
                id::adr(repo, &adr_id),
                Attrs::new(),
                Src::FileLineAt {
                    sha: tip.to_string(),
                    path: entry.path.clone(),
                    line: 1,
                },
            ));
        }
        Ok(())
    }

    // -----------------------------------------------------------------
    // PB §5.5 — reading one landing
    // -----------------------------------------------------------------

    /// Read a first-parent commit as a landing, or `None` when it is not one.
    ///
    /// *"A `Spine-Seal` trailer marks a landing `L`"* (PB §5.5). A first-parent
    /// commit that is neither a landing nor the trust root is an **orphan** —
    /// *"a push around the pipeline"* — and contributes no changeset node,
    /// which is DM §8.2's rule stated from the other side: a changeset is in
    /// the dump when *"a first-parent trunk commit carrying `Spine-Seal`"*.
    fn read_landing(&self, sha: &str) -> Result<Option<Landing>> {
        let message = self.repo.commit_message(sha)?;
        let envelope = Envelope::read(&message);
        let Some(seal_line) = envelope.trailer("Spine-Seal") else {
            return Ok(None);
        };
        let seal = Payload::parse(&seal_line.payload);
        let seal_raw = seal_line.raw.clone();

        // PB §5.5: signatures are verified "against `.spine/allowed_signers`
        // **as it existed at the seal's `base=`** — a landing can never admit
        // its own signer."
        let base = seal.get_str("base").unwrap_or_default();
        let keyring = self
            .repo
            .blob_at(&base, ".spine/allowed_signers")
            .unwrap_or_default();
        let principal = seal.get_str("signer").unwrap_or_default();
        let seal_verified = envelope
            .trailer("Spine-Seal-Sig")
            .and_then(|sig| {
                self.verifier.namespace_that_verifies(
                    &keyring,
                    &principal,
                    &seal_raw,
                    sig.payload_str(),
                )
            })
            .is_some();

        // **The subset of G9 a serializer can compute.** PB §6.3's G9 is a
        // whole first-parent walk with a dozen clauses and it belongs to the
        // gates crate; two of its clauses are functions of this commit's own
        // bytes and are computed here so that `unattested` is not simply
        // `false` for want of a checker: the seal's signature, and PB §5.5's
        // *"the seal's `envelope=` digest — SHA-256 over every `Spine-*` line
        // above it, in order"*. **Fail-closed:** either failing marks the
        // landing `unattested`, which PB §6.3 calls "reported and counted".
        let envelope_ok = seal
            .get_str("envelope")
            .is_some_and(|declared| declared == envelope.digest_above_seal());
        let unattested = !(seal_verified && envelope_ok);

        let doc = envelope.fenced.as_ref().and_then(|f| IntentDoc::parse(&f.body));
        let paths = match seal.get_str("base") {
            Some(base) => self.repo.diff_names(&base, sha)?,
            None => Vec::new(),
        };

        Ok(Some(Landing {
            sha: sha.to_string(),
            envelope,
            seal,
            seal_verified,
            unattested,
            doc,
            paths,
        }))
    }

    // -----------------------------------------------------------------
    // PB §6.6 — the post-landing lifecycle, and the reseal range
    // -----------------------------------------------------------------

    /// Every landing a later reseal's range covers.
    ///
    /// PB §5.5: a reseal's *"seal `base=` the last valid landing below the
    /// range and `head=O`"*, and *"Resealed commits index as `unattested`
    /// members of the reseal changeset"*. A landing can be inside such a range
    /// — *"an `unattested` reseal is an orphan like any other and the next
    /// reseal covers it as one"* — and that is what `changeset.resealed`
    /// records. **DERIVED** only in that DM §7.2 lists the attr among the seal
    /// fields while no seal field carries it; it is computed from the range
    /// rather than left constant.
    fn resealed(&self, landings: &[Landing]) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for landing in landings.iter().filter(|l| l.event() == "reseal") {
            let (Some(base), Some(head)) =
                (landing.seal.get_str("base"), landing.seal.get_str("head"))
            else {
                continue;
            };
            out.extend(self.repo.rev_list_range(&base, &head)?);
        }
        Ok(out)
    }

    /// Pairs `(reverted intent id, reverting landing sha)` under PB §6.2's rule.
    ///
    /// > *"a landing `R` later than `L` on first-parent, with a non-empty diff,
    /// > whose `git diff R^ R -- <L's paths> | git patch-id --stable` equals
    /// > `git diff L L^ | git patch-id --stable` — restricted to `L`'s paths,
    /// > so the `BUG-` reproduction test `R` also lands does not disqualify it
    /// > … only `Spine-Event: land` commits participate"*
    ///
    /// The restriction is what makes the rule usable, and it is also this
    /// implementation's one silent edge: a path whose bytes are not valid UTF-8
    /// cannot be passed in `argv`, so it is dropped from the restriction rather
    /// than widening it (see [`git::Repo::patch_id`]). **DERIVED**, and it can
    /// only ever *miss* a revert, never invent one.
    fn revert_pairs(&self, landings: &[Landing]) -> Result<Vec<(String, String)>> {
        let mut out = Vec::new();
        let lands: Vec<&Landing> = landings.iter().filter(|l| l.is_land()).collect();
        for (i, l) in lands.iter().enumerate() {
            if l.paths.is_empty() {
                continue;
            }
            let parents = self.repo.parents(&l.sha)?;
            let Some(first_parent) = parents.first() else {
                continue;
            };
            // `git diff L L^` — the *reverse* of the landing's own diff, which
            // is what a revert of it produces.
            let Some(forward) = self.repo.patch_id(&l.sha, first_parent, &[])? else {
                continue;
            };
            for r in &lands[i + 1..] {
                let r_parents = self.repo.parents(&r.sha)?;
                let Some(r_parent) = r_parents.first() else {
                    continue;
                };
                let Some(candidate) = self.repo.patch_id(r_parent, &r.sha, &l.paths)? else {
                    continue;
                };
                if candidate == forward
                    && let Some(intent) = l.intent_id()
                {
                    out.push((intent, r.sha.clone()));
                }
            }
        }
        Ok(out)
    }


    // -----------------------------------------------------------------
    // The landing's own elements
    // -----------------------------------------------------------------

    fn emit_landing(
        &self,
        repo: &str,
        landing: &Landing,
        statuses: &[(String, &'static str)],
        resealed: &[String],
        graph: &mut Graph,
    ) -> Result<()> {
        let sha = &landing.sha;
        let cs = id::changeset(repo, sha);
        let seal_src = Src::Trailer {
            sha: sha.clone(),
            name: "Spine-Seal".into(),
        };

        // --- the landing changeset, from the seal's fields (DM §7.2) --------
        let seal = &landing.seal;
        let strategy = seal_field(&landing.envelope, "Spine-Strategy");
        let mut attrs = Attrs::new()
            .bool("landing", true)
            .str("lane", seal_field(&landing.envelope, "Spine-Lane"))
            .str("event", landing.event())
            .str("strategy", &strategy)
            .bool("seal_verified", landing.seal_verified)
            .bool("unattested", landing.unattested)
            // No seal field carries this; it is a fact about a *range*
            // ([`Indexer::resealed`]).
            .bool("resealed", resealed.contains(sha));
        for (attr, field) in [
            ("base", "base"),
            ("head", "head"),
            ("report_sha256", "report"),
            ("threat", "threat"),
            ("profile", "profile"),
            ("git_version", "git"),
            ("mode", "mode"),
        ] {
            if let Some(value) = seal.get_str(field) {
                attrs = attrs.str(attr, value);
            }
        }
        if let Some(tool) = seal.get_str("tool") {
            attrs = attrs.str("tool_version", tool_version_from_seal(&tool));
        }
        if let Some(principal) = seal.get(b"signer") {
            attrs = attrs.bytes("seal_principal", &principal);
        }
        // DM §7.2.1: `tree` is "`L`'s tree oid — normally the seal's `tree=`" —
        // except under squash, where "`H` is unreachable by design and the tree
        // rule is never consulted", which PB §6.3's G9 records as a sentinel.
        // The second sentinel, `unverifiable(git-version)`, is G9's to raise
        // and is **NOT IMPLEMENTED** here: it requires recomputing
        // `merge-tree(B, H)` and comparing git versions, which is G9's walk.
        let tree = if strategy == "squash" {
            "unverifiable(squash)".to_string()
        } else {
            seal.get_str("tree").unwrap_or_default()
        };
        if !tree.is_empty() {
            attrs = attrs.str("tree", tree);
        }
        graph.add_node(Node::new(
            NodeKind::Changeset,
            cs.clone(),
            attrs,
            seal_src.clone(),
        ));

        // --- attested_by: the landing → the seal's signer -------------------
        if let Some(principal) = seal.get(b"signer") {
            graph.add_edge(Edge::new(
                EdgeKind::AttestedBy,
                cs.clone(),
                id::signer(repo, &principal),
                Attrs::new(),
                seal_src.clone(),
            ));
        }

        // --- the intent, its ACs and its touchpoints ------------------------
        let intent_node = landing.intent_id().map(|local| id::intent(repo, &local));
        if let (Some(intent_node), Some(doc), Some(fenced)) = (
            intent_node.as_ref(),
            landing.doc.as_ref(),
            landing.envelope.fenced.as_ref(),
        ) {
            // A line of the fenced block, as a message line: `git:<L>:msg:L<n>`
            // (DM §5.4), `n` counted over the whole commit message.
            let msg = |offset: u64| Src::MessageLine {
                sha: sha.clone(),
                line: fenced.first_line + offset - 1,
            };
            let status = statuses
                .iter()
                .find(|(id, _)| Some(id.as_str()) == landing.intent_id().as_deref())
                .map(|(_, s)| *s)
                .unwrap_or("merged");

            let signoff = landing.envelope.trailer("Spine-Signoff").map(|t| Payload::parse(&t.payload));
            let approve = landing.envelope.trailer("Spine-Approve").map(|t| Payload::parse(&t.payload));
            let reopens: Vec<Payload> = landing
                .envelope
                .trailers_named("Spine-Reopen")
                .iter()
                .map(|t| Payload::parse(&t.payload))
                .collect();
            // "of those, the ones after the binding approval" (DM §7.2). A
            // reopen after the approval is exactly the one that voids it: PB
            // §6's table calls a reopen the event that "voids the freeze digest
            // it names", so a copied reopen whose `voids=` is the copied
            // approve's `freeze=` is late and one naming `none` or an older
            // digest is not. **DERIVED**, and it is the only rule available:
            // EV §2.4 emits every reopen at rank 7, above the approve line, so
            // position in the envelope cannot separate them.
            let late = approve
                .as_ref()
                .and_then(|a| a.get_str("freeze"))
                .map(|freeze| {
                    reopens
                        .iter()
                        .filter(|r| r.get_str("voids").as_deref() == Some(freeze.as_str()))
                        .count()
                })
                .unwrap_or(0);

            let mut attrs = Attrs::new()
                .str("status", status)
                .bytes("title", &doc.title)
                .str("template", &doc.template)
                .str("landing", sha)
                .int("reopen_count", reopens.len() as u64)
                .int("late_reopen_count", late as u64);
            if let Some(owner) = &doc.owner {
                attrs = attrs.bytes("owner", owner);
            }
            if let Some(base) = seal.get_str("base") {
                attrs = attrs.str("base", base);
            }
            // "the signed intent blob": the fence names it and the sign-off
            // binds it (PB §5.5, EV §2.6).
            attrs = attrs.str("blob", &fenced.blob);
            if let Some(signer) = signoff.as_ref().and_then(|s| s.get(b"signer")) {
                attrs = attrs.bytes("signer", &signer);
            }
            graph.add_node(Node::new(
                NodeKind::Intent,
                intent_node.clone(),
                attrs,
                msg(doc.title_line),
            ));

            for (n, line) in &doc.acs {
                let ac = id::ac(repo, &doc.id, *n);
                graph.add_node(Node::new(NodeKind::Ac, ac.clone(), Attrs::new(), msg(*line)));
                graph.add_edge(Edge::new(
                    EdgeKind::HasAc,
                    intent_node.clone(),
                    ac,
                    Attrs::new(),
                    msg(*line),
                ));
            }

            if let Some(version) = doc.constitution {
                graph.add_edge(Edge::new(
                    EdgeKind::BuiltUnder,
                    intent_node.clone(),
                    id::constitution(repo, version),
                    Attrs::new(),
                    msg(doc.header_line),
                ));
            }

            // ID §6.6: one `code_unit` per distinct pattern, "written as
            // declared, never expanded", and the citation is the touchpoint
            // **label line's**, "since several patterns share one line".
            for (pattern, polarity, line) in &doc.touchpoints {
                let unit = id::code_unit(repo, pattern);
                graph.add_node(Node::new(
                    NodeKind::CodeUnit,
                    unit.clone(),
                    Attrs::new(),
                    msg(*line),
                ));
                graph.add_edge(Edge::new(
                    EdgeKind::Declares,
                    intent_node.clone(),
                    unit,
                    Attrs::new().str("polarity", *polarity),
                    msg(*line),
                ));
            }

            if let Some(target) = landing.envelope.trailer("Spine-Supersedes") {
                graph.add_supersession(
                    intent_node,
                    &id::intent(repo, target.payload_str()),
                    Src::Trailer {
                        sha: sha.clone(),
                        name: "Spine-Supersedes".into(),
                    },
                );
            }
        }

        self.emit_approvals(repo, landing, intent_node.as_deref(), graph)?;
        self.emit_changesets(repo, landing, intent_node.as_deref(), graph)?;
        Ok(())
    }

    /// `approval` nodes and their `approves` / `signed_by` edges.
    ///
    /// PB §6.2 derives them from *"`Spine-Signoff`, `Spine-Approve`,
    /// `Spine-Review`, `Spine-Reopen`, `Spine-Withdraw`, `Spine-Upgrade` lines
    /// with verifying `-Sig` … copied into the envelope once landed"*, and
    /// `approves` *"names the intent for every line carrying an id and the
    /// landing changeset `cs:<L>` for those that do not"*.
    fn emit_approvals(
        &self,
        repo: &str,
        landing: &Landing,
        intent_node: Option<&str>,
        graph: &mut Graph,
    ) -> Result<()> {
        let sha = &landing.sha;
        let base = landing.seal.get_str("base").unwrap_or_default();
        let keyring = self
            .repo
            .blob_at(&base, ".spine/allowed_signers")
            .unwrap_or_default();
        let cs = id::changeset(repo, sha);
        let approve_freeze = landing
            .envelope
            .trailer("Spine-Approve")
            .and_then(|t| Payload::parse(&t.payload).get_str("freeze"));

        for (name, event) in STATEMENT_TRAILERS {
            for line in landing.envelope.trailers_named(name) {
                let payload = Payload::parse(&line.payload);
                let src = Src::Trailer {
                    sha: sha.clone(),
                    name: name.to_string(),
                };
                let principal = payload
                    .get(b"signer")
                    .or_else(|| payload.get(b"reviewer"))
                    .unwrap_or_default();
                // "**the namespace the signature verified under**, never a
                // claim in the trailer" (DM §7.2). Asking which namespace
                // verifies is therefore the derivation, not a shortcut: a v1
                // approve line signed under `spine-review@v1` is `reviewer`,
                // and the trailer's name would have said `signer`.
                let signature = landing
                    .envelope
                    .trailer(&format!("{name}-Sig"))
                    .map(|s| s.payload_str().to_string());
                let namespace = signature.as_ref().and_then(|sig| {
                    self.verifier.namespace_that_verifies(
                        &keyring,
                        &String::from_utf8_lossy(&principal),
                        &line.raw,
                        sig,
                    )
                });
                let verified = namespace.is_some();
                // **DERIVED, fail-closed.** DM §7.2 makes `role` mandatory and
                // defines it only for a signature that verified. Where none
                // does, the role PB §11 *requires* of that trailer is recorded
                // beside `verified: false`, so the pair reads "claimed this,
                // proved nothing" rather than dropping the element.
                let role = namespace
                    .as_deref()
                    .map(role_of_namespace)
                    .unwrap_or_else(|| declared_role(name));

                let mut attrs = Attrs::new()
                    .str("event", event)
                    .str("role", role)
                    .bytes("principal", &principal)
                    .bool("verified", verified);
                if let Some(blob) = payload
                    .get_str("blob")
                    .or_else(|| payload.get_str("intent"))
                {
                    attrs = attrs.str("blob", blob);
                }
                for field in ["base", "head", "tree"] {
                    if let Some(value) = payload.get_str(field) {
                        attrs = attrs.str(field, value);
                    }
                }
                if event == "review"
                    && let Some(class) = payload.get_str("class")
                {
                    attrs = attrs.str("class", class);
                }
                for field in ["rounds", "total_rounds", "reopens"] {
                    if let Some(n) = payload.get_str(field).and_then(|v| v.parse::<u64>().ok()) {
                        attrs = attrs.int(field, n);
                    }
                }
                if event == "approve" {
                    for field in ["red", "freeze"] {
                        if let Some(value) = payload.get_str(field) {
                            attrs = attrs.str(field, value);
                        }
                    }
                }
                if event == "review"
                    && let Some(wires) = payload.get_str("wires")
                {
                    // "**in the line's order**, which PB §11 fixes as ascending
                    // by unsigned byte value over the whole token (so `G11`
                    // precedes `G2`) … Not re-sorted here: the signed line's
                    // order is the fact, and a dump that re-sorted it would
                    // hide a non-conforming review rather than reproduce it."
                    attrs = attrs.arr("wires", wires.split(',').filter(|w| !w.is_empty()));
                }
                // A copied reopen voids the approval whose freeze it names
                // (PB §6, the reopen row).
                if event == "approve"
                    && let Some(freeze) = &approve_freeze
                {
                    for reopen in landing.envelope.trailers_named("Spine-Reopen") {
                        let r = Payload::parse(&reopen.payload);
                        if r.get_str("voids").as_deref() == Some(freeze.as_str()) {
                            // "the commit carrying that reopen": the event
                            // commit whose message holds this byte-identical
                            // line, when it is still reachable, and otherwise
                            // the landing that copied it. Under squash the
                            // event commits are unreachable by design (PB
                            // §5.5), so the fallback is not an error case.
                            let carrier = self
                                .carrier_of(landing, &reopen.raw)?
                                .unwrap_or_else(|| sha.clone());
                            attrs = attrs.str("voided_by", carrier);
                            if let Some(reason) = r.get(b"reason") {
                                attrs = attrs.bytes("void_reason", &reason);
                            }
                        }
                    }
                }

                let node = id::approval(repo, &line.raw);
                graph.add_node(Node::new(
                    NodeKind::Approval,
                    node.clone(),
                    attrs,
                    src.clone(),
                ));

                // The id-carrying lines name the intent; the others name the
                // landing changeset (PB §6.2).
                let target = match (payload.first_field_id(), intent_node) {
                    (Some(_), Some(intent)) => intent.to_string(),
                    _ => cs.clone(),
                };
                graph.add_edge(Edge::new(
                    EdgeKind::Approves,
                    node.clone(),
                    target,
                    Attrs::new(),
                    src.clone(),
                ));
                if !principal.is_empty() {
                    graph.add_edge(Edge::new(
                        EdgeKind::SignedBy,
                        node.clone(),
                        id::signer(repo, &principal),
                        Attrs::new(),
                        src.clone(),
                    ));
                }
                if event == "approve" {
                    self.emit_freezes(repo, landing, &node, graph)?;
                }
            }
        }
        Ok(())
    }

    /// The member commit whose message carries `line`, ancestor-first.
    fn carrier_of(&self, landing: &Landing, line: &[u8]) -> Result<Option<String>> {
        let Some(base) = landing.seal.get_str("base") else {
            return Ok(None);
        };
        let mut members = self.repo.rev_list_range(&base, &landing.sha)?;
        members.retain(|m| m != &landing.sha);
        members.reverse();
        for member in members {
            let message = self.repo.commit_message(&member)?;
            if message
                .split(|&b| b == b'\n')
                .any(|candidate| candidate == line)
            {
                return Ok(Some(member));
            }
        }
        Ok(None)
    }

    /// `freezes` edges, from *"`Spine-Frozen` (→ `code_unit`, with the blob)
    /// and `Spine-Test` (→ `test`) lines of the binding approval (§4.3)"*
    /// (PB §6.2).
    ///
    /// Where those lines live is the strategy's business, and both places are
    /// cited as themselves: under **squash** they are copied into the envelope
    /// (PB §11 confines them there) and cited `git:<L>:trailer:…`; under
    /// **merge** the approval commit is reachable — `Spine-Approval` names it —
    /// so they are read there and cited `git:<approval>:trailer:…`, which is
    /// what DM §12.2 publishes.
    fn emit_freezes(
        &self,
        repo: &str,
        landing: &Landing,
        approval: &str,
        graph: &mut Graph,
    ) -> Result<()> {
        let (sha, envelope);
        let source = match landing.envelope.trailer("Spine-Approval") {
            Some(pointer) if landing.envelope.trailer("Spine-Frozen").is_none() => {
                sha = pointer.payload_str().to_string();
                let Ok(message) = self.repo.commit_message(&sha) else {
                    // The approval commit is gone (garbage-collected after a
                    // squash whose envelope also lacks the lines). Nothing to
                    // derive, and nothing to invent.
                    return Ok(());
                };
                envelope = Envelope::read(&message);
                &envelope
            }
            _ => {
                sha = landing.sha.clone();
                &landing.envelope
            }
        };

        for line in source.trailers_named("Spine-Frozen") {
            let Some((oid, path)) = parse_frozen(&line.payload) else {
                continue;
            };
            let src = Src::Trailer {
                sha: sha.clone(),
                name: "Spine-Frozen".into(),
            };
            let unit = id::code_unit(repo, &path);
            graph.add_node(Node::new(
                NodeKind::CodeUnit,
                unit.clone(),
                Attrs::new(),
                src.clone(),
            ));
            graph.add_edge(Edge::new(
                EdgeKind::Freezes,
                approval.to_string(),
                unit,
                Attrs::new().str("oid", oid),
                src,
            ));
        }

        for line in source.trailers_named("Spine-Test") {
            // EV §4.4: "`<runner> <runner-native function id>` … the split at
            // the **first** space is exact even though a function id may itself
            // contain spaces (vitest's `>`-joined names do)".
            let Some(space) = line.payload.iter().position(|&b| b == b' ') else {
                continue;
            };
            let runner = String::from_utf8_lossy(&line.payload[..space]).into_owned();
            let native = &line.payload[space + 1..];
            let src = Src::Trailer {
                sha: sha.clone(),
                name: "Spine-Test".into(),
            };
            let test = id::test(repo, &runner, native);
            // **DERIVED citation.** PB §6.2 cites a landed test node
            // `git:<L>:<path>:<line>` — "the frozen blob, reachable through
            // `L`'s tree forever". Reaching that line means mapping a
            // runner-native id to its file and its definition line, which is
            // the per-runner id grammar of IR §11 and lives in `spine-resolve`;
            // this crate may not depend on it, and reimplementing it is
            // precisely the divergence PB §6.7 warns about — "two resolvers
            // differing on one edge case reject each other's approvals". The
            // line that *does* name this node is the `Spine-Test` line, so that
            // is what it cites, and the finer citation is reported as missing
            // rather than guessed.
            graph.add_node(Node::new(
                NodeKind::Test,
                test.clone(),
                Attrs::new(),
                src.clone(),
            ));
            // "A `freezes` edge to a `test` carries `{}` — PB §6.2 says so."
            graph.add_edge(Edge::new(
                EdgeKind::Freezes,
                approval.to_string(),
                test,
                Attrs::new(),
                src,
            ));
        }
        Ok(())
    }

    /// The landing's members, their `implements` edges, and every `modifies`.
    ///
    /// PB §5.5: *"its members are `M(L) = git rev-list B..L`, `B` being the
    /// seal's `base=` … merge strategy: `L` plus every branch commit not
    /// already on trunk … squash: `{L}`"*, and *"membership comes from the
    /// landing range, never from a trailer on a branch commit, because a branch
    /// commit can claim anything."*
    fn emit_changesets(
        &self,
        repo: &str,
        landing: &Landing,
        intent_node: Option<&str>,
        graph: &mut Graph,
    ) -> Result<()> {
        let sha = &landing.sha;
        let cs = id::changeset(repo, sha);
        let seal_src = Src::Trailer {
            sha: sha.clone(),
            name: "Spine-Seal".into(),
        };
        // DM §7.2: `implements.provisional` is "`false` in every dumped record"
        // — a provisional edge is an in-flight changeset's, and §8.2 excluded
        // the changeset. `verified` is "membership verified by G9's walk", of
        // which the seal's own signature is the part this module computes.
        let role = |role: &str| {
            Attrs::new()
                .str("role", role)
                .bool("provisional", false)
                .bool("verified", landing.seal_verified)
        };

        if let Some(intent) = intent_node {
            graph.add_edge(Edge::new(
                EdgeKind::Implements,
                cs.clone(),
                intent.to_string(),
                role("landing"),
                seal_src.clone(),
            ));
        }

        // `modifies`: "`git diff --name-only B L` — the integrated delta G2
        // gates on; per-member diffs for archaeology" (PB §6.2).
        let emit_modifies = |from: &str, sha: &str, paths: &[Vec<u8>], graph: &mut Graph| {
            for path in paths {
                let unit = id::code_unit(repo, path);
                let src = Src::Commit {
                    sha: sha.to_string(),
                };
                graph.add_node(Node::new(
                    NodeKind::CodeUnit,
                    unit.clone(),
                    Attrs::new(),
                    src.clone(),
                ));
                graph.add_edge(Edge::new(
                    EdgeKind::Modifies,
                    from.to_string(),
                    unit,
                    Attrs::new(),
                    src,
                ));
            }
        };
        emit_modifies(&cs, sha, &landing.paths, graph);

        let Some(base) = landing.seal.get_str("base") else {
            return Ok(());
        };
        let mut members = self.repo.rev_list_range(&base, sha)?;
        members.retain(|m| m != sha);
        for member in members {
            let member_cs = id::changeset(repo, &member);
            graph.add_node(Node::new(
                NodeKind::Changeset,
                member_cs.clone(),
                // "A member changeset carries `{"landing":false}` and nothing
                // else: it has no seal, and every one of those fields is a seal
                // field." (DM §7.2)
                Attrs::new().bool("landing", false),
                Src::Commit {
                    sha: member.clone(),
                },
            ));
            if let Some(intent) = intent_node {
                graph.add_edge(Edge::new(
                    EdgeKind::Implements,
                    member_cs.clone(),
                    intent.to_string(),
                    role("member"),
                    seal_src.clone(),
                ));
            }
            let parents = self.repo.parents(&member)?;
            if let Some(parent) = parents.first() {
                let paths = self.repo.diff_names(parent, &member)?;
                emit_modifies(&member_cs, &member, &paths, graph);
            }
        }
        Ok(())
    }
}

/// Every landed intent's `status`, decided before any node is emitted.
///
/// DM §7.3 closes the domain at `merged`, `withdrawn`, `reverted` and
/// `superseded`: *"`orphan`, `unattested` and `resealed` are properties of a
/// **changeset** … a landing can be `unattested` while its intent is plainly
/// `merged`."*
///
/// PB §6.6's governing sentence is the order of the rules below: *"A revert is
/// detected, never declared; a supersession is sealed, never asserted; a
/// withdrawal is landed, never deleted."*
fn statuses(landings: &[Landing], reverts: &[(String, String)]) -> Vec<(String, &'static str)> {
    let mut out: Vec<(String, &'static str)> = Vec::new();
    for landing in landings {
        let Some(intent) = landing.intent_id() else {
            continue;
        };
        // A tombstone "records an abandoned intent on trunk: no code, the
        // signed doc, the reason … the id is retired" (PB §6.6).
        let status = if landing.event() == "withdraw" {
            "withdrawn"
        } else {
            "merged"
        };
        out.push((intent, status));
    }
    // "a later intent whose `Supersedes:` header names this one lands with a
    // `Spine-Supersedes` trailer" (PB §6.6).
    for landing in landings {
        let Some(target) = landing.envelope.trailer("Spine-Supersedes") else {
            continue;
        };
        let target = target.payload_str().to_string();
        if let Some(row) = out.iter_mut().find(|(id, _)| *id == target) {
            row.1 = "superseded";
        }
    }
    // Decided by the same patch-id rule the `reverts` edge is emitted from, so
    // the status and the edge cannot disagree.
    for (reverted, _reverting) in reverts {
        if let Some(row) = out.iter_mut().find(|(id, _)| id == reverted) {
            // PB §6's table has one row back out — "reverted | that revert
            // itself fully reverted | merged" — which needs a third landing to
            // reach and is **NOT IMPLEMENTED**.
            row.1 = "reverted";
        }
    }
    out
}

/// `reverts` edges, changeset → changeset, cited `git:<R>:patch-id`.
///
/// `partial` is `false` on every emitted edge. PB §6.2's partial case —
/// *"missing hunks inside `L`'s paths → `{partial: true}` and a warning"* — is
/// not detectable from a patch id, which is equal or not; detecting it needs a
/// hunk-level comparison this module does not do. **NOT IMPLEMENTED**, and
/// reported: a partial reversal yields no edge rather than a wrong one.
fn derive_reverts(
    repo: &str,
    landings: &[Landing],
    reverts: &[(String, String)],
    graph: &mut Graph,
) {
    for (intent, reverting) in reverts {
        let Some(reverted) = landings
            .iter()
            .find(|l| l.intent_id().as_deref() == Some(intent.as_str()))
        else {
            continue;
        };
        graph.add_edge(Edge::new(
            EdgeKind::Reverts,
            id::changeset(repo, reverting),
            id::changeset(repo, &reverted.sha),
            Attrs::new().bool("partial", false),
            Src::PatchId {
                sha: reverting.clone(),
            },
        ));
    }
}

/// The six statement trailers PB §6.2 derives an `approval` node from, and the
/// `event` each produces (DM §7.2's closed domain).
const STATEMENT_TRAILERS: [(&str, &str); 6] = [
    ("Spine-Signoff", "signoff"),
    ("Spine-Approve", "approve"),
    ("Spine-Review", "review"),
    ("Spine-Reopen", "reopen"),
    ("Spine-Withdraw", "withdraw"),
    ("Spine-Upgrade", "upgrade"),
];

/// PB §11: *"signer `spine-signoff@v1` · reviewer `spine-review@v1` · pipeline
/// `spine-seal@v1`"* — the map DM §7.2's `role` is the image of.
fn role_of_namespace(namespace: &str) -> &'static str {
    match namespace {
        "spine-seal@v1" => "pipeline",
        "spine-review@v1" => "reviewer",
        _ => "signer",
    }
}

/// The role PB §11 requires of a trailer, used only where no signature verified.
fn declared_role(trailer: &str) -> &'static str {
    match trailer {
        "Spine-Approve" | "Spine-Review" => "reviewer",
        _ => "signer",
    }
}

fn seal_field(envelope: &Envelope, name: &str) -> String {
    envelope
        .trailer(name)
        .map(|t| t.payload_str().to_string())
        .unwrap_or_default()
}

/// `# ADR-007: Tax rounding` → `ADR-007`. See [`Indexer::derive_adrs`].
fn heading_id(line: &[u8]) -> Option<String> {
    let start = line
        .iter()
        .position(|&b| b != b'#' && b != b' ' && b != b'\t')?;
    let rest = &line[start..];
    let end = rest
        .iter()
        .position(|&b| !(b.is_ascii_alphanumeric() || b == b'-'))
        .unwrap_or(rest.len());
    let id = String::from_utf8(rest[..end].to_vec()).ok()?;
    (!id.is_empty()).then_some(id)
}

/// CN §9.1: line 2 is the header, and `Version:` is its first field.
fn constitution_version(bytes: &[u8]) -> Option<u64> {
    let line = bytes.split(|&b| b == b'\n').nth(1)?;
    for field in String::from_utf8_lossy(line).split(" · ") {
        let (name, value) = field.split_once(": ")?;
        // "Names are matched **ASCII-case-insensitively** (`version:` parses),
        // values are not." (CN §9.1)
        if name.eq_ignore_ascii_case("version") {
            return value.strip_prefix('v')?.parse().ok();
        }
    }
    None
}

/// `effective(C-A2)`'s patterns and the line each was declared on.
///
/// CN §3.2 test 2: a rule line is one *"whose first two bytes are `0x43 0x2D` —
/// `C-`, exact case"*, at byte 0. CN §4.5: *"split the body at its first
/// `0x3D`"*, and CN §5.5 splits the value on `,` and strips spaces and tabs.
///
/// **DERIVED where the file is silent:** a repository with no `C-A2` line gets
/// no `protects` edge, rather than CN §7.2's fail-closed default `["**"]`. The
/// default is what a *gate* uses while it refuses (CN §7.4); making it a
/// `protects` edge would put a `code_unit` node for `**` in the ledger of every
/// repository whose constitution is missing or malformed, and a dump records
/// what the objects say.
fn effective_c_a2(bytes: &[u8]) -> Vec<(Vec<u8>, u64)> {
    let mut out = Vec::new();
    for (i, line) in bytes.split(|&b| b == b'\n').enumerate() {
        if !line.starts_with(b"C-A2:") {
            continue;
        }
        let Some(eq) = line.iter().position(|&b| b == b'=') else {
            continue;
        };
        for field in line[eq + 1..].split(|&b| b == b',') {
            let pattern = trim_ascii(field);
            if !pattern.is_empty() {
                out.push((pattern.to_vec(), i as u64 + 1));
            }
        }
    }
    out
}

/// `Spine-Frozen: <oid> <path>` — EV §4.3's C-quoting, undone.
///
/// *"Deciding whether the path field is quoted is exact: it is quoted iff its
/// first byte is `"`. A real path beginning with `"` contains `"` and is
/// therefore always quoted, so the test can never misfire."*
fn parse_frozen(payload: &[u8]) -> Option<(String, Vec<u8>)> {
    let space = payload.iter().position(|&b| b == b' ')?;
    let oid = String::from_utf8(payload[..space].to_vec()).ok()?;
    let field = &payload[space + 1..];
    if field.first() != Some(&b'"') {
        return Some((oid, field.to_vec()));
    }
    let mut path = Vec::new();
    let mut i = 1;
    while i < field.len() {
        match field[i] {
            b'"' => return Some((oid, path)),
            b'\\' => {
                let escape = *field.get(i + 1)?;
                i += 2;
                match escape {
                    b'a' => path.push(0x07),
                    b'b' => path.push(0x08),
                    b't' => path.push(b'\t'),
                    b'n' => path.push(b'\n'),
                    b'v' => path.push(0x0B),
                    b'f' => path.push(0x0C),
                    b'r' => path.push(b'\r'),
                    b'"' => path.push(b'"'),
                    b'\\' => path.push(b'\\'),
                    // "`\` + exactly three octal digits, zero-padded" — the
                    // form every byte above 0x7E takes, and the reason a
                    // `café.json` freeze reads back as its own bytes.
                    b'0'..=b'7' => {
                        let mut value = u32::from(escape - b'0');
                        for _ in 0..2 {
                            let digit = *field.get(i)?;
                            if !(b'0'..=b'7').contains(&digit) {
                                return None;
                            }
                            value = value * 8 + u32::from(digit - b'0');
                            i += 1;
                        }
                        path.push(u8::try_from(value).ok()?);
                    }
                    _ => return None,
                }
            }
            byte => {
                path.push(byte);
                i += 1;
            }
        }
    }
    None
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|b| !matches!(b, b' ' | b'\t'))
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|b| !matches!(b, b' ' | b'\t'))
        .map_or(start, |i| i + 1);
    &bytes[start..end]
}

// ---------------------------------------------------------------------------
// The envelope, read
// ---------------------------------------------------------------------------
//
// **This reader is deliberately minimal and deliberately temporary.** EV owns
// the envelope — its closed name set, its rank order, its cap, its digests and
// every refusal — and `spine-envelope` is where that belongs; this crate may
// not depend on it. What follows reads only what PB §6.2's derivation table
// needs: which lines are present, their bytes, their line numbers, and the
// fenced block. It **validates nothing**: a malformed envelope is G9's finding,
// and a derivation that refused one would hide the landing G9 must report.

/// A `Spine-*` line, with the two facts a citation needs.
#[derive(Debug, Clone)]
struct Trailer {
    name: String,
    payload: Vec<u8>,
    /// The whole line, which DM §5.2.1 hashes for the `approval` id: *"from the
    /// first byte of the trailer name … through the last byte before its
    /// terminating LF, with no LF included."*
    raw: Vec<u8>,
}

impl Trailer {
    fn payload_str(&self) -> &str {
        core::str::from_utf8(&self.payload).unwrap_or_default()
    }
}

/// EV §2.6's fenced intent block.
#[derive(Debug, Clone)]
struct Fenced {
    blob: String,
    body: Vec<u8>,
    /// The 1-based message line the fenced body's own line 1 falls on.
    first_line: u64,
}

#[derive(Debug, Clone, Default)]
struct Envelope {
    fenced: Option<Fenced>,
    trailers: Vec<Trailer>,
}

impl Envelope {
    /// EV §2.3: *"A **`Spine-*` line** is a line whose first six bytes are `S`,
    /// `p`, `i`, `n`, `e`, `-` … case-sensitive."* Selection is *"purely
    /// lexical and total"* — deciding whether a line is one *"never requires
    /// parsing it, knowing its name, or judging it well-formed"*.
    fn read(message: &[u8]) -> Self {
        let mut envelope = Envelope::default();
        let lines: Vec<&[u8]> = message.split(|&b| b == b'\n').collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            if let Some(rest) = line.strip_prefix(b"-----BEGIN SPINE-INTENT ") {
                // EV §2.6: "A parser reads exactly `n` bytes. It never searches
                // for the END delimiter, so an intent that somehow contained
                // the END line as text could not truncate the block."
                let head = String::from_utf8_lossy(rest);
                let blob = field_after(&head, "blob=").unwrap_or_default();
                let bytes: usize = field_after(&head, "bytes=")
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
                // The body starts after this line's LF; find that offset in the
                // message so exactly `n` bytes can be taken.
                let start = offset_of_line(message, i + 1);
                if let Some(start) = start
                    && start + bytes <= message.len()
                {
                    let body = message[start..start + bytes].to_vec();
                    let body_lines = body.iter().filter(|&&b| b == b'\n').count();
                    envelope.fenced = Some(Fenced {
                        blob,
                        body,
                        first_line: i as u64 + 2,
                    });
                    i += body_lines + 1;
                    continue;
                }
            }
            if line.starts_with(b"Spine-")
                && let Some(colon) = line.iter().position(|&b| b == b':')
            {
                let name = String::from_utf8_lossy(&line[..colon]).into_owned();
                // EV §2.3: "the name is followed by exactly `:` `U+0020`". A
                // line missing the space is malformed and is still selected —
                // the payload is then whatever follows the colon.
                let payload_at = if line.get(colon + 1) == Some(&b' ') {
                    colon + 2
                } else {
                    colon + 1
                };
                envelope.trailers.push(Trailer {
                    name,
                    payload: line[payload_at.min(line.len())..].to_vec(),
                    raw: line.to_vec(),
                });
            }
            i += 1;
        }
        envelope
    }

    fn trailer(&self, name: &str) -> Option<&Trailer> {
        self.trailers.iter().find(|t| t.name == name)
    }

    fn trailers_named(&self, name: &str) -> Vec<&Trailer> {
        self.trailers.iter().filter(|t| t.name == name).collect()
    }

    /// PB §5.5's `envelope=`: *"SHA-256 over every `Spine-*` line above it, in
    /// order"* — EV §3.1 fixes the join as LF with **no trailing LF**, and EV
    /// §8.3 publishes the wrong value a trailing one produces.
    fn digest_above_seal(&self) -> String {
        let mut joined: Vec<u8> = Vec::new();
        for trailer in &self.trailers {
            if trailer.name == "Spine-Seal" {
                break;
            }
            if !joined.is_empty() {
                joined.push(b'\n');
            }
            joined.extend_from_slice(&trailer.raw);
        }
        sha256_prefixed(&joined)
    }
}

/// A trailer payload, split into its `key=value` fields.
///
/// EV §2.5: *"One `U+0020` between fields, none before the first, none after
/// the last"*, and *"`reason=` values are JSON string literals … a `"`
/// delimited run with JSON's escaping, so a reason containing a quote, a
/// backslash, a newline or any non-ASCII character is representable and the
/// line stays one line."* A naive split on spaces would truncate every reason
/// with a space in it, which is every reason.
#[derive(Debug, Clone, Default)]
struct Payload {
    /// PB §11: *"the first field is the landing's identity"* — `INT-042`,
    /// `quick` or `reseal`, present only on the lines that carry one.
    first: Option<Vec<u8>>,
    fields: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Payload {
    fn parse(payload: &[u8]) -> Self {
        let mut out = Payload::default();
        let mut i = 0;
        while i < payload.len() {
            if payload[i] == b' ' {
                i += 1;
                continue;
            }
            let start = i;
            let mut eq = None;
            while i < payload.len() && payload[i] != b' ' {
                if payload[i] == b'=' && eq.is_none() {
                    eq = Some(i);
                    // A quoted value runs to its closing quote, spaces and all.
                    if payload.get(i + 1) == Some(&b'"') {
                        i += 2;
                        while i < payload.len() {
                            match payload[i] {
                                b'\\' => i += 2,
                                b'"' => {
                                    i += 1;
                                    break;
                                }
                                _ => i += 1,
                            }
                        }
                        break;
                    }
                }
                i += 1;
            }
            match eq {
                Some(eq) => {
                    let mut value = payload[eq + 1..i].to_vec();
                    // The delimiters are not part of the value; the escapes are
                    // left as written, since nothing here re-emits them.
                    if value.first() == Some(&b'"') && value.last() == Some(&b'"') && value.len() > 1
                    {
                        value = value[1..value.len() - 1].to_vec();
                    }
                    out.fields.push((payload[start..eq].to_vec(), value));
                }
                None if out.first.is_none() && out.fields.is_empty() => {
                    out.first = Some(payload[start..i].to_vec());
                }
                None => {}
            }
        }
        out
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    fn get_str(&self, key: &str) -> Option<String> {
        self.get(key.as_bytes())
            .map(|v| String::from_utf8_lossy(&v).into_owned())
    }

    /// The first field when it is an intent id, and not `quick` or `reseal`.
    fn first_field_id(&self) -> Option<String> {
        let first = String::from_utf8(self.first.clone()?).ok()?;
        (first.starts_with("INT-") || first.starts_with("BUG-")).then_some(first)
    }
}

fn field_after(haystack: &str, key: &str) -> Option<String> {
    let at = haystack.find(key)? + key.len();
    let rest = &haystack[at..];
    let end = rest
        .find([' ', '-'])
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// The byte offset of 0-based line `n` in `message`.
fn offset_of_line(message: &[u8], n: usize) -> Option<usize> {
    let mut offset = 0;
    for _ in 0..n {
        offset += message[offset..].iter().position(|&b| b == b'\n')? + 1;
    }
    Some(offset)
}

// ---------------------------------------------------------------------------
// The intent document, read
// ---------------------------------------------------------------------------
//
// ID owns this grammar and every refusal in it; `spine-intent` is where the
// parser belongs, and this crate may not depend on it. What follows reads the
// members PB §6.2 and DM §7.2 turn into nodes and edges — id, title, owner,
// template, constitution, ACs, touchpoints — and **refuses nothing**: ID §8.3
// covers a landed document that does not parse, and a derivation that dropped
// a landing over a malformed doc would delete a ledger entry to report a
// document defect.

/// The parse, reduced to what the graph carries. Line numbers are 1-based
/// **within the document**, which [`Indexer::emit_landing`] rebases onto the
/// commit message.
#[derive(Debug, Clone)]
struct IntentDoc {
    id: String,
    title: Vec<u8>,
    owner: Option<Vec<u8>>,
    /// ID §5.6: *"the `intent.template` attr … is their canonical
    /// concatenation `<variant>@<n>`, reconstructed rather than copied"*.
    template: String,
    constitution: Option<u64>,
    title_line: u64,
    header_line: u64,
    acs: Vec<(u64, u64)>,
    touchpoints: Vec<(Vec<u8>, &'static str, u64)>,
}

impl IntentDoc {
    fn parse(bytes: &[u8]) -> Option<Self> {
        let lines: Vec<&[u8]> = bytes.split(|&b| b == b'\n').collect();
        // ID §4.2: "Line 1 … `# ` id `: ` title".
        let title_line = lines.first()?.strip_prefix(b"# ")?;
        let colon = title_line.windows(2).position(|w| w == b": ")?;
        let id = String::from_utf8(title_line[..colon].to_vec()).ok()?;
        let title = title_line[colon + 2..].to_vec();

        // ID §4.3: "Line 2 … a sequence of **fields** separated by the three
        // bytes `0x20 0xC2·0xB7 0x20` — space, U+00B7 MIDDLE DOT, space."
        let header = String::from_utf8_lossy(lines.get(1)?).into_owned();
        let mut owner = None;
        let mut template = None;
        let mut constitution = None;
        for field in header.split(" · ") {
            let Some((name, value)) = field.split_once(": ") else {
                continue;
            };
            match name {
                "Owner" => owner = Some(value.as_bytes().to_vec()),
                "Template" => template = Some(value.to_string()),
                "Constitution" => {
                    constitution = value.strip_prefix('v').and_then(|n| n.parse().ok());
                }
                _ => {}
            }
        }
        let template = canonical_template(&template?, &id, bytes);

        // ID §4.6: "A **heading line** is a line whose first three bytes are
        // exactly `## `"; §4.7 computes its key.
        let mut section = String::new();
        let mut acs = Vec::new();
        let mut touchpoints = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let line_no = i as u64 + 1;
            if let Some(rest) = line.strip_prefix(b"## ") {
                section = section_key(rest);
                continue;
            }
            match section.as_str() {
                // ID §5.3: `"AC-" number ": " text`, numbers contiguous from 1.
                "acceptance criteria" => {
                    if let Some(rest) = line.strip_prefix(b"AC-")
                        && let Some(colon) = rest.iter().position(|&b| b == b':')
                        && let Ok(n) = String::from_utf8_lossy(&rest[..colon]).parse::<u64>()
                    {
                        acs.push((n, line_no));
                    }
                }
                // ID §5.4: one label line per polarity, patterns split on `,`.
                "touchpoints" => {
                    let Some(colon) = line.iter().position(|&b| b == b':') else {
                        continue;
                    };
                    let label = String::from_utf8_lossy(trim_ascii(&line[..colon])).to_lowercase();
                    let polarity = match label.as_str() {
                        "expected to change" => "expected",
                        "must not change" => "forbidden",
                        _ => continue,
                    };
                    for field in line[colon + 1..].split(|&b| b == b',') {
                        let pattern = trim_ascii(field);
                        if !pattern.is_empty() {
                            touchpoints.push((pattern.to_vec(), polarity, line_no));
                        }
                    }
                }
                _ => {}
            }
        }

        Some(IntentDoc {
            id,
            title,
            owner,
            template,
            constitution,
            title_line: 1,
            header_line: 2,
            acs,
            touchpoints,
        })
    }
}

/// ID §3.2's `<variant>@<n>`, or §3.3's derivation for the legacy bare `v<n>`:
///
/// > ```text
/// > variant_legacy(d) :=
/// >   "intent-bug"     if the id's prefix is "BUG"
/// >   "intent-change"  else if d contains a line whose section key is "invariants"
/// >   "intent"         otherwise
/// > ```
fn canonical_template(value: &str, id: &str, doc: &[u8]) -> String {
    if value.contains('@') {
        return value.to_string();
    }
    let Some(version) = value.strip_prefix('v') else {
        return value.to_string();
    };
    let variant = if id.starts_with("BUG") {
        "intent-bug"
    } else if doc
        .split(|&b| b == b'\n')
        .filter_map(|line| line.strip_prefix(b"## "))
        .any(|rest| section_key(rest) == "invariants")
    {
        "intent-change"
    } else {
        "intent"
    };
    format!("{variant}@{version}")
}

/// ID §4.7's three steps: the bytes after `## `, stripped of spaces and tabs,
/// truncated before the first `(`, stripped again, and ASCII-lowercased.
fn section_key(rest: &[u8]) -> String {
    let stripped = trim_ascii(rest);
    let before_paren = match stripped.iter().position(|&b| b == b'(') {
        Some(at) => &stripped[..at],
        None => stripped,
    };
    String::from_utf8_lossy(trim_ascii(before_paren)).to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frozen_path_round_trips_through_git_c_quoting() {
        // EV §8.2's own line: `tests/fixtures/café.json` holds 0xC3 0xA9, "so
        // it is wrapped and the two bytes become `\303\251`".
        let (oid, path) = parse_frozen(
            b"0c3a7f18e2b56d94a0c7f3e18b52d6a4907c1e3f \"tests/fixtures/caf\\303\\251.json\"",
        )
        .unwrap();
        assert_eq!(oid, "0c3a7f18e2b56d94a0c7f3e18b52d6a4907c1e3f");
        assert_eq!(path, "tests/fixtures/café.json".as_bytes());
    }

    #[test]
    fn a_space_in_a_frozen_path_does_not_trigger_quoting_and_the_split_is_the_first_space() {
        // EV §4.3: "A space does not trigger quoting … Parsing is still exact:
        // the payload splits at its **first** space."
        let (oid, path) =
            parse_frozen(b"1e9f4b7d0c3a6e589b2d4f7a1c0e3b6d8f2a5c94 tests/fixtures/tax rates.json")
                .unwrap();
        assert_eq!(oid, "1e9f4b7d0c3a6e589b2d4f7a1c0e3b6d8f2a5c94");
        assert_eq!(path, b"tests/fixtures/tax rates.json");
    }

    #[test]
    fn a_reason_with_spaces_is_one_field_and_the_fields_after_it_survive() {
        // A naive split on spaces truncates the reason and loses `reviewer=`,
        // which would drop the `signed_by` edge of every reviewed landing.
        let payload = br#"INT-042 class=tripwire wires=G11,G2:src/x.ts reason="shared helper touched outside touchpoints; read the diff" reviewer=bob@example.com"#;
        let parsed = Payload::parse(payload);
        assert_eq!(parsed.first_field_id().as_deref(), Some("INT-042"));
        assert_eq!(parsed.get_str("class").as_deref(), Some("tripwire"));
        assert_eq!(
            parsed.get_str("reason").as_deref(),
            Some("shared helper touched outside touchpoints; read the diff")
        );
        assert_eq!(
            parsed.get_str("reviewer").as_deref(),
            Some("bob@example.com")
        );
        assert_eq!(parsed.get_str("wires").as_deref(), Some("G11,G2:src/x.ts"));
    }

    #[test]
    fn quick_and_reseal_are_not_intent_ids_so_their_approvals_name_the_landing() {
        // PB §6.2: `approves` "names the intent for every line carrying an id
        // and the landing changeset `cs:<L>` for those that do not".
        for first in ["quick", "reseal"] {
            let payload = format!("{first} class=protected reviewer=bob@example.com");
            assert!(Payload::parse(payload.as_bytes()).first_field_id().is_none());
        }
        assert!(
            Payload::parse(b"BUG-051 blob=aa signer=a@b").first_field_id().is_some(),
            "a BUG- id is an id"
        );
    }

    #[test]
    fn the_fenced_block_is_read_by_byte_count_and_never_by_searching_for_the_end() {
        // EV §2.6, and the reason: "an intent that somehow contained the END
        // line as text could not truncate the block."
        let body = "# INT-1: t\nOwner: @a · Template: intent@2 · Constitution: v3\n-----END SPINE-INTENT-----\nstill inside\n";
        let message = format!(
            "INT-1: t\n\n-----BEGIN SPINE-INTENT blob=aa bytes={}-----\n{body}-----END SPINE-INTENT-----\n\nSpine-Envelope: 1\nSpine-Seal: INT-1 base=bb\n",
            body.len()
        );
        let envelope = Envelope::read(message.as_bytes());
        let fenced = envelope.fenced.clone().unwrap();
        assert_eq!(fenced.body, body.as_bytes());
        // Line 1 of the message is the subject, 2 is blank, 3 is the fence, so
        // the body's first line is message line 4.
        assert_eq!(fenced.first_line, 4);
        // The `-----END-----` inside the body did not end the block, and the
        // trailer scan resumed *after* it rather than inside it.
        assert!(envelope.trailer("Spine-Seal").is_some());
    }

    #[test]
    fn the_envelope_digest_joins_with_lf_and_never_terminates_with_one() {
        // EV §8.3: "**Verified**, over exactly those fifteen lines joined by
        // fourteen `0x0A`", and the published wrong value is the trailing-LF
        // reading. Two lines here, one separator.
        let message = "s\n\nSpine-Envelope: 1\nSpine-Event: land\nSpine-Seal: x\nSpine-Seal-Sig: y\n";
        let joined = b"Spine-Envelope: 1\nSpine-Event: land";
        assert_eq!(
            Envelope::read(message.as_bytes()).digest_above_seal(),
            sha256_prefixed(joined)
        );
        assert_ne!(
            Envelope::read(message.as_bytes()).digest_above_seal(),
            sha256_prefixed(b"Spine-Envelope: 1\nSpine-Event: land\n"),
            "the trailing-LF reading is the published wrong value"
        );
    }

    #[test]
    fn an_intent_doc_yields_its_acs_and_both_touchpoint_polarities_with_label_lines() {
        let doc = b"# INT-042: Invoice totals include tax\nOwner: @alice \xc2\xb7 Template: intent@2 \xc2\xb7 Constitution: v3\n\n## Goal\nprose\n\n## Acceptance criteria\nAC-1: one\nAC-2: two\n\n## Touchpoints (expected blast radius)\nExpected to change: src/billing/, api/invoices.ts\nMust NOT change: auth/, shared/schema/\n";
        let parsed = IntentDoc::parse(doc).unwrap();
        assert_eq!(parsed.id, "INT-042");
        assert_eq!(parsed.title, b"Invoice totals include tax");
        assert_eq!(parsed.owner.as_deref(), Some(&b"@alice"[..]));
        assert_eq!(parsed.template, "intent@2");
        assert_eq!(parsed.constitution, Some(3));
        assert_eq!(parsed.acs, vec![(1, 8), (2, 9)]);
        // ID §6.6: "the line number … is the touchpoint **label line's**, not
        // the individual pattern's, since several patterns share one line."
        assert_eq!(
            parsed.touchpoints,
            vec![
                (b"src/billing/".to_vec(), "expected", 12),
                (b"api/invoices.ts".to_vec(), "expected", 12),
                (b"auth/".to_vec(), "forbidden", 13),
                (b"shared/schema/".to_vec(), "forbidden", 13),
            ]
        );
    }

    #[test]
    fn a_headings_parenthetical_is_discarded_when_its_key_is_computed() {
        // ID §4.7: "`## Acceptance criteria (maximum 6 — more means split the
        // task)` has key `acceptance criteria`".
        assert_eq!(
            section_key("Acceptance criteria (maximum 6 — more means split the task)".as_bytes()),
            "acceptance criteria"
        );
        assert_eq!(section_key(b"Non-Goals"), "non-goals");
    }

    #[test]
    fn a_legacy_bare_template_derives_its_variant_from_the_id_and_the_headings() {
        // ID §3.3's `variant_legacy`, all three arms.
        assert_eq!(canonical_template("intent-bug@2", "BUG-1", b""), "intent-bug@2");
        assert_eq!(canonical_template("v2", "BUG-051", b""), "intent-bug@2");
        assert_eq!(
            canonical_template("v2", "INT-042", b"## Invariants\nx\n"),
            "intent-change@2"
        );
        assert_eq!(canonical_template("v2", "INT-042", b"## Goal\nx\n"), "intent@2");
    }

    #[test]
    fn the_constitution_header_is_line_2_by_rule_and_its_name_casefolds() {
        // CN §3.1: "The header's position is fixed rather than located", which
        // is why `dump.md` §12.2 cites line 2.
        assert_eq!(
            constitution_version(b"# Constitution \xe2\x80\x94 myrepo\nVersion: v3 \xc2\xb7 Owner: @alice\n"),
            Some(3)
        );
        assert_eq!(
            constitution_version(b"# C\nversion: v12 \xc2\xb7 Owner: @a\n"),
            Some(12),
            "`version:` parses (CN §9.1)"
        );
        assert_eq!(constitution_version(b"# C\nOwner: @a\n"), None);
    }

    #[test]
    fn every_c_a2_pattern_on_one_line_shares_that_lines_number() {
        // CN §9.6, verbatim: "**Every pattern on the line shares the line's
        // number**".
        let file = b"# Constitution\nVersion: v3\n\nC-A1: mode = team\nC-A2: protected = adr/, infra/\n";
        assert_eq!(
            effective_c_a2(file),
            vec![(b"adr/".to_vec(), 5), (b"infra/".to_vec(), 5)]
        );
        // An indented `C-A2` is not a rule line (CN §3.2 test 3 refuses it as
        // `indented-rule`); it must not silently become a floor entry here.
        assert!(effective_c_a2(b"# C\nVersion: v1\n  C-A2: protected = secret/\n").is_empty());
    }

    #[test]
    fn an_adr_id_is_the_headings_leading_token() {
        assert_eq!(heading_id(b"# ADR-007: Tax rounding").as_deref(), Some("ADR-007"));
        assert_eq!(heading_id(b"ADR-1 rounding").as_deref(), Some("ADR-1"));
        assert_eq!(heading_id(b"# ").as_deref(), None);
    }
}
