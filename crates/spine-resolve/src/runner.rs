//! The four `runner` tokens, what each adapter invokes, and the tokens that
//! stay reserved.
//!
//! IR §11: "a `runner` token and an id grammar are sealed into landings forever
//! (`result-file.md` §6.3 obligation 1), and two implementations that disagree
//! on one reject each other's landings rather than merely differing."
//!
//! IR §11.6 rule 1: "**The `runner` token is a constant of the adapter**
//! (`result-file.md` §4.4): never read from a stream, a manifest,
//! `params.langs` or the environment." That is why [`Runner::token`] is `const`
//! and why nothing in this module takes a token from an input to decide one.

use crate::lang::Lang;
use core::fmt;

/// IR §11.1's four ratified `runner` tokens. "The mapping is 1:1 in v1 — one
/// adapter per language."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Runner {
    Pytest,
    Vitest,
    DartTest,
    SwiftTest,
}

impl Runner {
    /// The `runner` token. "These are the values `result-file.md` §4.4's token
    /// grammar carries, the values `Spine-Test` lines are sealed with, and the
    /// values `test:<runner>:<id>` node ids use (PB §6.2). **They are
    /// permanent.**"
    pub const fn token(self) -> &'static str {
        match self {
            Runner::Pytest => "pytest",
            Runner::Vitest => "vitest",
            Runner::DartTest => "dart-test",
            Runner::SwiftTest => "swift-test",
        }
    }

    /// IR §11.1's first column. "`result-file.md` §6.2's invocation set is
    /// exactly the image of trunk's `params.langs` under this table."
    pub const fn for_lang(lang: Lang) -> Runner {
        match lang {
            Lang::Python => Runner::Pytest,
            Lang::Ts => Runner::Vitest,
            Lang::Dart => Runner::DartTest,
            Lang::Swift => Runner::SwiftTest,
        }
    }

    /// The language whose files this adapter's ids point into.
    pub const fn lang(self) -> Lang {
        match self {
            Runner::Pytest => Lang::Python,
            Runner::Vitest => Lang::Ts,
            Runner::DartTest => Lang::Dart,
            Runner::SwiftTest => Lang::Swift,
        }
    }

    /// The closed token set. **Every reserved token answers `None`** — that is
    /// the whole point of reserving one, and a `from_token` that resolved
    /// `gradle` or `jest` to an adapter would spend a string a later release
    /// needs (IR §11.1: "Reserving costs nothing and prevents the one mistake
    /// that cannot be undone: a later release finding its natural token already
    /// spent.").
    pub fn from_token(token: &str) -> Option<Runner> {
        match token {
            "pytest" => Some(Runner::Pytest),
            "vitest" => Some(Runner::Vitest),
            "dart-test" => Some(Runner::DartTest),
            "swift-test" => Some(Runner::SwiftTest),
            _ => None,
        }
    }

    pub const ALL: [Runner; 4] = [
        Runner::Pytest,
        Runner::Vitest,
        Runner::DartTest,
        Runner::SwiftTest,
    ];

    /// IR §11.1, "Invocation on `T`". Every invocation "runs at the
    /// **repository root** with no selection argument of any kind
    /// (`result-file.md` §7.2)".
    pub const fn invocation(self) -> &'static [&'static str] {
        match self {
            Runner::Pytest => &["pytest"],
            Runner::Vitest => &["vitest", "run"],
            // `--no-retry` is **mandatory** (IR §11.4): "a retried test is
            // reported as a fresh `testStart` under the same name, which
            // composes to the same `id` and would make the file's
            // `(runner, id)` pair non-unique — malformed under
            // `result-file.md` §4.4."
            Runner::DartTest => &["dart", "test", "--reporter=json", "--no-retry"],
            // `--parallel` is **never** passed (IR §11.5): several `xctest`
            // processes onto one stream make a per-case line unattributable,
            // "and the multiplicity rule below cannot tell a second process
            // from a forgery".
            Runner::SwiftTest => &["swift", "test", "--disable-swift-testing"],
        }
    }

    /// IR §11.1, "`B` enumeration — what the floor's membership is taken from".
    ///
    /// "**The rule the four share, stated once so that four adapters cannot
    /// drift apart:** the `B` floor is the set of ids the runner **collected
    /// and selected** on the checkout of `B` — every id it enumerated, less any
    /// it reported as *deselected*, and **irrespective of outcome**."
    pub const fn base_enumeration(self) -> &'static [&'static str] {
        match self {
            // "`--collect-only` is **not** a selection argument in the sense
            // `result-file.md` §7.2 forbids: it narrows no test set and skips
            // nothing, it runs the collection phase and stops before the first
            // `call`."
            Runner::Pytest => &["pytest", "--collect-only"],
            // IR §11.3 refuses `vitest list` for the one truncation that
            // weakens the gate: it "omits every skipped test and shrinks the
            // floor" (case R17).
            Runner::Vitest => &["vitest", "run"],
            // "`dart test` has no list-only mode … there is no dry run."
            Runner::DartTest => &["dart", "test", "--reporter=json", "--no-retry"],
            Runner::SwiftTest => &["swift", "test", "list", "--disable-swift-testing"],
        }
    }

    /// IR §11.1, "`B` outcomes — what each `base` record's `out` is taken
    /// from". IR §11.6 rule 4: "**The `B` outcome is the adapter's own mapping,
    /// applied to `B`.** … No adapter defines a second mapping, a `B`-only
    /// value or a `B`-only refusal, and none may."
    ///
    /// For `pytest` and `swift-test` this is a **second** full run of the
    /// repository's suite on every landing. IR §11.1 states the cost rather
    /// than burying it: "That is the price of PB §6.3's `xfail`/`skipped`
    /// exemption and there is no cheaper way to it."
    pub const fn base_outcome_run(self) -> &'static [&'static str] {
        // "The `B` outcome run is the adapter's own `T` invocation, byte for
        // byte, executed against a checkout of `B`." For vitest and dart-test
        // that is the very run the enumeration already used.
        self.invocation()
    }

    /// IR §11.1's last column: **two** for `pytest` and `swift-test`, one for
    /// `vitest` and `dart-test`.
    ///
    /// The asymmetry is a fact about the runners: "pytest has a collection
    /// phase it can be asked to stop after, and `dart test` has no list-only
    /// mode at all", while `pytest --collect-only` and `swift test list` report
    /// no outcome at all and so cannot supply `out`.
    pub const fn base_invocations(self) -> u8 {
        match self {
            Runner::Pytest | Runner::SwiftTest => 2,
            Runner::Vitest | Runner::DartTest => 1,
        }
    }

    /// IR §11.6 rule 3's `<sep>`: "Where an adapter's `id → path` is 'the bytes
    /// before the first `<sep>`' …". `swift-test` has none — its id carries no
    /// path at all and its `id → path` is a declaration lookup (IR §11.5).
    pub const fn path_separator(self) -> Option<&'static str> {
        match self {
            Runner::Pytest | Runner::DartTest => Some("::"),
            Runner::Vitest => Some(" > "),
            Runner::SwiftTest => None,
        }
    }

    /// IR §11.6 rule 5: `xfail` "is producible by two of the four adapters".
    /// `pytest` produces it from the expected-failure marker and `swift-test`
    /// from an `Expected failure in` line; "`vitest` and `dart-test` have no
    /// expected-failure value in their mappings at all".
    ///
    /// This is not cosmetic: `result-file.md` §8.5 clause 2's carve-out reads
    /// `b.out` against the two literals `xfail` and `skipped`, so an adapter
    /// that claimed to produce a value it cannot would make the carve-out mean
    /// one thing on trunk and another on the branch.
    pub const fn produces_xfail(self) -> bool {
        matches!(self, Runner::Pytest | Runner::SwiftTest)
    }
}

impl fmt::Display for Runner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// IR §11.1's reserved table, hard set first.
///
/// "**The hard set is `kotlin`, `gradle`, `jest`.** Those three are emitted by
/// nothing in v1 and no other adapter may take them (§20, item 26)." `kotlin`
/// is a `params.langs` value rather than a `runner` token and lives in
/// [`crate::lang::RESERVED_LANG_TOKENS`]; the two runner tokens are here.
///
/// `junit` and `kotest` join them: IR §11.1 files them as **contested** —
/// "`result-file.md` §6.4 reserves both; version 1 and version 2 of this
/// document reserved neither" — and §18 OPEN-12 asks the owner "for one word
/// rather than for an argument". The owner's word is that they are reserved,
/// which costs nothing today (no v1 release emits any of them) and costs a
/// permanent token later if they are not.
pub const RESERVED_RUNNER_TOKENS: [&str; 4] = ["gradle", "jest", "junit", "kotest"];

/// IR §11.1: `swift-testing` is "**not reserved; reservation recommended**" —
/// "§18 OPEN-8 recommends reserving it now and it is the owner's call, not this
/// document's."
///
/// It is listed separately rather than folded into [`RESERVED_RUNNER_TOKENS`]
/// because the two states are different facts, and because a v1 release emits
/// neither: §11.5's adapter **detects** swift-testing and fails the job with
/// `swift-testing-unsupported` rather than ignoring it.
pub const RESERVATION_RECOMMENDED_RUNNER_TOKENS: [&str; 1] = ["swift-testing"];

/// The **identity pair** `result-file.md` §4.4 makes unique per result file, and
/// the pair PB §6.2's `test:<runner>:<id>` node id is built from.
///
/// IR §11.6 rule 2: "**`(runner, id)` uniqueness is a collector precondition,
/// not a hope.** … If two distinct reported items compose to one id under any
/// runner, the collector **fails the job and writes nothing** — finding
/// `duplicate-test-id`." Case R7 shows it is reachable in an honest repository:
/// "two `test('x')` calls with the same name in one Dart suite compose to one
/// id".
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TestKey {
    pub runner: Runner,
    pub id: String,
}

impl TestKey {
    pub fn new(runner: Runner, id: impl Into<String>) -> Self {
        TestKey {
            runner,
            id: id.into(),
        }
    }

    /// PB §6.2's node id for this test.
    pub fn node_id(&self) -> String {
        format!("test:{}:{}", self.runner.token(), self.id)
    }
}

impl fmt::Display for TestKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.runner.token(), self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// IR §11.1's table, first two columns.
    #[test]
    fn ir_11_1_the_four_tokens_and_their_languages() {
        for (lang, token) in [
            (Lang::Python, "pytest"),
            (Lang::Ts, "vitest"),
            (Lang::Dart, "dart-test"),
            (Lang::Swift, "swift-test"),
        ] {
            let runner = Runner::for_lang(lang);
            assert_eq!(runner.token(), token, "{lang}");
            assert_eq!(runner.lang(), lang, "the mapping is 1:1");
            assert_eq!(Runner::from_token(token), Some(runner));
        }
    }

    /// IR §11.1's table, columns 3 to 6, byte for byte. A wrong argument here
    /// is a different floor.
    #[test]
    fn ir_11_1_every_invocation_is_the_ratified_argv() {
        assert_eq!(Runner::Pytest.invocation(), ["pytest"]);
        assert_eq!(
            Runner::Pytest.base_enumeration(),
            ["pytest", "--collect-only"]
        );
        assert_eq!(Runner::Pytest.base_outcome_run(), ["pytest"]);

        assert_eq!(Runner::Vitest.invocation(), ["vitest", "run"]);
        assert_eq!(Runner::Vitest.base_enumeration(), ["vitest", "run"]);

        assert_eq!(
            Runner::DartTest.invocation(),
            ["dart", "test", "--reporter=json", "--no-retry"]
        );
        assert_eq!(
            Runner::DartTest.base_enumeration(),
            ["dart", "test", "--reporter=json", "--no-retry"]
        );

        assert_eq!(
            Runner::SwiftTest.invocation(),
            ["swift", "test", "--disable-swift-testing"]
        );
        assert_eq!(
            Runner::SwiftTest.base_enumeration(),
            ["swift", "test", "list", "--disable-swift-testing"]
        );
        assert_eq!(
            Runner::SwiftTest.base_outcome_run(),
            ["swift", "test", "--disable-swift-testing"]
        );
    }

    /// IR §11.6 rule 4 and case R13a: the `B` outcome run is the `T` invocation
    /// "unchanged and with no selection argument", byte for byte. A second
    /// argv here would be a second mapping by another name.
    #[test]
    fn the_base_outcome_run_is_the_t_invocation_byte_for_byte() {
        for runner in Runner::ALL {
            assert_eq!(
                runner.base_outcome_run(),
                runner.invocation(),
                "{runner} must not invent a B-only command"
            );
        }
    }

    /// IR §11.1's last column, and §11.1's own summary of why: pytest and
    /// swift-test "obtain the floor from a cheap listing that reports no
    /// outcome at all", so each pays "one more full run of the repository's
    /// suite against `B`, on every landing".
    #[test]
    fn two_runners_pay_for_two_base_invocations_and_two_pay_for_one() {
        assert_eq!(Runner::Pytest.base_invocations(), 2);
        assert_eq!(Runner::SwiftTest.base_invocations(), 2);
        assert_eq!(Runner::Vitest.base_invocations(), 1);
        assert_eq!(Runner::DartTest.base_invocations(), 1);

        // The one-invocation runners are exactly those whose enumeration and
        // outcome run are the same command; the two-invocation ones are exactly
        // those whose enumeration is a cheaper command that reports no outcome.
        for runner in Runner::ALL {
            let same = runner.base_enumeration() == runner.base_outcome_run();
            assert_eq!(same, runner.base_invocations() == 1, "{runner}");
        }
    }

    /// IR §11.1 and case R17a. `vitest` and `dart-test` "have no
    /// expected-failure value in their mappings at all", so a `base` record
    /// from either can never carry `xfail`.
    #[test]
    fn only_pytest_and_swift_test_can_ever_produce_xfail() {
        assert!(Runner::Pytest.produces_xfail());
        assert!(Runner::SwiftTest.produces_xfail());
        assert!(!Runner::Vitest.produces_xfail());
        assert!(!Runner::DartTest.produces_xfail());

        // And the pair that produces `xfail` is exactly the pair that pays for
        // a second `B` invocation — IR §11.6 rule 5: "an implementer who reads
        // §11.1's cost table and wonders whether the cheap half could be
        // skipped for the expensive runners has it backwards."
        for runner in Runner::ALL {
            assert_eq!(
                runner.produces_xfail(),
                runner.base_invocations() == 2,
                "{runner}"
            );
        }
    }

    /// IR §11.1's reserved table. **The whole content of a reservation is that
    /// the token resolves to nothing**, so this is the test that enforces it.
    #[test]
    fn every_reserved_runner_token_is_assigned_to_no_adapter() {
        for token in RESERVED_RUNNER_TOKENS {
            assert_eq!(
                Runner::from_token(token),
                None,
                "{token} is reserved and must stay unspent"
            );
            assert!(
                !Runner::ALL.iter().any(|r| r.token() == token),
                "{token} must be no adapter's own token"
            );
        }
        assert_eq!(
            RESERVED_RUNNER_TOKENS,
            ["gradle", "jest", "junit", "kotest"]
        );
    }

    /// IR §11.1: `swift-testing` is not reserved, and §11.5 detects it and
    /// fails the job rather than ignoring it. It resolves to no adapter either
    /// way, which is what this pins.
    #[test]
    fn swift_testing_is_recommended_for_reservation_and_names_no_adapter() {
        assert_eq!(RESERVATION_RECOMMENDED_RUNNER_TOKENS, ["swift-testing"]);
        assert_eq!(Runner::from_token("swift-testing"), None);
        // Distinct from the shipped adapter's token, which it is one byte from.
        assert_ne!(Runner::SwiftTest.token(), "swift-testing");
    }

    /// IR §11.6 rule 3's `<sep>`: "`::` for `pytest` and `dart-test` and ` > `
    /// for `vitest`; `swift-test` has no such split and is unaffected."
    #[test]
    fn the_path_separator_is_per_runner_and_swift_test_has_none() {
        assert_eq!(Runner::Pytest.path_separator(), Some("::"));
        assert_eq!(Runner::DartTest.path_separator(), Some("::"));
        assert_eq!(Runner::Vitest.path_separator(), Some(" > "));
        assert_eq!(Runner::SwiftTest.path_separator(), None);
    }

    /// PB §6.2's node id spelling.
    #[test]
    fn a_test_key_spells_pb_6_2s_node_id() {
        let key = TestKey::new(Runner::Pytest, "tests/test_totals.py::test_tax");
        assert_eq!(key.node_id(), "test:pytest:tests/test_totals.py::test_tax");
    }

    /// The four tokens are distinct and nothing else resolves.
    #[test]
    fn nothing_outside_the_closed_set_resolves_to_a_runner() {
        for token in ["", "Pytest", "dart", "swift", "dart_test", "vitest run"] {
            assert_eq!(Runner::from_token(token), None, "{token:?}");
        }
    }
}
