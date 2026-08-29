//! A `spine-resolve` [`Tree`] over a git tree object.
//!
//! IR §2.1 makes the closure a function of two trees and nothing else — "Not
//! inputs: the working tree, `HEAD`, any ref other than the two commits above
//! … and the order in which git enumerates a tree." So this reads a commit's
//! tree through `ls-tree -r` and sorts, rather than walking a checkout.

use spine_init::Repo;
use spine_resolve::tree::{Entry, EntryKind, Tree};

#[derive(Debug)]
pub struct GitTree {
    entries: Vec<Entry>,
    blobs: std::collections::BTreeMap<String, Vec<u8>>,
}

impl GitTree {
    /// Read one commit's tree, blobs included.
    ///
    /// Blobs are read eagerly because the closure walk reads most of what it
    /// enumerates and a lazy reader would need interior mutability for no gain
    /// on a tree of this size.
    pub fn read(repo: &Repo, commit: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut entries = Vec::new();
        let mut blobs = std::collections::BTreeMap::new();
        for (mode, _oid, path) in repo.ls_tree_all(commit)? {
            // IR's `Tree` distinguishes a file from a symlink and a submodule:
            // `is_file` is "An ordinary file, and not a symlink or a
            // submodule", and a resolver that followed one would resolve
            // through a link the branch controls.
            let kind = match mode.as_str() {
                "120000" => EntryKind::Symlink,
                "160000" => EntryKind::Submodule,
                _ => EntryKind::File,
            };
            if kind == EntryKind::File
                && let Some(bytes) = repo.read_at(commit, &path)
            {
                blobs.insert(path.clone(), bytes);
            }
            entries.push(Entry { path, kind });
        }
        // "Every entry, **sorted by path bytes ascending**. The order is part
        // of the contract."
        entries.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
        Ok(GitTree { entries, blobs })
    }
}

impl Tree for GitTree {
    fn entries(&self) -> &[Entry] {
        &self.entries
    }

    fn read(&self, path: &str) -> Option<&[u8]> {
        self.blobs.get(path).map(Vec::as_slice)
    }
}
