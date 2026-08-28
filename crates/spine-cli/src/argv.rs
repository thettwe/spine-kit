//! Argument parsing for the four commands PB §11 fixes.
//!
//! Hand-rolled rather than delegated to a parser library, for one reason: the
//! spec fixes exit codes and refusal tokens, and a library that decides for
//! itself when to print usage and exit 2 would be making decisions the corpus
//! has already made. Every refusal here is one a document names.
//!
//! PB §11's signature, verbatim as it now stands (`--strategy` was removed by
//! the owner's ruling of 2026-08-27 — it had no conforming write target):
//!
//! ```text
//! spine init [--ci github|gitlab|generic] [--langs <l>[,<l>…]]
//!            [--isolation container|none] [--trunk <name>] [--signer-key <pub>]
//!            [--identity <principal>] [--pipeline-key <pub>] [--hooks]
//!            [--trust-root <sha>] [--rotate-trust-root] [--dry-run] [--status]
//!            [--merge] [--adopt <path|file#region>] [--force <path>] [--abort]
//!            [--rollback [<sha>]] [--uninstall]
//!
//! spine check [--ci] [--collect] [--constitution] [--authority]
//!             [--approve <id>] [--review [<id> | --quick <branch> | --reseal]]
//!             [--land [<id> | --quick <branch> | --reseal] [--print] [--dry-run]]
//!             [--report <path>] [--reconstruct] [--verify <sha>]
//!             [--break-glass "<reason>"] [--pre-receive]
//! ```

use core::fmt;

/// PB §11's four commands. Everything else is roadmap 3+, not v1.
///
/// `Init` is boxed because it carries eighteen fields and the other three
/// variants carry almost nothing; without the box every `Command` is as large
/// as its largest variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Init(Box<Init>),
    New,
    Index { fresh: bool, dump: bool },
    Check(Box<Check>),
}

/// What `spine check` was asked to do.
///
/// **`--land`, `--review` and `--approve` take a *subject*, and the three
/// spellings are not interchangeable.** PB §11: `--land [<id> | --quick
/// <branch> | --reseal]`, with "`<id>` omitted for upgrade landings" — so a
/// bare `--land` is a fourth, legal subject and not a missing argument.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Check {
    /// "the untrusted job passes both, a solo developer passes `--collect`
    /// alone … and nesting it inside `--ci` would leave the solo path with no
    /// legal invocation."
    pub ci: bool,
    pub collect: bool,
    pub constitution: bool,
    pub authority: bool,
    pub approve: Option<String>,
    pub review: Option<Subject>,
    pub land: Option<Subject>,
    /// Only with `--land`. "`--print` emits a sealed envelope only for a run
    /// that would have landed."
    pub print: bool,
    /// Only with `--land`. "`--dry-run` never signs."
    pub dry_run: bool,
    /// "`--land --ci` **always** writes the canonical gate report to
    /// `.spine/cache/report.json`; `--report <path>` overrides the
    /// destination."
    pub report: Option<String>,
    pub reconstruct: bool,
    pub verify: Option<String>,
    pub break_glass: Option<String>,
    pub pre_receive: bool,
}

/// The subject of a `--land` or `--review`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    /// A gated landing or review: `--land INT-042`.
    Intent(String),
    /// `--land --quick <branch>` (PB §5.2).
    Quick(String),
    /// `--land --reseal` (PB §5.5).
    Reseal,
    /// "`<id>` omitted for upgrade landings" — the toolkit lifecycle landings
    /// of PB §6.7, which ride the quick lane with no intent id.
    Upgrade,
}

impl Check {
    /// PB §7.1: "any invocation that produces a `-Sig` line with a key that is
    /// not the `--ci` pipeline secret — `--sign`, `--reopen`, `--withdraw`,
    /// `--approve`, `--review`, `--break-glass`, and `--land` outside `--ci` —
    /// is TTY-only and refuses under `SPINE_AGENT=1`."
    ///
    /// The three that belong to `spine new` are that command's to enforce.
    /// `--dry-run` is excluded because it "never signs", and a dry run under
    /// an agent produces no `-Sig` line to refuse.
    pub fn signs_with_a_human_key(&self) -> bool {
        self.approve.is_some()
            || self.review.is_some()
            || self.break_glass.is_some()
            || (self.land.is_some() && !self.ci && !self.dry_run)
    }

    /// PB §11: "`--collect`, `--approve` … and `--constitution` … are the flags
    /// that execute repository code; none of them runs in the trusted stage."
    pub fn executes_repository_code(&self) -> bool {
        self.collect || self.approve.is_some() || self.constitution
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Init {
    pub ci: Option<String>,
    pub langs: Option<Vec<String>>,
    pub isolation: Option<String>,
    pub trunk: Option<String>,
    pub signer_key: Option<String>,
    pub identity: Option<String>,
    pub pipeline_key: Option<String>,
    pub hooks: bool,
    pub trust_root: Option<String>,
    pub rotate_trust_root: bool,
    pub dry_run: bool,
    pub status: bool,
    pub merge: bool,
    pub adopt: Vec<String>,
    pub force: Vec<String>,
    pub abort: bool,
    /// `--rollback [<sha>]` — the argument is optional, so `Some(None)` means
    /// "rollback, default target" and `None` means the flag was absent.
    pub rollback: Option<Option<String>>,
    pub uninstall: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgError {
    NoCommand,
    UnknownCommand(String),
    UnknownFlag {
        command: &'static str,
        flag: String,
    },
    MissingValue(String),
    /// PB §11 removed this flag on 2026-08-27; naming it explicitly is kinder
    /// than "unknown flag" to anyone with it in a script.
    WithdrawnFlag {
        flag: String,
        why: &'static str,
    },
    /// The value is outside the domain PB §11 fixes for the flag.
    BadValue {
        flag: String,
        value: String,
        domain: &'static str,
    },
}

impl fmt::Display for ArgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgError::NoCommand => write!(f, "usage: spine <init|new|index|check> [options]"),
            ArgError::UnknownCommand(c) => {
                write!(
                    f,
                    "unknown command {c:?}; spine has four: init, new, index, check"
                )
            }
            ArgError::UnknownFlag { command, flag } => {
                write!(f, "spine {command}: unknown flag {flag}")
            }
            ArgError::MissingValue(flag) => write!(f, "{flag} needs a value"),
            ArgError::WithdrawnFlag { flag, why } => write!(f, "{flag} was removed: {why}"),
            ArgError::BadValue {
                flag,
                value,
                domain,
            } => {
                write!(f, "{flag} {value:?} is not one of {domain}")
            }
        }
    }
}

impl core::error::Error for ArgError {}

pub fn parse(args: &[String]) -> Result<Command, ArgError> {
    let mut it = args.iter();
    let command = it.next().ok_or(ArgError::NoCommand)?;
    let rest: Vec<&String> = it.collect();

    match command.as_str() {
        "init" => parse_init(&rest).map(|init| Command::Init(Box::new(init))),
        "new" => Ok(Command::New),
        "index" => {
            let mut fresh = false;
            let mut dump = false;
            for arg in &rest {
                match arg.as_str() {
                    "--fresh" => fresh = true,
                    "--dump" => dump = true,
                    other => {
                        return Err(ArgError::UnknownFlag {
                            command: "index",
                            flag: other.to_string(),
                        });
                    }
                }
            }
            Ok(Command::Index { fresh, dump })
        }
        "check" => parse_check(&rest).map(|check| Command::Check(Box::new(check))),
        other => Err(ArgError::UnknownCommand(other.to_string())),
    }
}

fn parse_init(args: &[&String]) -> Result<Init, ArgError> {
    let mut init = Init::default();
    let mut index = 0;

    while index < args.len() {
        let arg = args[index].as_str();
        // A flag taking a value: `--flag value`. `next()` is the value, or a
        // MissingValue refusal — never a silent empty string, because an empty
        // `--trunk` would reach the manifest as a frozen field.
        let value = |index: &mut usize| -> Result<String, ArgError> {
            *index += 1;
            args.get(*index)
                .map(|s| s.to_string())
                .filter(|s| !s.starts_with("--"))
                .ok_or_else(|| ArgError::MissingValue(arg.to_string()))
        };

        match arg {
            "--ci" => {
                let v = value(&mut index)?;
                if !["github", "gitlab", "generic"].contains(&v.as_str()) {
                    return Err(ArgError::BadValue {
                        flag: "--ci".into(),
                        value: v,
                        domain: "github, gitlab, generic",
                    });
                }
                init.ci = Some(v);
            }
            "--langs" => {
                let v = value(&mut index)?;
                init.langs = Some(v.split(',').map(|s| s.trim().to_string()).collect());
            }
            "--isolation" => {
                let v = value(&mut index)?;
                // PB §11: "the CLI accepts `container` | `none` only". `uid` is
                // refused **at the flag**, because v1 ships no mechanism for it
                // and a manifest carrying it fails G16 outright with
                // `isolation-unsupported` (MF §6.2 check 12b). Refusing where
                // it is given beats refusing where it bites.
                if !["container", "none"].contains(&v.as_str()) {
                    return Err(ArgError::BadValue {
                        flag: "--isolation".into(),
                        value: v,
                        domain: "container, none (v1 ships no `uid` mechanism)",
                    });
                }
                init.isolation = Some(v);
            }
            "--trunk" => init.trunk = Some(value(&mut index)?),
            "--signer-key" => init.signer_key = Some(value(&mut index)?),
            "--identity" => init.identity = Some(value(&mut index)?),
            "--pipeline-key" => init.pipeline_key = Some(value(&mut index)?),
            "--hooks" => init.hooks = true,
            "--trust-root" => init.trust_root = Some(value(&mut index)?),
            "--rotate-trust-root" => init.rotate_trust_root = true,
            "--dry-run" => init.dry_run = true,
            "--status" => init.status = true,
            "--merge" => init.merge = true,
            "--adopt" => init.adopt.push(value(&mut index)?),
            "--force" => init.force.push(value(&mut index)?),
            "--abort" => init.abort = true,
            "--uninstall" => init.uninstall = true,
            "--rollback" => {
                // The argument is optional: PB §6.7's default is "the
                // first-parent commit that last touched the manifest".
                let next = args.get(index + 1).map(|s| s.as_str());
                match next {
                    Some(v) if !v.starts_with("--") => {
                        init.rollback = Some(Some(v.to_string()));
                        index += 1;
                    }
                    _ => init.rollback = Some(None),
                }
            }
            "--strategy" => {
                return Err(ArgError::WithdrawnFlag {
                    flag: "--strategy".into(),
                    why: "`params` has no `strategy` member and the constitution is user-owned, \
                          so it could write nothing. Merge strategy is `C-M1` in CONSTITUTION.md, \
                          which a human edits under the protected review it already takes",
                });
            }
            other => {
                return Err(ArgError::UnknownFlag {
                    command: "init",
                    flag: other.to_string(),
                });
            }
        }
        index += 1;
    }
    Ok(init)
}

/// PB §11's `spine check`.
///
/// **The subject of `--land` and `--review` is parsed positionally and the
/// three spellings are mutually exclusive.** `--land INT-042 --quick b` is not
/// a landing of two things; it is a mistake, and a landing is the wrong place
/// to guess which half was meant.
fn parse_check(args: &[&String]) -> Result<Check, ArgError> {
    let mut check = Check::default();
    let mut index = 0;

    // Which flag is currently collecting a subject: `--land` and `--review`
    // take theirs from the arguments that follow, and `--quick`/`--reseal`
    // belong to whichever of the two opened.
    #[derive(PartialEq)]
    enum Open {
        None,
        Land,
        Review,
    }
    let mut open = Open::None;

    while index < args.len() {
        let arg = args[index].as_str();
        let mut next = |flag: &str| -> Result<String, ArgError> {
            index += 1;
            args.get(index)
                .map(|v| v.to_string())
                .ok_or_else(|| ArgError::MissingValue(flag.to_string()))
        };

        match arg {
            "--ci" => check.ci = true,
            "--collect" => check.collect = true,
            "--constitution" => check.constitution = true,
            "--authority" => check.authority = true,
            "--reconstruct" => check.reconstruct = true,
            "--pre-receive" => check.pre_receive = true,
            "--print" => check.print = true,
            "--dry-run" => check.dry_run = true,
            "--approve" => check.approve = Some(next("--approve")?),
            "--report" => check.report = Some(next("--report")?),
            "--verify" => check.verify = Some(next("--verify")?),
            "--break-glass" => check.break_glass = Some(next("--break-glass")?),
            "--land" => {
                // "`<id>` omitted for upgrade landings" — so a bare `--land`
                // is a subject and not an omission. The id, if there is one,
                // arrives as the next argument and is claimed below.
                check.land = Some(Subject::Upgrade);
                open = Open::Land;
            }
            "--review" => {
                check.review = Some(Subject::Upgrade);
                open = Open::Review;
            }
            "--quick" => {
                let branch = next("--quick")?;
                match open {
                    Open::Land => check.land = Some(Subject::Quick(branch)),
                    Open::Review => check.review = Some(Subject::Quick(branch)),
                    // PB §5.2 spells the quick-lane landing
                    // `spine check --land --quick <branch>`; `--quick` alone
                    // names no operation.
                    Open::None => {
                        return Err(ArgError::UnknownFlag {
                            command: "check",
                            flag: "--quick without --land or --review".into(),
                        });
                    }
                }
                open = Open::None;
            }
            "--reseal" => {
                match open {
                    Open::Land => check.land = Some(Subject::Reseal),
                    Open::Review => check.review = Some(Subject::Reseal),
                    Open::None => {
                        return Err(ArgError::UnknownFlag {
                            command: "check",
                            flag: "--reseal without --land or --review".into(),
                        });
                    }
                }
                open = Open::None;
            }
            other if other.starts_with('-') => {
                return Err(ArgError::UnknownFlag {
                    command: "check",
                    flag: other.to_string(),
                });
            }
            // A bare word: the id of whichever of `--land`/`--review` is open.
            // Anywhere else it is a word `spine check` has no use for, and
            // swallowing it would let `spine check INT-042` run every gate on
            // the wrong subject in silence.
            id => match open {
                Open::Land => {
                    check.land = Some(Subject::Intent(id.to_string()));
                    open = Open::None;
                }
                Open::Review => {
                    check.review = Some(Subject::Intent(id.to_string()));
                    open = Open::None;
                }
                Open::None => {
                    return Err(ArgError::UnknownFlag {
                        command: "check",
                        flag: id.to_string(),
                    });
                }
            },
        }
        index += 1;
    }

    check_combination(&check)?;
    Ok(check)
}

/// The combinations PB §11 forbids, refused here rather than acted on.
fn check_combination(check: &Check) -> Result<(), ArgError> {
    // "`--print` emits a sealed envelope only for a run that would have
    // landed" — there is no run that would have landed without `--land`.
    if check.print && check.land.is_none() {
        return Err(ArgError::UnknownFlag {
            command: "check",
            flag: "--print without --land".into(),
        });
    }
    if check.dry_run && check.land.is_none() {
        return Err(ArgError::UnknownFlag {
            command: "check",
            flag: "--dry-run without --land".into(),
        });
    }
    // A dry run "never signs" and `--print` "emits a sealed envelope": asking
    // for both asks for an envelope nothing signed. Refusing beats printing an
    // unsigned one that looks like the real thing.
    if check.print && check.dry_run {
        return Err(ArgError::UnknownFlag {
            command: "check",
            flag: "--print with --dry-run".into(),
        });
    }
    // PB §11: `--collect`, `--approve` and `--constitution` "execute
    // repository code; none of them runs in the trusted stage." `--land` is
    // the trusted stage.
    if check.land.is_some() && check.executes_repository_code() {
        return Err(ArgError::UnknownFlag {
            command: "check",
            flag: "--land with a flag that executes repository code".into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn the_four_commands_and_nothing_else() {
        assert!(matches!(parse(&argv(&["init"])), Ok(Command::Init(_))));
        assert!(matches!(parse(&argv(&["new"])), Ok(Command::New)));
        assert!(matches!(
            parse(&argv(&["index"])),
            Ok(Command::Index { .. })
        ));
        assert!(matches!(parse(&argv(&["check"])), Ok(Command::Check(_))));

        // PB §11 lists `spine context`, `spine stats`, `spine review` and
        // `spine eval` as roadmap 3+, "not v1" — so they are unknown, not
        // unimplemented.
        for roadmap in ["context", "stats", "review", "eval"] {
            assert!(matches!(
                parse(&argv(&[roadmap])),
                Err(ArgError::UnknownCommand(_))
            ));
        }
        assert_eq!(parse(&[]), Err(ArgError::NoCommand));
    }

    fn check(words: &[&str]) -> Result<Check, ArgError> {
        let mut all = vec!["check"];
        all.extend_from_slice(words);
        match parse(&argv(&all)) {
            Ok(Command::Check(c)) => Ok(*c),
            Ok(other) => panic!("parsed as {other:?}"),
            Err(e) => Err(e),
        }
    }

    /// PB §11's three spellings of a subject, plus the fourth the prose adds:
    /// "`<id>` omitted for upgrade landings".
    #[test]
    fn land_takes_one_of_four_subjects() {
        assert_eq!(
            check(&["--land", "INT-042"]).unwrap().land,
            Some(Subject::Intent("INT-042".into()))
        );
        assert_eq!(
            check(&["--land", "--quick", "quick/typo"]).unwrap().land,
            Some(Subject::Quick("quick/typo".into()))
        );
        assert_eq!(
            check(&["--land", "--reseal"]).unwrap().land,
            Some(Subject::Reseal)
        );
        assert_eq!(check(&["--land"]).unwrap().land, Some(Subject::Upgrade));
    }

    /// `--quick` and `--reseal` belong to whichever of `--land`/`--review`
    /// opened; alone they name no operation.
    #[test]
    fn quick_and_reseal_are_not_operations_of_their_own() {
        assert!(matches!(
            check(&["--quick", "quick/typo"]),
            Err(ArgError::UnknownFlag { .. })
        ));
        assert!(matches!(
            check(&["--reseal"]),
            Err(ArgError::UnknownFlag { .. })
        ));
        assert_eq!(
            check(&["--review", "--quick", "quick/typo"])
                .unwrap()
                .review,
            Some(Subject::Quick("quick/typo".into()))
        );
    }

    /// A bare word with nothing open is a word `spine check` has no use for.
    /// Swallowing it would run every gate on the wrong subject in silence.
    #[test]
    fn a_stray_word_is_refused_rather_than_ignored() {
        assert!(matches!(
            check(&["INT-042"]),
            Err(ArgError::UnknownFlag { .. })
        ));
        assert!(matches!(
            check(&["--collect", "INT-042"]),
            Err(ArgError::UnknownFlag { .. })
        ));
    }

    /// PB §11: "`--collect` … is **independent of `--ci`** — the untrusted job
    /// passes both, a solo developer passes `--collect` alone."
    #[test]
    fn collect_stands_alone_and_stands_with_ci() {
        assert!(check(&["--collect"]).unwrap().collect);
        let both = check(&["--ci", "--collect"]).unwrap();
        assert!(both.ci && both.collect);
    }

    /// "`--print` emits a sealed envelope only for a run that would have
    /// landed"; "`--dry-run` never signs". Neither has a meaning without
    /// `--land`, and together they ask for an envelope nothing signed.
    #[test]
    fn print_and_dry_run_belong_to_land_and_not_to_each_other() {
        assert!(matches!(
            check(&["--print"]),
            Err(ArgError::UnknownFlag { .. })
        ));
        assert!(matches!(
            check(&["--dry-run"]),
            Err(ArgError::UnknownFlag { .. })
        ));
        assert!(matches!(
            check(&["--land", "INT-042", "--print", "--dry-run"]),
            Err(ArgError::UnknownFlag { .. })
        ));
        assert!(check(&["--land", "INT-042", "--print"]).unwrap().print);
    }

    /// PB §11: `--collect`, `--approve` and `--constitution` "execute
    /// repository code; none of them runs in the trusted stage" — and
    /// `--land` is the trusted stage.
    #[test]
    fn land_refuses_the_flags_that_execute_repository_code() {
        for flag in ["--collect", "--constitution"] {
            assert!(
                matches!(
                    check(&["--land", "INT-042", flag]),
                    Err(ArgError::UnknownFlag { .. })
                ),
                "{flag} was accepted beside --land"
            );
        }
        assert!(matches!(
            check(&["--land", "--approve", "INT-042"]),
            Err(ArgError::UnknownFlag { .. })
        ));
    }

    /// PB §7.1's TTY rule, as a property of the invocation.
    #[test]
    fn the_invocations_that_sign_under_a_human_key_are_the_ones_pb_7_1_names() {
        assert!(
            check(&["--approve", "INT-042"])
                .unwrap()
                .signs_with_a_human_key()
        );
        assert!(
            check(&["--review", "INT-042"])
                .unwrap()
                .signs_with_a_human_key()
        );
        assert!(
            check(&["--break-glass", "why"])
                .unwrap()
                .signs_with_a_human_key()
        );
        assert!(
            check(&["--land", "INT-042"])
                .unwrap()
                .signs_with_a_human_key()
        );

        // "`--land` outside `--ci`" — inside it, the pipeline secret signs.
        assert!(
            !check(&["--ci", "--land", "INT-042"])
                .unwrap()
                .signs_with_a_human_key()
        );
        // "`--dry-run` never signs", so there is no `-Sig` line to refuse.
        assert!(
            !check(&["--land", "INT-042", "--dry-run"])
                .unwrap()
                .signs_with_a_human_key()
        );
        assert!(!check(&["--collect"]).unwrap().signs_with_a_human_key());
    }

    /// A value flag with nothing after it is a refusal, never an empty string.
    #[test]
    fn a_check_value_flag_needs_its_value() {
        for flag in ["--approve", "--report", "--verify", "--break-glass"] {
            assert!(
                matches!(check(&[flag]), Err(ArgError::MissingValue(_))),
                "{flag} accepted no value"
            );
        }
        assert!(matches!(
            check(&["--land", "--quick"]),
            Err(ArgError::MissingValue(_))
        ));
    }

    #[test]
    fn init_flags_round_trip() {
        let parsed = parse(&argv(&[
            "init",
            "--ci",
            "github",
            "--langs",
            "python,ts",
            "--isolation",
            "container",
            "--trunk",
            "main",
            "--dry-run",
        ]))
        .unwrap();
        let Command::Init(init) = parsed else {
            panic!()
        };
        assert_eq!(init.ci.as_deref(), Some("github"));
        assert_eq!(
            init.langs,
            Some(vec!["python".to_string(), "ts".to_string()])
        );
        assert_eq!(init.isolation.as_deref(), Some("container"));
        assert_eq!(init.trunk.as_deref(), Some("main"));
        assert!(init.dry_run);
    }

    /// PB §11: the CLI accepts `container | none` only. `uid` is refused **at
    /// the flag** — v1 ships no mechanism, and a manifest carrying it fails G16
    /// outright with `isolation-unsupported`, "because no protected reviewer
    /// can make a mechanism exist".
    #[test]
    fn uid_is_refused_at_the_flag_not_at_the_gate() {
        let err = parse(&argv(&["init", "--isolation", "uid"])).unwrap_err();
        assert!(matches!(err, ArgError::BadValue { .. }));
        assert!(err.to_string().contains("v1 ships no `uid` mechanism"));
    }

    #[test]
    fn the_ci_domain_is_closed() {
        assert!(parse(&argv(&["init", "--ci", "jenkins"])).is_err());
        for good in ["github", "gitlab", "generic"] {
            assert!(parse(&argv(&["init", "--ci", good])).is_ok());
        }
    }

    /// `--rollback [<sha>]` takes an optional argument, so the parser must
    /// distinguish "flag with no argument" from "flag followed by another flag".
    #[test]
    fn rollback_takes_an_optional_sha() {
        let Command::Init(bare) = parse(&argv(&["init", "--rollback"])).unwrap() else {
            panic!()
        };
        assert_eq!(bare.rollback, Some(None));

        let Command::Init(with_sha) = parse(&argv(&["init", "--rollback", "abc123"])).unwrap()
        else {
            panic!()
        };
        assert_eq!(with_sha.rollback, Some(Some("abc123".into())));

        let Command::Init(before_flag) =
            parse(&argv(&["init", "--rollback", "--dry-run"])).unwrap()
        else {
            panic!()
        };
        assert_eq!(before_flag.rollback, Some(None));
        assert!(before_flag.dry_run, "the next flag is not eaten as a value");
    }

    /// `--adopt` and `--force` name paths and may be repeated — PB §6.7 step 3
    /// resolves one refusing path at a time.
    #[test]
    fn adopt_and_force_accumulate() {
        let Command::Init(init) = parse(&argv(&[
            "init",
            "--adopt",
            "AGENTS.md#spine",
            "--force",
            ".spine/ci.sh",
            "--force",
            ".gitignore#spine",
        ]))
        .unwrap() else {
            panic!()
        };
        assert_eq!(init.adopt, vec!["AGENTS.md#spine"]);
        assert_eq!(init.force, vec![".spine/ci.sh", ".gitignore#spine"]);
    }

    /// A flag that takes a value and is given none must refuse, never take an
    /// empty string — `--trunk` writes a frozen manifest field.
    #[test]
    fn a_value_flag_with_no_value_refuses() {
        assert_eq!(
            parse(&argv(&["init", "--trunk"])).unwrap_err(),
            ArgError::MissingValue("--trunk".into())
        );
        assert_eq!(
            parse(&argv(&["init", "--trunk", "--dry-run"])).unwrap_err(),
            ArgError::MissingValue("--trunk".into())
        );
    }

    /// The withdrawn flag is named rather than reported as unknown, because
    /// somebody has it in a script.
    #[test]
    fn the_withdrawn_strategy_flag_says_what_happened_to_it() {
        let err = parse(&argv(&["init", "--strategy", "merge"])).unwrap_err();
        assert!(matches!(err, ArgError::WithdrawnFlag { .. }));
        assert!(err.to_string().contains("C-M1"));
    }

    #[test]
    fn index_takes_its_two_flags() {
        assert_eq!(
            parse(&argv(&["index", "--fresh", "--dump"])).unwrap(),
            Command::Index {
                fresh: true,
                dump: true
            }
        );
        assert!(parse(&argv(&["index", "--wat"])).is_err());
    }
}
