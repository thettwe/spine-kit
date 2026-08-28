//! PB §4.3's **freeze closure** — what `--approve` freezes, and why it is not
//! the file list.
//!
//! > "A frozen test that imports an unfrozen fixture is frozen in name only —
//! > A weakens the fixture and the test passes."
//!
//! The closure is a function of IR §2.1's closed input set and nothing else:
//! the approval tree `A`, the base tree `B`, the intent's `expected`
//! touchpoints `E`, its acceptance-criterion numbers, `C-T1`/`C-T2` from the
//! constitution at `base`, `params.langs`, and the pinned release. Not the
//! working tree, not `HEAD`, not a clock, and — the one worth stating twice —
//! **not a collection**: "`--approve` writes no result file", and IR §2.1.1
//! records that defining the seed over collected ids made it unimplementable
//! at the place it is recomputed.

use std::collections::{BTreeMap, BTreeSet};

use crate::glob::Pattern;
use crate::lang::{Lang, lang as lang_of};
use crate::pragma::IntentId;
use crate::site::Disposition;
use crate::tree::Tree;

/// Why a path is in the closure, for the diagnostic a human reads and for the
/// tripwire PB §4.3 names rather than freezes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// IR §2.1.1's `S`: under a `C-T1` root, carrying a pragma of this intent.
    Seed,
    /// Reached by a repo-local import from something already in the closure.
    Imported,
    /// "It is **frozen as a leaf** in exactly one case: the module existed at
    /// `base=` and no non-test file imported it there." Test-only code living
    /// at the address of code under test.
    TestOnlyLeaf,
    /// "an import that resolves outside both expected and the harness is frozen
    /// as a leaf, because A had no business touching it".
    OutsideBothLeaf,
}

impl Reason {
    /// Whether the walk continues through this path. A leaf is frozen and not
    /// followed: "The walk **prunes at an excluded import** — what code under
    /// test imports is code under test", and the two leaf cases are the edge of
    /// the same boundary.
    fn walks(self) -> bool {
        matches!(self, Reason::Seed | Reason::Imported)
    }
}

/// The closure, and what the walk met and did not freeze.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Closure {
    /// Every frozen path, sorted, with why.
    pub frozen: BTreeMap<String, Reason>,
    /// Paths the walk reached and **excluded** as code under test: they
    /// existed at `base` and a non-test file imported them there, or they did
    /// not exist at `base` at all.
    ///
    /// Kept because PB §4.3 names the second case as a tripwire rather than a
    /// freeze — "the stub the red tests import, which the tripwire below names
    /// rather than freezes" — so a caller has to be able to name it.
    pub excluded: BTreeSet<String>,
    /// Sites the resolver could not resolve, in files that are in the closure.
    /// IR §2.11 makes these a tripwire where the file satisfies `H`.
    pub unresolvable: BTreeSet<String>,
}

impl Closure {
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.frozen.keys().map(String::as_str)
    }
}

/// IR §2.1's inputs, gathered so a caller cannot supply half of them.
/// `Debug` is written rather than derived: `dyn Tree` has none, and giving the
/// trait one would put a formatting requirement on every implementation for the
/// sake of a derive — the same reason `spine-graph`'s `Indexer` writes its own.
#[derive(Clone, Copy)]
pub struct Inputs<'a> {
    /// `A` — the approval tree. Every resolution is performed against it
    /// (IR §2.2).
    pub approval: &'a dyn Tree,
    /// `B` — the base tree. It answers only "was this already here, and did
    /// non-test code already use it" (IR §2.2).
    pub base: &'a dyn Tree,
    /// `C-T1` — the test roots. The seed is drawn from these.
    pub c_t1: &'a [Pattern],
    /// `C-T2` — the harness configuration. `H(p)` is `C-T1 ∪ C-T2`
    /// (IR §2.3), and IR §17 D3 records that PB's three-way phrasing
    /// "`C-T1`/`C-T2`/runner-config" names this same set.
    pub c_t2: &'a [Pattern],
    /// `E` — the intent's "Expected to change" list.
    pub expected: &'a [Pattern],
    /// The intent whose pragmas seed the walk.
    pub intent: &'a IntentId,
    /// How many acceptance criteria it has, so a pragma naming `AC-9` on a
    /// three-criterion intent seeds nothing (IR §2.1.1).
    pub ac_count: u8,
}

impl Inputs<'_> {
    /// IR §2.3: "`H(p)` is true iff `p` matches any pattern in `C-T1` ∪ `C-T2`."
    fn harness(&self, path: &str) -> bool {
        matches(self.c_t1, path) || matches(self.c_t2, path)
    }

    fn expected(&self, path: &str) -> bool {
        matches(self.expected, path)
    }
}

impl core::fmt::Debug for Inputs<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Inputs")
            .field("intent", &self.intent.as_str())
            .field("ac_count", &self.ac_count)
            .field("c_t1", &self.c_t1.len())
            .field("c_t2", &self.c_t2.len())
            .field("expected", &self.expected.len())
            .finish_non_exhaustive()
    }
}

fn matches(patterns: &[Pattern], path: &str) -> bool {
    patterns.iter().any(|p| p.matches(path.as_bytes()))
}

/// PB §4.3's clause (1): "every file under a `C-T1` test root, in the approval
/// tree, carrying a pragma naming an acceptance criterion of this intent."
///
/// **Lexical, not collected.** IR §2.1.1: "Nothing a collection produces is
/// read." The pragma is scanned out of the file's bytes in `A`.
pub fn seed(inputs: &Inputs<'_>) -> Vec<String> {
    inputs
        .approval
        .entries()
        .iter()
        .filter(|e| e.kind == crate::tree::EntryKind::File)
        .filter(|e| matches(inputs.c_t1, &e.path))
        .filter(|e| {
            let Some(bytes) = inputs.approval.read(&e.path) else {
                return false;
            };
            let Ok(text) = core::str::from_utf8(bytes) else {
                return false;
            };
            crate::pragma::scan_path(text, &e.path)
                .iter()
                .any(|o| seeds(o, inputs))
        })
        .map(|e| e.path.clone())
        .collect()
}

/// IR §2.1.1: "A pragma this intent does not own seeds nothing … `@verifies
/// INT-042/AC-9` where the intent has three criteria" is an occurrence and not
/// a seed.
fn seeds(occurrence: &crate::pragma::Occurrence, inputs: &Inputs<'_>) -> bool {
    occurrence.intent.as_str() == inputs.intent.as_str()
        && occurrence.ac.names_a_criterion_of(inputs.ac_count)
}

/// The closure: PB §4.3's clauses (1) and (2).
///
/// Clause (2) verbatim, because every limb decides a different path:
///
/// > "An import that resolves inside the intent's `expected` touchpoints and
/// > outside every `C-T1`/`C-T2`/runner-config pattern is **code under test,
/// > and excluded, in two cases**: it resolves into a module that existed at
/// > the approval's `base=` and was imported there by a non-test file, or into
/// > a module that did not exist at `base=` at all … It is **frozen as a leaf**
/// > in exactly one case: the module existed at `base=` and no non-test file
/// > imported it there. The walk **prunes at an excluded import**".
pub fn closure(inputs: &Inputs<'_>) -> Closure {
    let mut out = Closure::default();
    let imported_by_non_test_at_base = non_test_imports_at_base(inputs);

    // Breadth-first over a sorted frontier, so the result is a function of the
    // tree and not of the order anything was met (IR §15 rule 5).
    let mut frontier: Vec<String> = seed(inputs);
    frontier.sort();
    for path in &frontier {
        out.frozen.insert(path.clone(), Reason::Seed);
    }

    while let Some(path) = frontier.pop() {
        if !out.frozen.get(&path).is_some_and(|r| r.walks()) {
            continue;
        }
        let mut targets: Vec<String> = Vec::new();
        for site in sites_of(inputs.approval, &path) {
            match &site.disposition {
                Disposition::Repo(paths) => targets.extend(paths.iter().cloned()),
                Disposition::Unresolvable(_) => {
                    out.unresolvable.insert(path.clone());
                }
                _ => {}
            }
        }
        targets.sort();
        targets.dedup();

        for target in targets {
            if out.frozen.contains_key(&target) || out.excluded.contains(&target) {
                continue;
            }
            // "an import that resolves inside the intent's `expected`
            // touchpoints **and outside** every harness pattern is code under
            // test" — both conjuncts, so a test file inside `expected` is
            // still harness and still frozen.
            let code_under_test = inputs.expected(&target) && !inputs.harness(&target);
            let reason = if code_under_test {
                let existed = inputs.base.is_file(&target);
                let used_by_non_test = imported_by_non_test_at_base.contains(&target);
                match (existed, used_by_non_test) {
                    // "the module existed at `base=` and no non-test file
                    // imported it there" — test-only code at the address of
                    // code under test, frozen as a leaf.
                    (true, false) => Some(Reason::TestOnlyLeaf),
                    // Excluded: already code under test, or the stub the red
                    // tests import.
                    (true, true) | (false, _) => None,
                }
            } else if inputs.harness(&target) {
                Some(Reason::Imported)
            } else {
                // "an import that resolves outside both expected and the
                // harness is frozen as a leaf, because A had no business
                // touching it".
                Some(Reason::OutsideBothLeaf)
            };

            match reason {
                Some(reason) => {
                    out.frozen.insert(target.clone(), reason);
                    if reason.walks() {
                        frontier.push(target);
                    }
                }
                None => {
                    out.excluded.insert(target);
                }
            }
        }
    }

    out
}

/// PB §4.3's clause-2 test, evaluated "**wholly in `B`**" (IR §2.2): which
/// paths a **non-test** file imported at `base`.
///
/// "It is read from the base tree, which the branch cannot edit" — which is
/// the whole reason the test is there: a branch that could add a non-test
/// importer could turn any test-only module into code under test and unfreeze
/// its subtree.
fn non_test_imports_at_base(inputs: &Inputs<'_>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in inputs.base.entries() {
        if entry.kind != crate::tree::EntryKind::File || inputs.harness(&entry.path) {
            continue;
        }
        for site in sites_of(inputs.base, &entry.path) {
            if let Disposition::Repo(paths) = &site.disposition {
                out.extend(paths.iter().cloned());
            }
        }
    }
    out
}

/// One file's import sites, dispatched by language.
///
/// A file whose language is unclassifiable contributes no sites rather than
/// failing the walk: IR §3.8's language level already decides what an
/// unreadable package costs, and a closure that refused here would make one
/// stray file unfreezable rather than under-frozen.
fn sites_of(tree: &dyn Tree, path: &str) -> Vec<crate::site::ImportSite> {
    let Some(bytes) = tree.read(path) else {
        return Vec::new();
    };
    let Ok(source) = core::str::from_utf8(bytes) else {
        return Vec::new();
    };
    match lang_of(path) {
        Some(Lang::Python) => crate::python::sites(source, path, tree),
        Some(Lang::Ts) => crate::ts::sites(source, path, tree, &crate::ts::Rc::default()),
        Some(Lang::Dart) => crate::dart::sites(source, path, tree, &crate::dart::Rc::default()),
        // Swift's resolver is module-keyed and needs a package graph this walk
        // does not carry; a Swift closure is owed and is not guessed at here.
        Some(Lang::Swift) | None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::MapTree;

    fn pattern(p: &str) -> Pattern {
        Pattern::parse(p).expect("a legal pattern")
    }

    fn intent() -> IntentId {
        IntentId::parse("INT-042").expect("a canonical id")
    }

    /// A repository with a seeded test, a fixture it imports, and code under
    /// test that both the test and trunk's own code import.
    fn repo() -> (MapTree, MapTree) {
        let approval = MapTree::new([
            (
                "tests/test_invoice.py",
                "# @verifies INT-042/AC-1\nimport tests.fixtures\nimport src.billing\n",
            ),
            ("tests/fixtures.py", "import tests.deep\n"),
            ("tests/deep.py", "VALUE = 1\n"),
            ("tests/__init__.py", ""),
            ("src/billing.py", "TAX = 1\n"),
            ("src/__init__.py", ""),
        ]);
        let base = MapTree::new([
            ("src/billing.py", "TAX = 1\n"),
            ("src/__init__.py", ""),
            ("src/app.py", "import src.billing\n"),
        ]);
        (approval, base)
    }

    fn inputs<'a>(approval: &'a MapTree, base: &'a MapTree, id: &'a IntentId) -> Inputs<'a> {
        Inputs {
            approval,
            base,
            c_t1: Box::leak(Box::new([pattern("tests/**")])),
            c_t2: &[],
            expected: Box::leak(Box::new([pattern("src/")])),
            intent: id,
            ac_count: 2,
        }
    }

    /// PB §4.3 clause (1): the seed is "every file under a `C-T1` test root …
    /// carrying a pragma naming an acceptance criterion of this intent".
    #[test]
    fn the_seed_is_lexical_and_this_intents_own() {
        let (approval, base) = repo();
        let id = intent();
        assert_eq!(
            seed(&inputs(&approval, &base, &id)),
            ["tests/test_invoice.py"]
        );

        // "A pragma this intent does not own seeds nothing."
        let other = IntentId::parse("INT-041").unwrap();
        assert!(seed(&inputs(&approval, &base, &other)).is_empty());

        // And a criterion the intent does not have is an occurrence, not a seed.
        let mut narrow = inputs(&approval, &base, &id);
        narrow.ac_count = 0;
        assert!(seed(&narrow).is_empty());
    }

    /// Clause (2): the transitive imports are frozen, and the walk continues
    /// through them — `tests/deep.py` is reached only through `fixtures`.
    #[test]
    fn the_closure_follows_imports_transitively() {
        let (approval, base) = repo();
        let id = intent();
        let closure = closure(&inputs(&approval, &base, &id));
        for path in [
            "tests/test_invoice.py",
            "tests/fixtures.py",
            "tests/deep.py",
        ] {
            assert!(closure.frozen.contains_key(path), "{path}: {closure:?}");
        }
    }

    /// "it resolves into a module that existed at the approval's `base=` **and
    /// was imported there by a non-test file**" — excluded, and the walk prunes.
    #[test]
    fn code_under_test_that_trunk_already_used_is_excluded() {
        let (approval, base) = repo();
        let id = intent();
        let closure = closure(&inputs(&approval, &base, &id));
        assert!(
            closure.excluded.contains("src/billing.py"),
            "trunk's own src/app.py imports it: {closure:?}"
        );
        assert!(!closure.frozen.contains_key("src/billing.py"));
    }

    /// "It is **frozen as a leaf** in exactly one case: the module existed at
    /// `base=` and no non-test file imported it there." Test-only code living
    /// at the address of code under test.
    #[test]
    fn a_module_no_non_test_file_imported_at_base_freezes_as_a_leaf() {
        let approval = MapTree::new([
            (
                "tests/test_a.py",
                "# @verifies INT-042/AC-1\nimport src.helper\n",
            ),
            ("tests/__init__.py", ""),
            ("src/helper.py", "import src.deeper\n"),
            ("src/deeper.py", "X = 1\n"),
            ("src/__init__.py", ""),
        ]);
        // At base, `helper` exists and nothing non-test imports it.
        let base = MapTree::new([
            ("src/helper.py", "import src.deeper\n"),
            ("src/deeper.py", "X = 1\n"),
            ("src/__init__.py", ""),
        ]);
        let id = intent();
        let closure = closure(&inputs(&approval, &base, &id));

        assert_eq!(
            closure.frozen.get("src/helper.py"),
            Some(&Reason::TestOnlyLeaf)
        );
        // A leaf is frozen and **not walked**: what it imports is not dragged
        // in behind it.
        assert!(
            !closure.frozen.contains_key("src/deeper.py"),
            "a leaf is not followed: {closure:?}"
        );
    }

    /// "into a module that did not exist at `base=` at all — the stub the red
    /// tests import, which the tripwire below names rather than freezes."
    #[test]
    fn a_stub_that_did_not_exist_at_base_is_excluded_and_nameable() {
        let approval = MapTree::new([
            (
                "tests/test_a.py",
                "# @verifies INT-042/AC-1\nimport src.brand_new\n",
            ),
            ("tests/__init__.py", ""),
            ("src/brand_new.py", "def f(): pass\n"),
            ("src/__init__.py", ""),
        ]);
        let base = MapTree::new([("src/__init__.py", "")]);
        let id = intent();
        let closure = closure(&inputs(&approval, &base, &id));
        assert!(closure.excluded.contains("src/brand_new.py"), "{closure:?}");
        assert!(!closure.frozen.contains_key("src/brand_new.py"));
    }

    /// "an import that resolves outside both expected and the harness is
    /// frozen as a leaf, because A had no business touching it."
    #[test]
    fn an_import_outside_both_freezes_as_a_leaf() {
        let approval = MapTree::new([
            (
                "tests/test_a.py",
                "# @verifies INT-042/AC-1\nimport vendor.thing\n",
            ),
            ("tests/__init__.py", ""),
            ("vendor/thing.py", "import vendor.deeper\n"),
            ("vendor/deeper.py", "X = 1\n"),
            ("vendor/__init__.py", ""),
        ]);
        let base = MapTree::new([("vendor/__init__.py", "")]);
        let id = intent();
        let closure = closure(&inputs(&approval, &base, &id));
        assert_eq!(
            closure.frozen.get("vendor/thing.py"),
            Some(&Reason::OutsideBothLeaf)
        );
        assert!(
            !closure.frozen.contains_key("vendor/deeper.py"),
            "a leaf is not followed"
        );
    }

    /// Both conjuncts of "inside `expected` **and outside** every harness
    /// pattern": a test file that happens to live inside `expected` is still
    /// harness, and is frozen and walked rather than excluded.
    #[test]
    fn a_harness_file_inside_expected_is_still_harness() {
        let approval = MapTree::new([
            (
                "src/tests/test_a.py",
                "# @verifies INT-042/AC-1\nimport src.tests.helper\n",
            ),
            ("src/tests/helper.py", "X = 1\n"),
            ("src/tests/__init__.py", ""),
            ("src/__init__.py", ""),
        ]);
        let base = MapTree::new([("src/__init__.py", "")]);
        let id = intent();
        let mut ins = inputs(&approval, &base, &id);
        ins.c_t1 = Box::leak(Box::new([pattern("src/tests/**")]));
        let closure = closure(&ins);
        assert_eq!(
            closure.frozen.get("src/tests/helper.py"),
            Some(&Reason::Imported),
            "inside expected but also harness: {closure:?}"
        );
    }

    /// IR §15 rule 5: the closure is a function of the tree, not of the order
    /// anything was walked.
    #[test]
    fn the_closure_is_the_same_however_the_tree_was_enumerated() {
        let (approval, base) = repo();
        let id = intent();
        let once = closure(&inputs(&approval, &base, &id));
        let twice = closure(&inputs(&approval, &base, &id));
        assert_eq!(once, twice);
    }
}
