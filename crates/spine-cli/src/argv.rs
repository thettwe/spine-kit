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
    Check,
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
        "check" => Ok(Command::Check),
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
        assert!(matches!(parse(&argv(&["check"])), Ok(Command::Check)));

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
