//! `spine-resolve` — the per-language reads that turn a repository's source
//! bytes into the identities the gates compare.
//!
//! Five things live here, and they are five because `import-resolver.md` §12
//! puts them in one place for one reason: "both are per-language lexical reads
//! of the files the resolver already lexes, over tokens §3.4 already produces."
//!
//! | Module | What it fixes | Where |
//! |---|---|---|
//! | [`glob`] | the one path-pattern dialect every list is matched with | ID §6.1–§6.3, adopted by IR §2.4 |
//! | [`lang`] | `lang(path)`, the four `params.langs` tokens, the refusal vocabularies | IR §3.1, §3.8, §4.7, §5.7, §6.7, §7.8 |
//! | [`runner`] | the four `runner` tokens, what each adapter invokes, and what stays reserved | IR §11.1, §11.6 |
//! | [`ids`] | each runner's test-id grammar, `id -> fn` and `id -> path` | IR §11.2–§11.6 |
//! | [`pragma`] | `@verifies`, the file-granular join, the `AC<n>` naming sugar | IR §12.1–§12.3 |
//! | [`lex`] | the shared lexical preliminaries and the four lexers | IR §3.4, §4.1, §5.1, §6.1, §7.1 |
//! | [`site`] | an import site and its four dispositions | IR §3.2 |
//! | [`tree`] | the tree a resolver reads, and the two entry kinds it may not follow | IR §2.12 rules 1–2 |
//! | [`python`] | Python's roots, dotted resolution and dynamic constructs | IR §4 |
//! | [`ts`] | `RC(ts, ·)`, the alias table and the ordered candidate list | IR §5 |
//! | [`dart`] | `RC(dart, ·)`, URI schemes and the library-name index | IR §6 |
//! | [`swift`] | `RC(swift, ·)`, `mixed-objc-target`, and §7.4's module-shaped imports | IR §7 |
//! | [`jsonc`] | JSON with comments and trailing commas, for `tsconfig.json` | IR §5.3 step 1 |
//! | [`yaml`] | the declarative YAML subset, for `pubspec.yaml` | IR §6.3 step 2 |
//!
//! **Why these are worth this much care.** IR §11: "a `runner` token and an id
//! grammar are sealed into landings forever … and two implementations that
//! disagree on one reject each other's landings rather than merely differing."
//! The same is true of the pattern dialect: IR §2.4.1 records a shipped `C-T1`
//! value that matched **nothing** under a rival dialect, so `--approve` refused
//! outright on every repository that used it, and "nothing in the dialect says
//! a pattern that matches nothing is suspicious."
//!
//! **Kotlin is not here and is not coming back in v1.** PB §6.7's rule — an
//! oracle in a `.java` file inside a mixed Kotlin/Java module is invisible to a
//! Kotlin resolver and nothing reports the miss — dropped the language (IR §8,
//! §18 OPEN-1). The same rule keeps Swift and refuses one Swift shape:
//! `mixed-objc-target` (IR §7.3). The tokens `kotlin` and `gradle` stay
//! reserved and are assigned to nothing.

pub mod closure;
pub mod dart;
pub mod glob;
pub mod ids;
pub mod jsonc;
pub mod lang;
pub mod lex;
pub mod pragma;
pub mod python;
pub mod runner;
pub mod site;
pub mod swift;
pub mod tree;
pub mod ts;
pub mod yaml;

pub use glob::{Pattern, PatternError};
pub use ids::IdError;
pub use lang::lang as lang_of;
pub use lang::{FileNotUtf8, Lang, LangUnclassifiable, Unresolvable, lang};
pub use lex::{Token, TokenKind, lex};
pub use pragma::{AcNumber, IntentId, Occurrence};
pub use runner::{Runner, TestKey};
pub use site::{Disposition, ImportSite};
pub use tree::{Entry, EntryKind, MapTree, Tree};
