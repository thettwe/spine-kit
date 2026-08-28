//! The three intent scaffolds — the bytes `spine new` emits (TM §6).
//!
//! **A scaffold does not parse, and that is the design** (TM §6.3). Run the
//! parse over any of the three and it reaches the first mandatory body, finds
//! it empty, and refuses with `empty-section` at exit 4. Three consequences,
//! all wanted: a placeholder can never be signed; a scaffolded touchpoint can
//! never become a lease binding on everyone else's landings; and the state
//! machine needs no new row, because `draft` is where a scaffold lives.
//!
//! Only four spans vary (TM §6.1). Every other byte "is fixed by §6.4 and is
//! identical in every repository on every platform".

use core::fmt;

/// TM §3.1: three variants, one lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Intent,
    IntentChange,
    IntentBug,
}

impl Variant {
    /// The template name, which is also the `templates`/`resign` key and the
    /// first half of the `Template:` value — one `name@version` vocabulary
    /// across all four sites (README decision 4, 2026-08-26).
    pub fn name(self) -> &'static str {
        match self {
            Variant::Intent => "intent",
            Variant::IntentChange => "intent-change",
            Variant::IntentBug => "intent-bug",
        }
    }

    /// The scaffold body, with the four substitution spans marked. Held as the
    /// published §6.4 vectors so the bytes in the binary are the bytes the
    /// corpus hashed.
    fn body(self) -> &'static str {
        match self {
            Variant::Intent => include_str!("../tests/vectors/tm-6.4-intent-2.md"),
            Variant::IntentChange => include_str!("../tests/vectors/tm-6.4-intent-change-2.md"),
            Variant::IntentBug => include_str!("../tests/vectors/tm-6.4-intent-bug-2.md"),
        }
    }

    /// The id prefix each variant's documents take. TM §3.3: `--bug` forces the
    /// prefix, "and that is now checked as well as required".
    pub fn id_prefix(self) -> &'static str {
        match self {
            Variant::IntentBug => "BUG",
            _ => "INT",
        }
    }
}

impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// TM §6.1's four substitutions, and nothing else varies.
#[derive(Debug, Clone)]
pub struct Instance<'a> {
    /// Substitution 1: the allocated id, "equal to the path's and the
    /// branch's".
    pub id: &'a str,
    /// Substitution 2: the principal of the signing identity, **verbatim, with
    /// no `@` prefix added**. PB §3.1's `Owner: @name` is a human convention —
    /// a forge handle — and `spine new` has no source for one; prefixing would
    /// produce `@alice@example.com`.
    pub owner: &'a str,
    /// Substitution 3: `templates.<variant>` from the manifest, read from
    /// trunk. Only the digits vary — the variant token is a literal.
    pub template_version: u64,
    /// Substitution 4: the version of the constitution at the manifest's
    /// `paths.constitution`.
    pub constitution_version: u32,
}

/// Why a scaffold could not be emitted. TM §6.1's refusal column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScaffoldError {
    /// ID §4.3: the owner value is empty, exceeds 128 bytes, contains `" · "`,
    /// or has leading or trailing space or tab.
    BadOwnerPrincipal,
}

impl fmt::Display for ScaffoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScaffoldError::BadOwnerPrincipal => f.write_str("bad-owner-principal"),
        }
    }
}

impl core::error::Error for ScaffoldError {}

/// ID §4.3's owner grammar, which TM §6.1 refuses against.
fn check_owner(owner: &str) -> Result<(), ScaffoldError> {
    let ok = !owner.is_empty()
        && owner.len() <= 128
        && !owner.contains(" \u{00B7} ")
        && !owner.starts_with([' ', '\t'])
        && !owner.ends_with([' ', '\t'])
        && !owner.contains('\n');
    if ok {
        Ok(())
    } else {
        Err(ScaffoldError::BadOwnerPrincipal)
    }
}

/// Render one scaffold.
pub fn render(variant: Variant, instance: &Instance<'_>) -> Result<String, ScaffoldError> {
    check_owner(instance.owner)?;

    // The published vectors carry a concrete instance, which is what let the
    // corpus hash them. Substituting back out of them — rather than holding a
    // second, marked-up copy — keeps the bytes in the binary and the bytes the
    // corpus hashed the same bytes.
    let (vector_id, vector_owner) = match variant {
        Variant::Intent => ("INT-042", "alice@example.com"),
        Variant::IntentChange => ("INT-043", "alice@example.com"),
        Variant::IntentBug => ("BUG-051", "bob@example.com"),
    };

    Ok(variant
        .body()
        .replacen(vector_id, instance.id, 1)
        .replacen(
            &format!("Owner: {vector_owner}"),
            &format!("Owner: {}", instance.owner),
            1,
        )
        .replacen(
            &format!("Template: {}@2", variant.name()),
            &format!("Template: {}@{}", variant.name(), instance.template_version),
            1,
        )
        .replacen(
            "Constitution: v3",
            &format!("Constitution: v{}", instance.constitution_version),
            1,
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use spine_canon::ObjectFormat;

    /// TM §6.4, all three, every published row. The instances are the ones
    /// §6.4 names: INT-042 / INT-043 owned by alice, BUG-051 by bob, template
    /// version 2 and the constitution at v3 in all three.
    #[test]
    fn tm_6_4_the_three_scaffolds_byte_for_byte() {
        struct Case {
            variant: Variant,
            id: &'static str,
            owner: &'static str,
            bytes: usize,
            chars: usize,
            lines: usize,
            sha1: &'static str,
            sha256_blob: &'static str,
            sha256_file: &'static str,
        }

        for case in [
            Case {
                variant: Variant::Intent,
                id: "INT-042",
                owner: "alice@example.com",
                bytes: 380,
                chars: 372,
                lines: 14,
                sha1: "e627ec183de2a71b0e5aaed0b6227c1e8437ccde",
                sha256_blob: "a4dae5b325b3661b7892cbb9d8b9c846fdda4c27ac97690d8503fe80bae35647",
                sha256_file: "eea04ff59b608f016a8f6ae7d24bdae0dcfe77615d99e9858c31af72d5603071",
            },
            Case {
                variant: Variant::IntentChange,
                id: "INT-043",
                owner: "alice@example.com",
                bytes: 501,
                chars: 489,
                lines: 18,
                sha1: "091549257b229b6a3eb7ae5d44e4e9937a7d941a",
                sha256_blob: "fd0059feb982fce1c8c90a2aebf62d61f243c56a0af660aabf51c14edb6e4257",
                sha256_file: "e130a6ca264383a8083ede79d81228b9fd6b5194ca8299e07c68349c6d74bffb",
            },
            Case {
                variant: Variant::IntentBug,
                id: "BUG-051",
                owner: "bob@example.com",
                bytes: 434,
                chars: 424,
                lines: 14,
                sha1: "5eb75dcc51602ecb01d9d428d2ed0eebb2d1a86c",
                sha256_blob: "62331b46c4b2602c8f24955e330e19c08e58a3f49ba757cf3961a75d1d0a665d",
                sha256_file: "868e04bfe7bd6fca19bc835a4b57a8e6423bb108d607a48ed350f52b62b5d54b",
            },
        ] {
            let rendered = render(
                case.variant,
                &Instance {
                    id: case.id,
                    owner: case.owner,
                    template_version: 2,
                    constitution_version: 3,
                },
            )
            .expect("a published instance renders");

            let name = case.variant.name();
            assert_eq!(rendered.len(), case.bytes, "{name} byte length");
            assert_eq!(rendered.chars().count(), case.chars, "{name} characters");
            assert_eq!(rendered.matches('\n').count(), case.lines, "{name} lines");
            assert_eq!(
                spine_canon::git_blob_id(rendered.as_bytes(), ObjectFormat::Sha1),
                case.sha1,
                "{name} blob id, sha1"
            );
            assert_eq!(
                spine_canon::git_blob_id(rendered.as_bytes(), ObjectFormat::Sha256),
                case.sha256_blob,
                "{name} blob id, sha256"
            );
            assert_eq!(
                spine_canon::sha256_hex(rendered.as_bytes()),
                case.sha256_file,
                "{name} sha256sum over the file's bytes"
            );
        }
    }

    /// TM §6.1: only four spans vary. Anything else moving would break every
    /// published digest above, so this pins the negative.
    #[test]
    fn exactly_four_spans_vary() {
        let base = render(
            Variant::Intent,
            &Instance {
                id: "INT-042",
                owner: "alice@example.com",
                template_version: 2,
                constitution_version: 3,
            },
        )
        .unwrap();
        let moved = render(
            Variant::Intent,
            &Instance {
                id: "INT-900",
                owner: "carol@example.com",
                template_version: 5,
                constitution_version: 11,
            },
        )
        .unwrap();

        assert!(moved.starts_with("# INT-900: <short imperative title>\n"));
        assert!(moved.contains(
            "Owner: carol@example.com \u{00B7} Template: intent@5 \u{00B7} Constitution: v11\n"
        ));
        // Every line after the header is untouched.
        assert_eq!(
            base.lines().skip(2).collect::<Vec<_>>(),
            moved.lines().skip(2).collect::<Vec<_>>()
        );
    }

    /// TM §6.2: the `touchpoints` section is the one section with structural
    /// body lines, and they are scaffolded because they are the section's
    /// *grammar* rather than its content — with them an unfilled author gets
    /// `no-expected-touchpoint`, which names what to add; without them,
    /// `missing-touchpoint-line`, which requires knowing two exact strings.
    #[test]
    fn touchpoints_carries_its_two_label_lines_and_nothing_else_does() {
        for variant in [Variant::Intent, Variant::IntentChange, Variant::IntentBug] {
            let rendered = render(
                variant,
                &Instance {
                    id: "INT-001",
                    owner: "a@b.c",
                    template_version: 2,
                    constitution_version: 1,
                },
            )
            .unwrap();
            assert!(rendered.contains("Expected to change:\nMust NOT change:\n"));
            // TM §6.2, quoting ID §5.5: "A scaffold that seeds a prose line
            // here makes every freshly created intent unsignable."
            assert!(
                rendered.ends_with(
                    "## Open questions (optional — must be empty before implementation)\n"
                )
            );
        }
    }

    /// TM §6.1: "No `@` is prefixed to the principal … Prefixing would produce
    /// `@alice@example.com`."
    #[test]
    fn the_owner_is_verbatim_and_refused_when_it_cannot_be() {
        let ok = render(
            Variant::Intent,
            &Instance {
                id: "INT-001",
                owner: "alice@example.com",
                template_version: 2,
                constitution_version: 1,
            },
        )
        .unwrap();
        assert!(ok.contains("Owner: alice@example.com "));
        assert!(!ok.contains("@alice@example.com"));

        // ID §4.3's refusals, each `bad-owner-principal`.
        for bad in ["", " alice", "alice ", "a \u{00B7} b", &"x".repeat(129)] {
            assert_eq!(
                render(
                    Variant::Intent,
                    &Instance {
                        id: "INT-001",
                        owner: bad,
                        template_version: 2,
                        constitution_version: 1,
                    },
                )
                .unwrap_err(),
                ScaffoldError::BadOwnerPrincipal,
                "{bad:?} should be refused"
            );
        }
    }

    /// TM §6.1: "`Ticket` is omitted from every scaffold" — an empty value is
    /// impossible and a scaffolded placeholder would be sealed into a landing
    /// forever. And "No `Supersedes:` line is scaffolded."
    #[test]
    fn the_optional_header_fields_are_not_scaffolded() {
        let rendered = render(
            Variant::Intent,
            &Instance {
                id: "INT-001",
                owner: "a@b.c",
                template_version: 2,
                constitution_version: 1,
            },
        )
        .unwrap();
        assert!(!rendered.contains("Ticket:"));
        assert!(!rendered.contains("Supersedes:"));
    }

    /// README decision 4, 2026-08-26: `Template:` names the variant *and* the
    /// version. "Every sealed intent carries this header for ever, so it was
    /// decidable now or never."
    #[test]
    fn template_is_qualified_never_a_bare_version() {
        for (variant, expected) in [
            (Variant::Intent, "Template: intent@2"),
            (Variant::IntentChange, "Template: intent-change@2"),
            (Variant::IntentBug, "Template: intent-bug@2"),
        ] {
            let rendered = render(
                variant,
                &Instance {
                    id: "INT-001",
                    owner: "a@b.c",
                    template_version: 2,
                    constitution_version: 1,
                },
            )
            .unwrap();
            assert!(rendered.contains(expected), "{expected}");
            assert!(!rendered.contains("Template: v2"));
        }
    }
}
