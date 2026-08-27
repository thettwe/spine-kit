//! RF §4.2's header line — six fields, one order, and a repeated key rejects
//! the file.
//!
//! ```text
//! tree=<oid> base=<sha> tool=<version>+sha256:<hex64> keys_visible=<bool> profile=<profile> ids=<n>
//! ```
//!
//! This line is signed-adjacent. Its `tool=` "is spelled exactly as the seal's
//! `tool=` (§11, `Spine-Seal`), so the two compare by byte equality and an
//! auditor can read them side by side"; its `profile=` "is sealed into the
//! landing and stays in the ledger for it forever"; and RF §8.4 reads
//! `keys_visible` and `profile` as two of auto-merge's three preconditions. A
//! header this module spells differently is a landing signed over different
//! bytes.
//!
//! RF §4.2 also fixes why the line does **not** grow under multi-runner: "one
//! header line cannot name several runners without a repeated key, and *a
//! repeated key rejects the file*." The runner qualification lives on each
//! record instead.

use crate::malformed::Malformed;
use core::fmt;
use spine_canon::ObjectFormat;

/// RF §4.2 field 5's closed domain, which is also PB §7.4 rule 3's three
/// profiles.
///
/// The header field is a **finding**, never a request: RF §7.1 says "`profile=`
/// names the boundary the collector achieved by creating it **and testing it**,
/// never what configuration claims". `params.isolation` is the request and
/// lives in [`spine_manifest::Isolation`]; the two are separate types here
/// because RF §8.4 precondition 1 exists precisely to compare them, and a
/// single type would make the comparison a tautology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Licensed only by RF §7.1's P1 ∧ P2 ∧ P3 ∧ P4, all four passed against a
    /// probe boundary built from the runner disposition.
    Container,
    /// RF §4.2: "`uid` is never written by a v1 collector (§7.1)" — v1 ships no
    /// mechanism for it and refuses the request at step 1 instead. The variant
    /// exists because the *reader* must still accept the value: the domain is
    /// three-valued in the grammar even though the writer's range is two.
    Uid,
    /// "no boundary is attempted … `none` asserts the *absence* of a boundary,
    /// and an absence needs no evidence."
    None,
}

impl Profile {
    pub fn token(self) -> &'static str {
        match self {
            Profile::Container => "container",
            Profile::Uid => "uid",
            Profile::None => "none",
        }
    }

    /// RF §4.2: "`n/a` is a seal value for landings that run no suite (§11); it
    /// is never a header value, and a header carrying it is malformed." It is
    /// not in the match below, so it parses as no profile at all.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "container" => Some(Profile::Container),
            "uid" => Some(Profile::Uid),
            "none" => Some(Profile::None),
            _ => None,
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// RF §4.2's six fields, in order. The order is the wire order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// "the `T` the untrusted job computed itself" — `git merge-tree
    /// --write-tree origin/<trunk> H`, lowercase hex, full length.
    pub tree: String,
    /// "`origin/<trunk>` tip at the moment the collector read policy — for a
    /// reseal, the seal's `base=`, from which every policy read for a reseal is
    /// taken (§5.5, §13 R22)."
    pub base: String,
    /// "the collector's **own** embedded version and artifact-list hash."
    ///
    /// RF §4.2 is emphatic that this is not copied: "The collector writes what
    /// it **is**, never what trunk pins. Copying the manifest's value would
    /// assert nothing. Equality between the two is the trusted stage's check,
    /// not the collector's (§8.3)."
    pub tool: String,
    /// "the key-material predicate below, over the collector's own environment
    /// and every runner's."
    ///
    /// One assertion for the whole job: "the field is not per-runner, and a
    /// collector that strips key material for one runner and not another writes
    /// `true`." RF §4.2 also requires the honest negation to be written rather
    /// than the field omitted.
    pub keys_visible: bool,
    /// "the boundary the collector **achieved** — created *and* tested."
    pub profile: Profile,
    /// "the number of `base` records that follow — the cardinality of the set
    /// of `(runner, id)` pairs collected on `B`."
    ///
    /// RF §4.2: it "counts the `base` records that follow, not the result
    /// records (§13, R2) … It is **not** the truncation guard: truncation
    /// removes the `end` record, which §4.5 already makes malformed. It
    /// cross-checks the collector against itself — a collector that emitted
    /// fewer `base` records than it counted is the case nothing else catches."
    pub ids: u64,
}

/// The five header values the collector **observes**; `ids=` is the sixth and
/// is derived from the body it wrote.
///
/// They are grouped because they have one provenance between them and RF §4.2
/// argues each separately: `tree` and `base` are what the run fixed, `tool` is
/// "the collector's **own** embedded version and artifact-list hash",
/// `keys_visible` is step 4's probe over the whole job, and `profile` is step
/// 6's finding. `ids=` belongs to none of that — it is a count of records —
/// which is why [`crate::file::ResultFile::new`] derives it rather than
/// accepting it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub tree: String,
    pub base: String,
    pub tool: String,
    pub keys_visible: bool,
    pub profile: Profile,
}

impl Provenance {
    /// Complete the header with the one field the body decides.
    pub fn into_header(self, ids: u64) -> Header {
        Header {
            tree: self.tree,
            base: self.base,
            tool: self.tool,
            keys_visible: self.keys_visible,
            profile: self.profile,
            ids,
        }
    }
}

/// RF §4.2's table, keys 1..6, in the fixed order.
const KEYS: [&str; 6] = ["tree", "base", "tool", "keys_visible", "profile", "ids"];

/// The literal RF §4.2 splits `tool=` at, "which is unambiguous because
/// `<dist_hash>` is exactly 64 lowercase hex".
const TOOL_SEPARATOR: &str = "+sha256:";

impl Header {
    /// Serialize to the header line, without its LF.
    ///
    /// Fields are joined by "exactly one `U+0020`". Nothing here can emit a
    /// second one: every value's grammar excludes the space, which is checked
    /// on the way in and is a property of the types on the way out.
    pub fn to_line(&self) -> String {
        format!(
            "tree={} base={} tool={} keys_visible={} profile={} ids={}",
            self.tree,
            self.base,
            self.tool,
            if self.keys_visible { "true" } else { "false" },
            self.profile,
            self.ids,
        )
    }

    /// Parse line 1.
    ///
    /// `format` comes from trunk's manifest, never from the file: RF §4.2 fixes
    /// the oid length "per `object_format`", and RF §3 spells it out — "40
    /// characters under `object_format: sha1`, 64 under `sha256`, as trunk's
    /// manifest records". A reader that accepted either length would accept a
    /// sha1 header in a sha256 repository, which is a `tree=` no run computed.
    pub fn parse(line: &str, format: ObjectFormat) -> Result<Self, Malformed> {
        // "separated by exactly one `U+0020`" — so a doubled separator yields an
        // empty field and a value containing a space yields a seventh. Both are
        // caught by the count, which is why the count is checked first.
        let fields: Vec<&str> = line.split(' ').collect();
        if fields.len() != KEYS.len() {
            return Err(Malformed::HeaderFieldCount {
                found: fields.len(),
            });
        }

        let mut values: Vec<&str> = Vec::with_capacity(KEYS.len());
        let mut seen: Vec<&str> = Vec::with_capacity(KEYS.len());
        for (index, field) in fields.iter().enumerate() {
            let position = index + 1;
            let Some((key, value)) = field.split_once('=') else {
                return Err(Malformed::HeaderFieldShape { position });
            };
            if key.is_empty() {
                return Err(Malformed::HeaderFieldShape { position });
            }
            // RF §4.2, PB §11: "**A repeated key rejects the file**". Checked
            // before the order check so that `tree=… tree=…` reports the repeat
            // rather than the position — the two are different mistakes, and
            // the repeat is the one multi-runner would have needed.
            if seen.contains(&key) {
                return Err(Malformed::HeaderRepeatedKey {
                    key: key.to_owned(),
                });
            }
            seen.push(key);
            // "The field order is fixed. A header whose keys appear in any
            // other order is malformed." A missing key and an unknown key are
            // the same failure seen from here: at this position the key is not
            // the one the table fixes.
            if key != KEYS[index] {
                return Err(Malformed::HeaderKeyOutOfOrder {
                    position,
                    expected: KEYS[index],
                    found: key.to_owned(),
                });
            }
            if value.is_empty() {
                return Err(Malformed::HeaderEmptyValue { key: KEYS[index] });
            }
            values.push(value);
        }

        let tree = oid(values[0], "tree", format)?;
        let base = oid(values[1], "base", format)?;
        let tool = tool(values[2])?;
        let keys_visible = match values[3] {
            "true" => true,
            "false" => false,
            _ => {
                return Err(Malformed::HeaderValueOutOfGrammar {
                    key: "keys_visible",
                    why: "not `true` or `false`",
                });
            }
        };
        let profile = Profile::parse(values[4]).ok_or(Malformed::HeaderValueOutOfGrammar {
            key: "profile",
            // Named rather than generic because `n/a` is the value a
            // seal-shaped implementation reaches for (RF §4.2, §13 R15).
            why: "not `container`, `uid` or `none` (`n/a` is a seal value, never a header one)",
        })?;
        let ids = decimal(values[5])?;

        Ok(Header {
            tree,
            base,
            tool,
            keys_visible,
            profile,
            ids,
        })
    }

    /// RF §3: "The filename stem equals the header's `tree=` value byte for
    /// byte. A file whose stem and header disagree is malformed (§8)."
    ///
    /// The stem is compared, not derived: RF §3 adds that "The stem carries no
    /// prefix, suffix, branch name or intent id", so there is nothing to strip.
    pub fn check_stem(&self, stem: &str) -> Result<(), Malformed> {
        if stem == self.tree {
            return Ok(());
        }
        Err(Malformed::StemDisagreesWithTree {
            stem: stem.to_owned(),
            tree: self.tree.clone(),
        })
    }

    /// RF §3's path for this header's tree. One file per `T`, "covering every
    /// runner. There is no per-runner file, no per-runner directory and no
    /// per-language suffix."
    pub fn path(&self) -> String {
        format!(".spine/cache/results/{}.jsonl", self.tree)
    }

    /// Split `tool=` into its version and its `sha256:`-prefixed hash.
    ///
    /// RF §4.2: "`tool=` needs no parse to be checked — the trusted stage
    /// constructs the expected token from trunk's manifest and compares bytes
    /// (§8.3) — and where a parse is wanted, the token splits at its **last**
    /// occurrence of the literal `+sha256:`". The last, not the first: a
    /// version string is free to contain `+sha256:` of its own, and RF §4.2
    /// admits every printable-ASCII version — "**No version is
    /// unrepresentable.**"
    pub fn tool_parts(&self) -> (&str, &str) {
        let at = self
            .tool
            .rfind(TOOL_SEPARATOR)
            .expect("a parsed tool= carries the separator");
        (&self.tool[..at], &self.tool[at + 1..])
    }
}

/// RF §4.2 fields 1 and 2: "lowercase hex object id, length per
/// `object_format`".
fn oid(value: &str, key: &'static str, format: ObjectFormat) -> Result<String, Malformed> {
    if value.len() != format.hex_len() {
        return Err(Malformed::HeaderValueOutOfGrammar {
            key,
            why: "object id length disagrees with trunk's object_format",
        });
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Malformed::HeaderValueOutOfGrammar {
            key,
            why: "not lowercase hex",
        });
    }
    Ok(value.to_owned())
}

/// RF §4.2 field 3: `<version>` `+` `sha256:` `<64 lowercase hex>`.
fn tool(value: &str) -> Result<String, Malformed> {
    let Some(at) = value.rfind(TOOL_SEPARATOR) else {
        return Err(Malformed::HeaderValueOutOfGrammar {
            key: "tool",
            why: "no `+sha256:` separator",
        });
    };
    let (version, rest) = value.split_at(at);
    let hex = &rest[TOOL_SEPARATOR.len()..];
    // "`<version>` is trunk's `cli.version` string verbatim: non-empty, every
    // character in `U+0021`–`U+007E` (printable ASCII, the space already
    // excluded by the field rule above)."
    if version.is_empty() {
        return Err(Malformed::HeaderValueOutOfGrammar {
            key: "tool",
            why: "empty version",
        });
    }
    if !version.bytes().all(|b| (0x21..=0x7E).contains(&b)) {
        return Err(Malformed::HeaderValueOutOfGrammar {
            key: "tool",
            why: "version holds a byte outside U+0021–U+007E",
        });
    }
    // "which is unambiguous because `<dist_hash>` is exactly 64 lowercase hex."
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Malformed::HeaderValueOutOfGrammar {
            key: "tool",
            why: "dist hash is not exactly 64 lowercase hex",
        });
    }
    Ok(value.to_owned())
}

/// RF §4.2 field 6: "non-negative decimal, no sign, no leading zero except `0`
/// itself".
fn decimal(value: &str) -> Result<u64, Malformed> {
    let bad = |why| Malformed::HeaderValueOutOfGrammar { key: "ids", why };
    if !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("not decimal digits"));
    }
    if value.len() > 1 && value.starts_with('0') {
        return Err(bad("leading zero"));
    }
    value.parse::<u64>().map_err(|_| bad("does not fit in u64"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RF §10's header line, byte for byte. It is the one published header in
    /// the corpus, and the twenty-line file below it is written against it.
    const VECTOR: &str = "tree=3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28 \
base=7b0d4a1f2c3e5d6a8b9c0d1e2f3a4b5c6d7e8f90 \
tool=1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db \
keys_visible=false profile=container ids=7";

    fn vector() -> Header {
        Header::parse(VECTOR, ObjectFormat::Sha1).expect("RF §10's header is conforming")
    }

    #[test]
    fn the_worked_examples_header_round_trips_byte_for_byte() {
        let header = vector();
        assert_eq!(header.tree, "3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28");
        assert_eq!(header.base, "7b0d4a1f2c3e5d6a8b9c0d1e2f3a4b5c6d7e8f90");
        assert!(!header.keys_visible);
        assert_eq!(header.profile, Profile::Container);
        assert_eq!(header.ids, 7);
        assert_eq!(header.to_line(), VECTOR);
    }

    /// RF §10: the file is
    /// `.spine/cache/results/3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28.jsonl`.
    #[test]
    fn the_path_is_the_tree_under_the_results_directory() {
        assert_eq!(
            vector().path(),
            ".spine/cache/results/3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28.jsonl"
        );
    }

    /// RF §3: "A file whose stem and header disagree is malformed (§8)."
    #[test]
    fn a_stem_that_is_not_the_headers_tree_is_malformed() {
        let header = vector();
        assert!(header.check_stem(&header.tree.clone()).is_ok());
        assert!(matches!(
            header.check_stem("0000000000000000000000000000000000000000"),
            Err(Malformed::StemDisagreesWithTree { .. })
        ));
    }

    /// RF §4.2, PB §11: "**A repeated key rejects the file**" — the rule that
    /// makes a multi-runner header impossible and therefore puts the `runner`
    /// qualifier on each record instead.
    #[test]
    fn a_repeated_key_rejects_the_file() {
        let line = VECTOR.replace("base=7b0d", "tree=7b0d");
        assert_eq!(
            Header::parse(&line, ObjectFormat::Sha1),
            Err(Malformed::HeaderRepeatedKey { key: "tree".into() })
        );
    }

    /// "The field order is fixed. A header whose keys appear in any other order
    /// is malformed."
    #[test]
    fn a_header_whose_keys_are_out_of_order_is_malformed() {
        let line = "base=7b0d4a1f2c3e5d6a8b9c0d1e2f3a4b5c6d7e8f90 \
tree=3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28 \
tool=1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db \
keys_visible=false profile=container ids=7";
        assert_eq!(
            Header::parse(line, ObjectFormat::Sha1),
            Err(Malformed::HeaderKeyOutOfOrder {
                position: 1,
                expected: "tree",
                found: "base".into()
            })
        );
    }

    /// "So does a missing key, an unknown key, an empty value, a value
    /// containing `U+0020`, and any value outside its grammar."
    #[test]
    fn a_missing_key_an_unknown_key_and_a_spaced_value_all_reject_the_file() {
        let missing = VECTOR.replace(" ids=7", "");
        assert!(matches!(
            Header::parse(&missing, ObjectFormat::Sha1),
            Err(Malformed::HeaderFieldCount { found: 5 })
        ));

        let unknown = format!("{VECTOR} lang=python");
        assert!(matches!(
            Header::parse(&unknown, ObjectFormat::Sha1),
            Err(Malformed::HeaderFieldCount { found: 7 })
        ));

        let spaced = VECTOR.replace("profile=container", "profile=con tainer");
        assert!(matches!(
            Header::parse(&spaced, ObjectFormat::Sha1),
            Err(Malformed::HeaderFieldCount { found: 7 })
        ));

        let doubled = VECTOR.replace("keys_visible=false profile", "keys_visible=false  profile");
        assert!(matches!(
            Header::parse(&doubled, ObjectFormat::Sha1),
            Err(Malformed::HeaderFieldCount { found: 7 })
        ));
    }

    #[test]
    fn an_empty_value_rejects_the_file() {
        let line = VECTOR.replace("profile=container", "profile=");
        assert_eq!(
            Header::parse(&line, ObjectFormat::Sha1),
            Err(Malformed::HeaderEmptyValue { key: "profile" })
        );
    }

    /// RF §4.2: "`n/a` is a seal value for landings that run no suite (§11); it
    /// is never a header value, and a header carrying it is malformed."
    #[test]
    fn n_slash_a_is_a_seal_value_and_never_a_header_one() {
        let line = VECTOR.replace("profile=container", "profile=n/a");
        assert!(matches!(
            Header::parse(&line, ObjectFormat::Sha1),
            Err(Malformed::HeaderValueOutOfGrammar { key: "profile", .. })
        ));
    }

    /// RF §4.2's three-value domain is what the *reader* admits, even though
    /// RF §7.1 says "`uid` is a value no v1 collector writes".
    #[test]
    fn the_profile_domain_is_three_values_on_read() {
        assert_eq!(Profile::parse("container"), Some(Profile::Container));
        assert_eq!(Profile::parse("uid"), Some(Profile::Uid));
        assert_eq!(Profile::parse("none"), Some(Profile::None));
        assert_eq!(Profile::parse("n/a"), None);
        assert_eq!(Profile::parse("NONE"), None);
    }

    /// RF §3: "lowercase hex, full length, never abbreviated — 40 characters
    /// under `object_format: sha1`, 64 under `sha256`, as trunk's manifest
    /// records."
    #[test]
    fn an_oid_length_is_trunks_object_format_and_not_the_files_claim() {
        // The sha1-length vector read as a sha256 repository is not a shorter
        // oid, it is a `tree=` no run in that repository computed.
        assert!(matches!(
            Header::parse(VECTOR, ObjectFormat::Sha256),
            Err(Malformed::HeaderValueOutOfGrammar { key: "tree", .. })
        ));
        let upper = VECTOR.replace(
            "tree=3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28",
            "tree=3F7B1C9D2A5E48F0B6C1D8E2A9F403B7C5D61E28",
        );
        assert!(matches!(
            Header::parse(&upper, ObjectFormat::Sha1),
            Err(Malformed::HeaderValueOutOfGrammar { key: "tree", .. })
        ));
    }

    /// RF §4.2: "the token splits at its **last** occurrence of the literal
    /// `+sha256:`, which is unambiguous because `<dist_hash>` is exactly 64
    /// lowercase hex" — and "**No version is unrepresentable**", which is what
    /// makes the *last*-occurrence rule necessary rather than decorative.
    #[test]
    fn tool_splits_at_the_last_plus_sha256_so_no_version_is_unrepresentable() {
        let hostile_version = "1.4.0+sha256:beef";
        let hash = "6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db";
        let line = VECTOR.replace(
            &format!("tool=1.4.0+sha256:{hash}"),
            &format!("tool={hostile_version}+sha256:{hash}"),
        );
        let header = Header::parse(&line, ObjectFormat::Sha1).expect("printable ASCII version");
        let (version, digest) = header.tool_parts();
        assert_eq!(version, hostile_version);
        assert_eq!(digest, format!("sha256:{hash}"));
    }

    #[test]
    fn a_tool_token_without_a_64_hex_dist_hash_is_malformed() {
        for bad in [
            "tool=1.4.0",
            "tool=1.4.0+sha256:beef",
            "tool=+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db",
            "tool=1.4.0+sha256:6F49644FDD3009155FE32AB46B9DA846B6645F52A15EB3AA44234C02B1C744DB",
        ] {
            let line = VECTOR.replace(
                "tool=1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db",
                bad,
            );
            assert!(
                matches!(
                    Header::parse(&line, ObjectFormat::Sha1),
                    Err(Malformed::HeaderValueOutOfGrammar { key: "tool", .. })
                ),
                "{bad}"
            );
        }
    }

    /// RF §4.2 field 6: "non-negative decimal, no sign, no leading zero except
    /// `0` itself".
    #[test]
    fn ids_admits_zero_and_no_leading_zero_and_no_sign() {
        let zero = VECTOR.replace("ids=7", "ids=0");
        assert_eq!(
            Header::parse(&zero, ObjectFormat::Sha1)
                .expect("ids=0 is the base-collect-failed shape")
                .ids,
            0
        );
        for bad in ["ids=07", "ids=+7", "ids=-1", "ids=7.0", "ids=seven"] {
            let line = VECTOR.replace("ids=7", bad);
            assert!(
                matches!(
                    Header::parse(&line, ObjectFormat::Sha1),
                    Err(Malformed::HeaderValueOutOfGrammar { key: "ids", .. })
                ),
                "{bad}"
            );
        }
    }

    /// RF §4.2: `true` "is the honest negation, and the collector writes it
    /// rather than omitting the field" — and nothing else is a boolean here.
    #[test]
    fn keys_visible_admits_exactly_true_and_false() {
        let t = VECTOR.replace("keys_visible=false", "keys_visible=true");
        assert!(
            Header::parse(&t, ObjectFormat::Sha1)
                .expect("true is legal")
                .keys_visible
        );
        for bad in ["keys_visible=1", "keys_visible=TRUE", "keys_visible=yes"] {
            let line = VECTOR.replace("keys_visible=false", bad);
            assert!(
                matches!(
                    Header::parse(&line, ObjectFormat::Sha1),
                    Err(Malformed::HeaderValueOutOfGrammar {
                        key: "keys_visible",
                        ..
                    })
                ),
                "{bad}"
            );
        }
    }
}
