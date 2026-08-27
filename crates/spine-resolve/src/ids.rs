//! Each runner's test-id grammar, its `id -> fn` and its `id -> path`.
//!
//! `result-file.md` §6.3's obligations 2 and 3, discharged by IR §11.2–§11.5
//! "for all four runners, and nowhere else. … There is no
//! `docs/spec/runner-adapters.md` and none is owed" (IR §11.6 rule 5).
//!
//! The ids these functions decompose are sealed into landings forever, so every
//! rule here is a byte rule: which separator, which occurrence of it, and what
//! the answer is when the tree does not agree.

use crate::lang::Lang;
use crate::lex::{self, TokenKind};
use crate::runner::{Runner, TestKey};
use core::fmt;

/// The three refusals IR §11.5 and §11.6 give a **collector**, each of which
/// makes it "fail the job and write nothing", plus §11.5's swift-testing
/// detection.
///
/// These are not per-record findings: `result-file.md` §7.3's all-or-nothing
/// rule means a job that hits one writes no `base` and no `result` records from
/// **any** runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    /// IR §11.6 rule 2: "If two distinct reported items compose to one id under
    /// any runner, the collector **fails the job and writes nothing** … This is
    /// reachable in an honest repository (two `test('x')` calls with the same
    /// name in one Dart suite compose to one id) and it is also where a forged
    /// stream event lands, so one rule serves the accident and the attack."
    DuplicateTestId { runner: Runner, id: String },
    /// IR §11.6 rule 3: "Refusing is the only fail-closed answer available —
    /// silently dropping the suite would narrow the `B` floor by exactly the
    /// tests a directory name chose, which is an attack, and guessing the split
    /// would make the same bytes name two tests."
    IdSeparatorInPath { runner: Runner, path: String },
    /// IR §11.5: "If two ids in that list share a `(class-path, method)` under
    /// different targets, the join is not single-valued on a corelibs
    /// toolchain … It is one lexical test over the list output, it is loud, and
    /// the repository's remedy is to rename a class."
    AmbiguousTestClass { class_path: String, method: String },
    /// IR §11.5: "**Any non-empty stdout makes the collector fail the job and
    /// write nothing** … a v1 adapter that ran with `--disable-swift-testing`
    /// and said nothing would silently omit from the floor exactly the tests a
    /// repository migrating to swift-testing trusts most."
    SwiftTestingUnsupported,
}

impl IdError {
    /// The bare finding token, which is what a report carries.
    pub fn token(&self) -> &'static str {
        match self {
            IdError::DuplicateTestId { .. } => "duplicate-test-id",
            IdError::IdSeparatorInPath { .. } => "id-separator-in-path",
            IdError::AmbiguousTestClass { .. } => "ambiguous-test-class",
            IdError::SwiftTestingUnsupported => "swift-testing-unsupported",
        }
    }
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // IR §11.6 rule 2: the finding carries "the runner token and the
            // id", so both are in the rendering.
            IdError::DuplicateTestId { runner, id } => {
                write!(f, "duplicate-test-id: {runner} {id}")
            }
            IdError::IdSeparatorInPath { runner, path } => {
                write!(f, "id-separator-in-path: {runner} {path}")
            }
            IdError::AmbiguousTestClass { class_path, method } => {
                write!(f, "ambiguous-test-class: {class_path}/{method}")
            }
            IdError::SwiftTestingUnsupported => f.write_str("swift-testing-unsupported"),
        }
    }
}

impl core::error::Error for IdError {}

/// `id -> fn`, `result-file.md` §6.3 obligation 2.
///
/// - **`pytest`** (IR §11.2): "split the nodeid on `::`. In the final
///   component, the parametrization suffix begins at the **first** `[` and runs
///   to the end, and exists only if the component's last byte is `]`. `fn` is
///   the nodeid with that suffix removed. Python identifiers cannot contain
///   `[`, so the split is exact and invertible."
/// - **`vitest`**, **`dart-test`**, **`swift-test`**: `fn == id`. IR §11.3
///   closes `result-file.md` §6.4's hard case by making it so: "vitest has no
///   parametrization suffix: a `test.each` case is a test whose name is its
///   own … Setting `fn == id` satisfies the prefix property trivially and has
///   exactly one consequence, which is benign: each generated case is a
///   separate `Spine-Test` entry rather than one rolled-up entry."
///
/// IR §11.6 rule 4 (the second one so numbered): "**`fn` is a prefix of `id`**,
/// checked per record by `result-file.md` §4.4."
pub fn fn_of(runner: Runner, id: &str) -> &str {
    match runner {
        Runner::Pytest => {
            let last_start = id.rfind("::").map(|i| i + 2).unwrap_or(0);
            let last = &id[last_start..];
            // "exists only if the component's last byte is `]`"
            if last.as_bytes().last() == Some(&b']')
                && let Some(open) = last.find('[')
            {
                return &id[..last_start + open];
            }
            id
        }
        Runner::Vitest | Runner::DartTest | Runner::SwiftTest => id,
    }
}

/// The lexical half of `id -> path` for the three runners whose id carries one:
/// the bytes before the **first** separator.
///
/// - `pytest` (IR §11.2): "the component before the first `::`".
/// - `vitest` (IR §11.3): "the substring before the first ` > `".
/// - `dart-test` (IR §11.4): "the bytes before the **first** `::`".
///
/// `swift-test` answers `None`: "XCTest identifies a test by class and method
/// and never by file, so obligation 3 is discharged against the tree rather
/// than against the id's own bytes" — see [`swift_id_to_path`].
pub fn path_prefix(runner: Runner, id: &str) -> Option<&str> {
    let sep = runner.path_separator()?;
    Some(match id.find(sep) {
        Some(i) => &id[..i],
        // A stream that reported an id with no separator at all names no path;
        // the whole id is the candidate and the tree lookup below will not
        // match it, which is the empty-string answer by the ordinary route.
        None => id,
    })
}

/// `id -> path`, complete: the lexical prefix "mapped onto a tree entry (of `B`
/// for a `base` record, of `T` for a `result` record) and emitted as the tree's
/// bytes; **the empty string where no entry matches**" (IR §11.4).
///
/// The empty string is the fail-closed answer everywhere in §11, and IR §11.5
/// states what it costs an id: "`result-file.md` §4.4's `G8:<path>` exemption,
/// and §12.2's pragma join".
pub fn id_to_path(runner: Runner, id: &str, exists: impl Fn(&str) -> bool) -> String {
    match path_prefix(runner, id) {
        Some(candidate) if exists(candidate) => candidate.to_string(),
        _ => String::new(),
    }
}

/// IR §11.6 rule 3, checked over the tree's own paths before any join.
///
/// `swift-test` "has no such split and is unaffected", so it can never raise
/// this.
pub fn check_separator_in_paths<'a>(
    runner: Runner,
    paths: impl IntoIterator<Item = &'a str>,
) -> Result<(), IdError> {
    let Some(sep) = runner.path_separator() else {
        return Ok(());
    };
    for path in paths {
        if path.contains(sep) {
            return Err(IdError::IdSeparatorInPath {
                runner,
                path: path.to_string(),
            });
        }
    }
    Ok(())
}

/// IR §11.6 rule 2 — the `(runner, id)` uniqueness precondition, over the keys
/// one collection produced.
///
/// The scan is over the keys in the order the collector reported them, so the
/// id named in the finding is the **second** occurrence: that is the item the
/// collector was about to write when it discovered it could not.
pub fn check_unique<'a>(keys: impl IntoIterator<Item = &'a TestKey>) -> Result<(), IdError> {
    let mut seen: Vec<&TestKey> = Vec::new();
    for key in keys {
        if seen.contains(&key) {
            return Err(IdError::DuplicateTestId {
                runner: key.runner,
                id: key.id.clone(),
            });
        }
        seen.push(key);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// dart-test — IR §11.4
// ---------------------------------------------------------------------------

/// IR §11.4's composed id: `id := sp ++ "::" ++ e.test.name`.
///
/// "**The runner emits no id.** … The adapter therefore **composes** one, and
/// the composition is fixed here rather than left to an implementer — that is
/// the whole of the divergence risk for this runner."
///
/// `test.name` "is already qualified by every enclosing group, joined with a
/// single `U+0020` … **The adapter does not re-join anything.**" (Case R1.)
pub fn dart_compose_id(suite_path: &str, test_name: &str) -> String {
    format!("{suite_path}::{test_name}")
}

/// IR §11.4 row 3: "`test.name` is `(setUpAll)` or `(tearDownAll)`, or ends
/// with `U+0020` followed by one of those — a **scaffold** test", which gets a
/// `result` record and **no** `base` record.
///
/// Why it may never enter the floor: "`package:test` … reports them **only when
/// they fail**. An id whose existence is conditioned on its own failure cannot
/// be frozen: it enters the `B` floor when the hook is broken on trunk, and the
/// moment the hook is fixed the id disappears, and absence is not a pass … That
/// is a permanent, unfixable G1 block reachable by **fixing a bug**."
pub fn dart_is_scaffold_test(test_name: &str) -> bool {
    ["(setUpAll)", "(tearDownAll)"]
        .iter()
        .any(|scaffold| test_name == *scaffold || test_name.ends_with(&format!(" {scaffold}")))
}

// ---------------------------------------------------------------------------
// swift-test — IR §11.5
// ---------------------------------------------------------------------------

/// IR §11.5's id, "the specifier line, byte for byte":
///
/// ```text
/// id := <target> "." <class-path> "/" <method>
/// ```
pub fn swift_compose_id(target: &str, class_path: &str, method: &str) -> String {
    format!("{target}.{class_path}/{method}")
}

/// The `(class-path, method)` pair an id carries — "the **case identity** the
/// adapter extracts from either spelling" of XCTest's `PrintObserver` line.
///
/// The class path is what sits between the target's `.` and the `/`, so this
/// needs the target set to know where the target name ends; [`swift_case_suffix`]
/// is the target-free form the stream join uses.
pub fn swift_case_suffix(id: &str) -> Option<(&str, &str)> {
    let slash = id.find('/')?;
    let (head, method) = (&id[..slash], &id[slash + 1..]);
    // The class path is everything after the target's own `.`. **DERIVED:** the
    // target is taken to be the first dotted component, because this function's
    // one caller — the ambiguity check over `swift test list`'s output — runs
    // before any target list is joined, and §11.5 gives it no other input. A
    // target name containing a `.` would split here in the wrong place; use
    // `swift_id_to_path`, which takes the target list and picks the longest
    // match, wherever one is available.
    let dot = head.find('.')?;
    Some((&head[dot + 1..], method))
}

/// IR §11.5's join refusal, "checked on the listing, once, before either run is
/// joined" (case R14).
pub fn check_ambiguous_test_class<'a>(
    listing: impl IntoIterator<Item = &'a str>,
) -> Result<(), IdError> {
    // The qualifier IR §11.5 puts on this check is "under different
    // **targets**", and dropping it reported the wrong token for the other
    // case: a repeated *identical* id is `duplicate-test-id` (IR §11.6 rule 2),
    // which is a different fault with a different remedy. Two implementations
    // reporting different tokens over one listing raise different wires, and a
    // reviewer's `wires=` names one of them.
    //
    // So the pair is remembered with the target it came from, and only a
    // collision **across** targets is ambiguity.
    let mut seen: Vec<(&str, (&str, &str))> = Vec::new();
    for id in listing {
        let Some(pair) = swift_case_suffix(id) else {
            continue;
        };
        let target = id.split('.').next().unwrap_or("");
        if let Some((other_target, _)) = seen
            .iter()
            .find(|(t, p)| *p == pair && *t != target)
        {
            let _ = other_target;
            return Err(IdError::AmbiguousTestClass {
                class_path: pair.0.to_string(),
                method: pair.1.to_string(),
            });
        }
        seen.push((target, pair));
    }
    Ok(())
}

/// One target of `RC(swift, tree)` as far as `id -> path` needs it: a name and
/// the target's **source files** — "`{ p ∈ F(t) : lang(p) = Swift }`", already
/// filtered by IR §7.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwiftTarget {
    pub name: String,
    pub sources: Vec<String>,
}

/// IR §11.5's `id -> path`: "a lexical declaration lookup, because the id
/// carries no path".
///
/// The four steps are §11.5's, in order. Step 4's fail-closed answer is the
/// point of the whole function: "A candidate that plants a decoy `class
/// InvoiceTests` in a second file of the same target buys itself the empty
/// string, which is strictly worse for it than doing nothing." (Case R10.)
pub fn swift_id_to_path(
    id: &str,
    targets: &[SwiftTarget],
    read: impl Fn(&str) -> Option<String>,
) -> String {
    // 1. "Let `M` be the **longest** target name in `RC(swift, X)` … such that
    //    the id begins `M ++ "."`. No such target → the empty string."
    //    Longest, because one target name may be a dotted prefix of another and
    //    the shorter would split the class path in the wrong place.
    let Some(target) = targets
        .iter()
        .filter(|t| id.starts_with(&format!("{}.", t.name)))
        .max_by_key(|t| t.name.len())
    else {
        return String::new();
    };
    // 2. "Let `C` be the bytes between that `.` and the first `/`, and `c` the
    //    last `.`-separated component of `C` — the class's own name, where `C`
    //    is a nested class path."
    let after = &id[target.name.len() + 1..];
    let Some(slash) = after.find('/') else {
        return String::new();
    };
    let class_path = &after[..slash];
    let class_name = class_path.rsplit('.').next().unwrap_or(class_path);

    // 3. "Among the source files of `M` … a file **declares** `c` iff its token
    //    stream … contains a `word` token `class` immediately followed by a
    //    `word` token equal to `c`."
    let mut declaring: Vec<&String> = Vec::new();
    for source in &target.sources {
        let Some(text) = read(source) else {
            continue;
        };
        if declares_class(&text, class_name) {
            declaring.push(source);
        }
    }
    // 4. "Exactly one such file → its path … Zero or several → **the empty
    //    string.**"
    match declaring.as_slice() {
        [one] => (*one).clone(),
        _ => String::new(),
    }
}

/// IR §11.5 step 3's two-token test. "This reaches `final class C`, `public
/// final class C`, `class C: XCTestCase` and `class C<T>` **without a
/// parser**."
///
/// Comments and string literals are discarded first, which is what stops a
/// commented-out declaration or a string containing `class C` from claiming the
/// path.
pub fn declares_class(src: &str, class_name: &str) -> bool {
    let tokens: Vec<_> = lex::lex(src, Lang::Swift)
        .into_iter()
        .filter(|t| !matches!(t.kind, TokenKind::Comment | TokenKind::Str(_)))
        .collect();
    tokens.windows(2).any(|w| {
        w[0].is_word(src, "class") && w[1].kind == TokenKind::Word && w[1].text(src) == class_name
    })
}

/// IR §12.3's **field**, per runner — the bytes the `AC<n>` naming sugar runs
/// over. The pattern is one and only the field varies: "a second spelling of
/// 'conventional position' is exactly the kind of per-runner divergence that
/// would make two implementations derive different `verified_by` edges from one
/// repository."
pub fn sugar_field(runner: Runner, id: &str) -> &str {
    match runner {
        // "the final `::`-separated component of `fn`, with the parametrization
        // suffix already removed"
        Runner::Pytest => {
            let f = fn_of(Runner::Pytest, id);
            match f.rfind("::") {
                Some(i) => &f[i + 2..],
                None => f,
            }
        }
        // "the final ` > `-separated component of `id`"
        Runner::Vitest => match id.rfind(" > ") {
            Some(i) => &id[i + 3..],
            None => id,
        },
        // "the bytes of `id` after the first `::` — the test's fully qualified
        // name, group prefixes included". Case: `group('AC3 rounding')` yields
        // AC-3 for every test in the group, which "can only make coverage
        // easier to satisfy, never harder".
        Runner::DartTest => match id.find("::") {
            Some(i) => &id[i + 2..],
            None => "",
        },
        // "the bytes of `id` after the `/` — the method name — **with a leading
        // `test` removed if present**. XCTest discovers a method only if its
        // name begins `test`, so removing that prefix is reading the runner's
        // own convention."
        Runner::SwiftTest => {
            let method = match id.find('/') {
                Some(i) => &id[i + 1..],
                None => id,
            };
            method.strip_prefix("test").unwrap_or(method)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// IR §11.2's `id → fn` for pytest, and the property `result-file.md` §4.4
    /// checks per record.
    #[test]
    fn pytest_strips_the_parametrization_suffix_at_the_first_open_bracket() {
        for (id, expected) in [
            (
                "tests/test_totals.py::test_tax[10-2]",
                "tests/test_totals.py::test_tax",
            ),
            // "the parametrization suffix begins at the **first** `[`"
            (
                "tests/t.py::test_x[a[b]]",
                "tests/t.py::test_x",
            ),
            // "and exists only if the component's last byte is `]`"
            ("tests/t.py::test_x[a]b", "tests/t.py::test_x[a]b"),
            ("tests/t.py::TestClass::test_x", "tests/t.py::TestClass::test_x"),
            ("tests/t.py::test_x", "tests/t.py::test_x"),
            // A `[` in an earlier component belongs to no suffix: only the
            // final component is inspected.
            ("tests/a[1]/t.py::test_x", "tests/a[1]/t.py::test_x"),
        ] {
            assert_eq!(fn_of(Runner::Pytest, id), expected, "{id}");
        }
    }

    /// Case R15: "Any adapter's `fn` | a prefix of its `id`; equal to it for
    /// `vitest`, `dart-test` and `swift-test`."
    #[test]
    fn fn_is_a_prefix_of_id_for_every_adapter_and_equal_for_three() {
        let cases = [
            (Runner::Pytest, "tests/t.py::test_x[1]"),
            (Runner::Vitest, "src/a.test.ts > suite > case"),
            (Runner::DartTest, "test/a_test.dart::g t"),
            (Runner::SwiftTest, "Billing.InvoiceTests/testTax"),
        ];
        for (runner, id) in cases {
            let f = fn_of(runner, id);
            assert!(id.starts_with(f), "{runner}: {f} is not a prefix of {id}");
            if runner != Runner::Pytest {
                assert_eq!(f, id, "{runner} must have fn == id");
            }
        }
    }

    /// IR §11.2–§11.4's `id → path` prefixes, each at the **first** separator.
    #[test]
    fn id_to_path_splits_at_the_first_separator_only() {
        assert_eq!(
            path_prefix(Runner::Pytest, "tests/t.py::TestC::test_x"),
            Some("tests/t.py")
        );
        assert_eq!(
            path_prefix(Runner::Vitest, "src/a.test.ts > outer > inner > case"),
            Some("src/a.test.ts")
        );
        assert_eq!(
            path_prefix(Runner::DartTest, "test/a_test.dart::g::t"),
            Some("test/a_test.dart")
        );
        // IR §11.5: swift-test's id carries no path at all.
        assert_eq!(path_prefix(Runner::SwiftTest, "M.C/testX"), None);
    }

    /// "the empty string where no tree entry matches" — the fail-closed answer
    /// §11 uses everywhere.
    #[test]
    fn id_to_path_is_the_empty_string_when_the_tree_has_no_such_entry() {
        let tree = ["tests/t.py"];
        let exists = |p: &str| tree.contains(&p);
        assert_eq!(
            id_to_path(Runner::Pytest, "tests/t.py::test_x", exists),
            "tests/t.py"
        );
        assert_eq!(id_to_path(Runner::Pytest, "gone/t.py::test_x", exists), "");
        assert_eq!(id_to_path(Runner::SwiftTest, "M.C/testX", exists), "");
    }

    /// Case R9: "A repository path containing `::`, under `pytest` or
    /// `dart-test` | `id-separator-in-path`; the collector fails the job and
    /// writes nothing."
    #[test]
    fn a_repository_path_carrying_the_separator_fails_the_job() {
        let err = check_separator_in_paths(Runner::Pytest, ["tests/we::ird/t.py"]).unwrap_err();
        assert_eq!(err.token(), "id-separator-in-path");
        assert!(check_separator_in_paths(Runner::Pytest, ["tests/t.py"]).is_ok());

        let err = check_separator_in_paths(Runner::Vitest, ["src/a > b.ts"]).unwrap_err();
        assert_eq!(err.token(), "id-separator-in-path");
        // A `::` is harmless under vitest, whose separator is ` > `.
        assert!(check_separator_in_paths(Runner::Vitest, ["src/a::b.ts"]).is_ok());

        // "swift-test has no such split and is unaffected."
        assert!(check_separator_in_paths(Runner::SwiftTest, ["Sources/a::b.swift"]).is_ok());
    }

    /// Case R7: "Two `test('x')` calls with the same name in one `dart-test`
    /// suite | `duplicate-test-id`; the collector fails the job and writes
    /// nothing." Reproduced through the composition that makes it happen.
    #[test]
    fn two_dart_tests_of_one_name_in_one_suite_compose_to_one_id_and_refuse() {
        let a = dart_compose_id("test/a_test.dart", "x");
        let b = dart_compose_id("test/a_test.dart", "x");
        assert_eq!(a, b, "the composition is what makes them collide");
        let keys = [
            TestKey::new(Runner::DartTest, a),
            TestKey::new(Runner::DartTest, b),
        ];
        let err = check_unique(&keys).unwrap_err();
        assert_eq!(err.token(), "duplicate-test-id");
        assert_eq!(
            err.to_string(),
            "duplicate-test-id: dart-test test/a_test.dart::x"
        );
    }

    /// The same id under two different runners is two records, not a
    /// duplicate — `result-file.md` §4.4's key is the **pair**.
    #[test]
    fn the_uniqueness_key_is_the_pair_and_not_the_id_alone() {
        let keys = [
            TestKey::new(Runner::Pytest, "a::b"),
            TestKey::new(Runner::DartTest, "a::b"),
        ];
        assert!(check_unique(&keys).is_ok());
    }

    /// Case R1: "one id, `test/a_test.dart::g t` — the group prefix comes from
    /// `test.name`; **must not** be re-joined by the adapter."
    #[test]
    fn r1_the_dart_id_is_the_suite_path_the_separator_and_the_reported_name() {
        assert_eq!(
            dart_compose_id("test/a_test.dart", "g t"),
            "test/a_test.dart::g t"
        );
    }

    /// Case R6: "A `dart-test` test named `outer (tearDownAll)` | a `result`
    /// record; **no** `base` record."
    #[test]
    fn dart_scaffold_tests_are_recognised_by_name_and_by_suffix() {
        assert!(dart_is_scaffold_test("(setUpAll)"));
        assert!(dart_is_scaffold_test("(tearDownAll)"));
        assert!(dart_is_scaffold_test("outer (tearDownAll)"));
        assert!(!dart_is_scaffold_test("outer(tearDownAll)"));
        assert!(!dart_is_scaffold_test("setUpAll"));
        assert!(!dart_is_scaffold_test("a (setUpAll) b"));
    }

    /// IR §11.5's id spelling, and case R14's refusal over the listing.
    #[test]
    fn r14_two_swift_ids_sharing_a_class_and_method_under_different_targets_refuse() {
        let a = swift_compose_id("Billing", "InvoiceTests", "testTax");
        let b = swift_compose_id("Shipping", "InvoiceTests", "testTax");
        assert_eq!(a, "Billing.InvoiceTests/testTax");
        let err = check_ambiguous_test_class([a.as_str(), b.as_str()]).unwrap_err();
        assert_eq!(err.token(), "ambiguous-test-class");
        assert_eq!(
            err.to_string(),
            "ambiguous-test-class: InvoiceTests/testTax"
        );

        // Two methods of one class are not ambiguous.
        let c = swift_compose_id("Billing", "InvoiceTests", "testRounding");
        assert!(check_ambiguous_test_class([a.as_str(), c.as_str()]).is_ok());
    }

    /// IR §11.5 step 3, "without a parser".
    #[test]
    fn the_class_declaration_test_reaches_every_shape_the_spec_names() {
        for src in [
            "final class C {}",
            "public final class C: XCTestCase {}",
            "class C: XCTestCase {}",
            "class C<T> {}",
            "@MainActor class C {}",
        ] {
            assert!(declares_class(src, "C"), "{src}");
        }
        for src in [
            "// class C {}",
            "/* class C */",
            "let s = \"class C\"",
            "class CC {}",
            "classC {}",
            "struct C {}",
        ] {
            assert!(!declares_class(src, "C"), "{src}");
        }
    }

    /// Case R10: "A `swift-test` id `M.C/testX` where two files of target `M`
    /// both declare `class C` | `path` is the empty string; **must not** pick
    /// either."
    #[test]
    fn r10_two_files_declaring_the_class_yield_the_empty_string() {
        let targets = [SwiftTarget {
            name: "M".into(),
            sources: vec![
                "Tests/M/A.swift".into(),
                "Tests/M/Decoy.swift".into(),
                "Tests/M/Other.swift".into(),
            ],
        }];
        let read = |p: &str| {
            Some(
                match p {
                    "Tests/M/A.swift" => "final class C: XCTestCase { func testX() {} }",
                    "Tests/M/Decoy.swift" => "class C {}",
                    _ => "class Unrelated {}",
                }
                .to_string(),
            )
        };
        assert_eq!(swift_id_to_path("M.C/testX", &targets, read), "");

        // With only the real declaration, the path is that file's.
        let read_one = |p: &str| {
            Some(
                match p {
                    "Tests/M/A.swift" => "final class C: XCTestCase { func testX() {} }",
                    _ => "class Unrelated {}",
                }
                .to_string(),
            )
        };
        assert_eq!(
            swift_id_to_path("M.C/testX", &targets, read_one),
            "Tests/M/A.swift"
        );
    }

    /// IR §11.5 step 1: the **longest** matching target name wins, and an id
    /// under no target is the empty string.
    #[test]
    fn the_longest_target_name_prefix_decides_where_the_class_path_starts() {
        let targets = [
            SwiftTarget {
                name: "App".into(),
                sources: vec!["Sources/App/A.swift".into()],
            },
            SwiftTarget {
                name: "App.Billing".into(),
                sources: vec!["Sources/AppBilling/B.swift".into()],
            },
        ];
        let read = |p: &str| {
            Some(
                match p {
                    "Sources/AppBilling/B.swift" => "class Tests {}",
                    _ => "class Billing {}",
                }
                .to_string(),
            )
        };
        assert_eq!(
            swift_id_to_path("App.Billing.Tests/testX", &targets, read),
            "Sources/AppBilling/B.swift"
        );
        assert_eq!(swift_id_to_path("Unknown.C/testX", &targets, read), "");
    }

    /// IR §11.5 step 2: "`c` the last `.`-separated component of `C` — the
    /// class's own name, where `C` is a nested class path."
    #[test]
    fn a_nested_class_path_declares_only_its_last_component() {
        let targets = [SwiftTarget {
            name: "M".into(),
            sources: vec!["Tests/M/A.swift".into()],
        }];
        let read = |_: &str| Some("class Outer { class Inner: XCTestCase {} }".to_string());
        assert_eq!(
            swift_id_to_path("M.Outer.Inner/testX", &targets, read),
            "Tests/M/A.swift"
        );
    }

    /// IR §12.3's field table, one row each.
    #[test]
    fn ir_12_3_the_sugar_field_per_runner() {
        assert_eq!(
            sugar_field(Runner::Pytest, "tests/t.py::TestC::test_AC1_totals[3]"),
            "test_AC1_totals"
        );
        assert_eq!(
            sugar_field(Runner::Vitest, "src/a.test.ts > outer > AC2 totals"),
            "AC2 totals"
        );
        assert_eq!(
            sugar_field(Runner::DartTest, "test/a_test.dart::AC3 rounding half even"),
            "AC3 rounding half even"
        );
        // "with a leading `test` removed if present": `testAC1TotalsIncludeTax`
        // gives `AC1TotalsIncludeTax`, "in which `AC1` is at the field's start".
        assert_eq!(
            sugar_field(Runner::SwiftTest, "Billing.InvoiceTests/testAC1TotalsIncludeTax"),
            "AC1TotalsIncludeTax"
        );
        assert_eq!(
            sugar_field(Runner::SwiftTest, "Billing.InvoiceTests/test_AC1_totals"),
            "_AC1_totals"
        );
    }

    /// IR §11.5 qualifies this check "under different **targets**". Without the
    /// qualifier a repeated identical id reported `ambiguous-test-class`, when
    /// IR §11.6 rule 2 makes it `duplicate-test-id` — a different fault with a
    /// different remedy, and a different wire for a reviewer to sign.
    #[test]
    fn ambiguity_is_across_targets_and_a_repeat_within_one_is_not() {
        // Same class and method under two targets: ambiguous.
        assert!(matches!(
            check_ambiguous_test_class(["ModuleA.C/testX", "ModuleB.C/testX"]),
            Err(IdError::AmbiguousTestClass { .. })
        ));

        // The identical id twice, under one target: not this check's fault.
        assert!(
            check_ambiguous_test_class(["ModuleA.C/testX", "ModuleA.C/testX"]).is_ok(),
            "a repeated identical id is duplicate-test-id, not ambiguous-test-class"
        );

        // Different methods under one class, and different classes, are fine.
        assert!(
            check_ambiguous_test_class(["ModuleA.C/testX", "ModuleA.C/testY", "ModuleA.D/testX"])
                .is_ok()
        );
    }
}
