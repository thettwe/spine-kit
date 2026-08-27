//! The tree a resolver resolves against, and the two entry kinds it must
//! refuse rather than follow.
//!
//! IR §2.12 rule 1: every read names `A` or `B`. "There is no third tree and no
//! filesystem." That is why this is a trait over a *tree* and never a path on
//! disk: IR §15 rule 10, "two runs of one release over one pair of trees
//! produce the same set on any host", and case C14 requires a byte-identical
//! closure "with a dirty working directory, or in a bare clone".

use crate::lang::Unresolvable;

/// The git entry kinds a resolver distinguishes. Ordinary blobs (`100644`,
/// `100755`) are [`EntryKind::File`]; the other two are the ones IR §2.12
/// rule 2 forbids following.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntryKind {
    File,
    /// Mode `120000`. Case C15: "`unresolvable`, reason `symlink-or-submodule`;
    /// **must not** follow the link."
    Symlink,
    /// Mode `160000`. Case C16: "**must not** descend into the submodule."
    Submodule,
}

/// One entry of a tree listing, as `git ls-tree -r` produces it: blobs,
/// symlinks and submodule gitlinks. **Directories are not entries** — git does
/// not list them under `-r`, and [`Tree::is_dir`] derives them, so no
/// implementation has to synthesise a listing git did not give it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub kind: EntryKind,
}

/// A tree, read-only and complete.
pub trait Tree {
    /// Every entry, **sorted by path bytes ascending**. The order is part of
    /// the contract because IR §2.12 rule 6 requires every candidate list to be
    /// ordered and exhaustive, and a resolver that enumerated a directory in
    /// listing order would otherwise inherit whatever order the caller had.
    fn entries(&self) -> &[Entry];

    /// The blob's bytes, or `None` where the path is not a readable blob.
    fn read(&self, path: &str) -> Option<&[u8]>;

    fn kind(&self, path: &str) -> Option<EntryKind> {
        self.entries()
            .binary_search_by(|e| e.path.as_str().cmp(path))
            .ok()
            .map(|i| self.entries()[i].kind)
    }

    /// An ordinary file, and not a symlink or a submodule.
    fn is_file(&self, path: &str) -> bool {
        self.kind(path) == Some(EntryKind::File)
    }

    /// Derived: some entry lies beneath it. IR §4.3's second root exists "if
    /// and only if a tree entry `src` exists and is a directory", and this is
    /// how a `-r` listing answers that.
    fn is_dir(&self, path: &str) -> bool {
        let prefix = format!("{path}/");
        self.entries().iter().any(|e| e.path.starts_with(&prefix))
    }

    /// IR §2.12 rule 2, applied to a candidate **and to every ancestor of it**.
    ///
    /// Case C15 covers the candidate itself; case C16 covers "a resolved
    /// candidate **under** a `160000` entry", which is a path whose ancestor is
    /// the gitlink. Both answer `symlink-or-submodule`, and a resolver that
    /// checked only the leaf would descend into a submodule by not noticing it
    /// had.
    fn refuses_to_follow(&self, path: &str) -> Option<Unresolvable> {
        let mut prefix = path;
        loop {
            if matches!(
                self.kind(prefix),
                Some(EntryKind::Symlink) | Some(EntryKind::Submodule)
            ) {
                return Some(Unresolvable::SymlinkOrSubmodule);
            }
            let cut = prefix.rfind('/')?;
            prefix = &prefix[..cut];
        }
    }
}

/// An in-memory tree. This is the shape a test states a tree fragment in, and
/// the shape a caller holding a `git ls-tree -r` listing builds.
#[derive(Debug, Clone, Default)]
pub struct MapTree {
    entries: Vec<Entry>,
    blobs: Vec<(String, Vec<u8>)>,
}

impl MapTree {
    /// Build from `(path, contents)` pairs, all ordinary files.
    pub fn new<I, P, C>(files: I) -> MapTree
    where
        I: IntoIterator<Item = (P, C)>,
        P: Into<String>,
        C: Into<Vec<u8>>,
    {
        let mut tree = MapTree::default();
        for (path, contents) in files {
            tree.insert(path.into(), contents.into(), EntryKind::File);
        }
        tree
    }

    /// Add an entry git records with a mode a resolver may not follow.
    pub fn with_special(mut self, path: impl Into<String>, kind: EntryKind) -> MapTree {
        self.insert(path.into(), Vec::new(), kind);
        self
    }

    fn insert(&mut self, path: String, contents: Vec<u8>, kind: EntryKind) {
        // Kept sorted on insertion so `entries()` can promise the order and
        // `kind()` can binary-search it.
        let entry = Entry {
            path: path.clone(),
            kind,
        };
        match self.entries.binary_search_by(|e| e.path.cmp(&path)) {
            Ok(i) => self.entries[i] = entry,
            Err(i) => self.entries.insert(i, entry),
        }
        match self.blobs.binary_search_by(|b| b.0.cmp(&path)) {
            Ok(i) => self.blobs[i].1 = contents,
            Err(i) => self.blobs.insert(i, (path, contents)),
        }
    }
}

impl Tree for MapTree {
    fn entries(&self) -> &[Entry] {
        &self.entries
    }

    fn read(&self, path: &str) -> Option<&[u8]> {
        if !self.is_file(path) {
            return None;
        }
        self.blobs
            .binary_search_by(|b| b.0.as_str().cmp(path))
            .ok()
            .map(|i| self.blobs[i].1.as_slice())
    }
}

/// Lexical path normalization: collapse `.` and `..` **textually**, never by a
/// tree lookup. IR §5.2 step 1 requires it for TypeScript by name — "the
/// lexical normalization of `dirname(f) + "/" + s`, collapsing `.` and `..`
/// textually" — and IR §6.2 for Dart.
///
/// `None` means the path escaped the repository root, which every language
/// spells `relative-escapes-root`.
pub fn normalize(path: &str) -> Option<String> {
    let mut out: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                // "If it escapes the repository root → `unresolvable`" — an
                // implementation that clamped at the root instead would resolve
                // `../../etc/passwd` to `etc/passwd` and freeze a file the
                // specifier never named.
                out.pop()?;
            }
            other => out.push(other),
        }
    }
    Some(out.join("/"))
}

/// The directory containing `path`, or the empty string for a root-level file.
pub fn dirname(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

/// Join a directory (possibly empty, meaning the root) to a relative path.
pub fn join(dir: &str, rest: &str) -> String {
    if dir.is_empty() {
        rest.to_string()
    } else if rest.is_empty() {
        dir.to_string()
    } else {
        format!("{dir}/{rest}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_are_sorted_by_path_bytes_whatever_order_they_arrived_in() {
        let tree = MapTree::new([("b/x.py", ""), ("a/y.py", ""), ("a/b/z.py", "")]);
        let paths: Vec<&str> = tree.entries().iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, ["a/b/z.py", "a/y.py", "b/x.py"]);
    }

    /// IR §4.3's second root "if and only if a tree entry `src` exists and is a
    /// directory" — derived from a `-r` listing, which lists no directories.
    #[test]
    fn a_directory_is_derived_from_the_entries_beneath_it() {
        let tree = MapTree::new([("src/pkg/mod.py", ""), ("README.md", "")]);
        assert!(tree.is_dir("src"));
        assert!(tree.is_dir("src/pkg"));
        assert!(!tree.is_dir("srcx"));
        // A file is not a directory, and a directory is not a file.
        assert!(!tree.is_dir("README.md"));
        assert!(!tree.is_file("src"));
    }

    /// Cases C15 and C16: the leaf and every ancestor of it.
    #[test]
    fn a_symlink_or_a_submodule_refuses_at_the_leaf_and_at_every_ancestor() {
        let tree = MapTree::new([("a/b.py", "")])
            .with_special("link.py", EntryKind::Symlink)
            .with_special("vendor", EntryKind::Submodule);
        assert_eq!(tree.refuses_to_follow("a/b.py"), None);
        assert_eq!(
            tree.refuses_to_follow("link.py"),
            Some(Unresolvable::SymlinkOrSubmodule)
        );
        // "A resolved candidate **under** a `160000` entry" — the gitlink is an
        // ancestor, not the leaf.
        assert_eq!(
            tree.refuses_to_follow("vendor/deep/mod.py"),
            Some(Unresolvable::SymlinkOrSubmodule)
        );
        // …and `read` never returns a symlink's bytes as if they were a file's.
        assert_eq!(tree.read("link.py"), None);
    }

    /// "If it escapes the repository root → `unresolvable`" — never clamped.
    #[test]
    fn normalization_is_textual_and_escaping_the_root_is_refused() {
        assert_eq!(normalize("a/./b/../c").as_deref(), Some("a/c"));
        assert_eq!(normalize("a/b/../../c").as_deref(), Some("c"));
        assert_eq!(normalize("a/../../etc/passwd"), None);
        assert_eq!(normalize("../x"), None);
        assert_eq!(normalize("a//b").as_deref(), Some("a/b"));
    }

    #[test]
    fn dirname_and_join_agree_at_the_root() {
        assert_eq!(dirname("a/b/c.py"), "a/b");
        assert_eq!(dirname("c.py"), "");
        assert_eq!(join("", "c.py"), "c.py");
        assert_eq!(join("a/b", "c.py"), "a/b/c.py");
        assert_eq!(join("a/b", ""), "a/b");
    }
}
