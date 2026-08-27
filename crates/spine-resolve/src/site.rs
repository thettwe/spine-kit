//! An import site and its disposition — IR §3.2, shared by all four resolvers.

use crate::lang::Unresolvable;

/// IR §3.2's four dispositions. "`disposition(i)` is exactly one of" these, and
/// each row of that table fixes both whether an edge is drawn and whether a
/// finding is raised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposition {
    /// "resolves to exactly one path `m` present in the tree being resolved
    /// against" — but IR §3.2 immediately widens it: "**One site yields at most
    /// one disposition, but a site may yield several `repo` targets.** Three
    /// forms genuinely name more than one file: a Python dotted import executes
    /// every ancestor package `__init__.py` as well as the module (§4.3); a
    /// Dart conditional import names one URI per branch (§6.2); a Swift
    /// `import M` names every source file of `M` (§7.4). … It is never a reason
    /// to call the site `unresolvable`."
    Repo(Vec<String>),
    /// "resolves outside the repository: a package, a stdlib or SDK module, a
    /// framework, generated code that is not in the tree."
    ///
    /// IR §3.2: "**`external` is the safe default for a bare name that matches
    /// nothing.** This is worth stating because the instinct runs the other
    /// way. An oracle must live in the repository to be an oracle; if it lives
    /// in the repository it is a tree entry; if it is a tree entry the
    /// language's resolution rule finds it. … Calling it `unresolvable` instead
    /// would make every Swift `import Foundation` and `import XCTest` a
    /// tripwire, which would mean every Swift approval routes to a human, which
    /// would mean the tripwire carries no information."
    External,
    /// "a recognized type-only form (§3.6)". Only TypeScript has one.
    TypeOnly,
    /// "recognized as an import site, but the target cannot be determined" —
    /// no edge, and `unresolvable-import` where the file satisfies `H`.
    Unresolvable(Unresolvable),
}

/// IR §3.2: "An **import site** is one syntactic occurrence, in one file, of one
/// of the forms §4–§7 enumerate for that file's language. Sites are identified
/// by `(path, byte offset of the first token)`, which makes them countable and
/// reportable and gives findings a stable location."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSite {
    /// The byte offset of the site's own first token, within the file.
    pub offset: usize,
    pub disposition: Disposition,
}

impl ImportSite {
    /// The paths this site draws edges to. Empty for every disposition but
    /// `repo`.
    pub fn targets(&self) -> &[String] {
        match &self.disposition {
            Disposition::Repo(targets) => targets,
            _ => &[],
        }
    }

    /// IR §2.11's tripwire input: a site whose target could not be determined.
    /// It is a *tripwire* only where the file satisfies `H`; outside the
    /// harness it is the counter `unresolvable-import-outside-harness`
    /// (IR §12.4.3), which is why this reports the reason rather than the
    /// finding.
    pub fn unresolvable(&self) -> Option<Unresolvable> {
        match &self.disposition {
            Disposition::Unresolvable(reason) => Some(*reason),
            _ => None,
        }
    }
}
