//! CI §3.3's render tokens, CI §3.4's substitution order, and the byte scan
//! that refuses a surviving one.
//!
//! Three rules, and each answers a way two implementations could diverge:
//!
//! - **Substitute literally, once, and never recursively.** "Every occurrence
//!   of a token is replaced by the value's bytes, and no substituted value is
//!   ever rescanned for tokens. The render is a function of the table, not of
//!   the order the table is walked." So this is one left-to-right pass with
//!   simultaneous matching, not four sequential `replace` calls.
//! - **The scan runs after every substitution and before every write**, and one
//!   failure refuses the **whole** plan. "A repository half-scaffolded by a bad
//!   release is worse than one not scaffolded at all."
//! - **The scan is a byte scan.** "It re-parses no YAML, does not know which
//!   template produced the bytes, and gives the same answer on every platform."

use crate::release::ReleaseManifest;
use core::fmt;

/// CI §3.3's tokens. Exactly these rows and no others (CI §3.4 step 2).
pub const DIST_BASE: &str = "@@DIST_BASE@@";
pub const PIN_CHECKOUT: &str = "PIN_CHECKOUT";
pub const PIN_UPLOAD_ARTIFACT: &str = "PIN_UPLOAD_ARTIFACT";
pub const PIN_DOWNLOAD_ARTIFACT: &str = "PIN_DOWNLOAD_ARTIFACT";

/// The three `PIN_` literals the byte scan looks for.
pub const PIN_TOKENS: [&str; 3] = [PIN_CHECKOUT, PIN_UPLOAD_ARTIFACT, PIN_DOWNLOAD_ARTIFACT];

/// The substitution table for one repository's render.
#[derive(Debug, Clone)]
pub struct Table {
    rows: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// CI §3.4: the rendered bytes still carry `@@` or a `PIN_` literal. "The
    /// whole plan is `REFUSE` and nothing is written."
    UnsubstitutedToken { token: String, at: usize },
    /// CI §3.4's one accepted residual: a trunk name that itself spells `@@` or
    /// one of the three `PIN_` literals renders a conforming file the scan then
    /// refuses, so `init` refuses the **name** where it is given instead.
    TrunkNameCollidesWithToken(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::UnsubstitutedToken { token, at } => {
                write!(f, "unsubstituted-token: {token:?} survives at byte {at}")
            }
            RenderError::TrunkNameCollidesWithToken(name) => {
                write!(f, "trunk-name-collides-with-token: {name:?}")
            }
        }
    }
}

impl core::error::Error for RenderError {}

impl Table {
    /// CI §3.4 step 2: "exactly §3.3's rows and no others".
    ///
    /// The trunk row replaces the literal `main` that every CI template carries
    /// as its trunk placeholder (CI §3.3's third row).
    pub fn build(release: &ReleaseManifest, trunk: &str) -> Result<Self, RenderError> {
        check_trunk(trunk)?;
        Ok(Table {
            rows: vec![
                (DIST_BASE.to_string(), release.dist_base.clone()),
                (
                    PIN_CHECKOUT.to_string(),
                    release.commit("checkout").unwrap_or_default().to_string(),
                ),
                (
                    PIN_UPLOAD_ARTIFACT.to_string(),
                    release
                        .commit("upload_artifact")
                        .unwrap_or_default()
                        .to_string(),
                ),
                (
                    PIN_DOWNLOAD_ARTIFACT.to_string(),
                    release
                        .commit("download_artifact")
                        .unwrap_or_default()
                        .to_string(),
                ),
                ("main".to_string(), trunk.to_string()),
            ],
        })
    }

    /// One pass, simultaneous matching, longest token first at each position.
    ///
    /// Four sequential `String::replace` calls would give a different answer
    /// whenever a substituted value contains another token's literal, and the
    /// answer would depend on the walk order — which CI §3.4 step 3 forbids by
    /// name.
    pub fn render(&self, template: &str) -> String {
        // Longest-first so no token is a prefix of another's match. (None of
        // the five is a prefix of another today; sorting keeps that from being
        // load-bearing.)
        let mut ordered: Vec<&(String, String)> = self.rows.iter().collect();
        ordered.sort_by_key(|(token, _)| core::cmp::Reverse(token.len()));

        let bytes = template.as_bytes();
        let mut out = String::with_capacity(template.len());
        let mut index = 0;
        'outer: while index < bytes.len() {
            for (token, value) in &ordered {
                if bytes[index..].starts_with(token.as_bytes()) {
                    out.push_str(value);
                    index += token.len();
                    continue 'outer;
                }
            }
            // Not a token boundary: copy one whole character.
            let ch = template[index..].chars().next().expect("in bounds");
            out.push(ch);
            index += ch.len_utf8();
        }
        out
    }

    /// Render and scan in one call, which is the only order CI §3.4 permits.
    pub fn render_checked(&self, template: &str) -> Result<String, RenderError> {
        let rendered = self.render(template);
        scan(&rendered)?;
        Ok(rendered)
    }
}

/// CI §3.4's token-free check, mechanically: the rendered bytes must contain no
/// `@@` "in any context", and none of the three `PIN_` literals.
pub fn scan(rendered: &str) -> Result<(), RenderError> {
    if let Some(at) = rendered.find("@@") {
        return Err(RenderError::UnsubstitutedToken {
            token: "@@".to_string(),
            at,
        });
    }
    for token in PIN_TOKENS {
        if let Some(at) = rendered.find(token) {
            return Err(RenderError::UnsubstitutedToken {
                token: token.to_string(),
                at,
            });
        }
    }
    Ok(())
}

/// CI §3.4's residual, refused where it is given rather than where it bites.
///
/// "The alternative — scanning before the trunk substitution — makes the
/// conformance test depend on substitution order, and an order-dependent test
/// is one two implementations can disagree about while both believing they
/// conform."
pub fn check_trunk(trunk: &str) -> Result<(), RenderError> {
    if trunk.contains("@@") || PIN_TOKENS.iter().any(|t| trunk.contains(t)) {
        return Err(RenderError::TrunkNameCollidesWithToken(trunk.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release::ReleaseManifest;

    const FIXTURE: &str = r#"{
      "release_manifest_version": 1,
      "version": "1.4.0",
      "dist_base": "https://dist.example.invalid/spine",
      "actions": {
        "checkout":          { "repo": "actions/checkout",          "commit": "11bd71901bbe5b1630ceea73d27597364c9af683" },
        "upload_artifact":   { "repo": "actions/upload-artifact",   "commit": "ea165f8d65b6e75b540449e92b4886f43607fa02" },
        "download_artifact": { "repo": "actions/download-artifact", "commit": "fa0a91b85d4f404e444e00e005971372dc801d16" }
      }
    }"#;

    fn table(trunk: &str) -> Table {
        let release = ReleaseManifest::parse(FIXTURE.as_bytes()).unwrap();
        Table::build(&release, trunk).unwrap()
    }

    #[test]
    fn every_row_of_the_substitution_table_lands() {
        let rendered = table("main")
            .render_checked(
                "curl @@DIST_BASE@@/x\n\
                 uses: actions/checkout@PIN_CHECKOUT\n\
                 uses: actions/upload-artifact@PIN_UPLOAD_ARTIFACT\n\
                 uses: actions/download-artifact@PIN_DOWNLOAD_ARTIFACT\n\
                 branches: [main]\n",
            )
            .unwrap();
        assert_eq!(
            rendered,
            "curl https://dist.example.invalid/spine/x\n\
             uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683\n\
             uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02\n\
             uses: actions/download-artifact@fa0a91b85d4f404e444e00e005971372dc801d16\n\
             branches: [main]\n"
        );
    }

    #[test]
    fn the_trunk_row_substitutes_the_literal_main() {
        let rendered = table("trunk")
            .render_checked("branches: [main]\nref: main\n")
            .unwrap();
        assert_eq!(rendered, "branches: [trunk]\nref: trunk\n");
    }

    /// CI §3.4 step 3: "no substituted value is ever rescanned for tokens".
    ///
    /// A trunk literally named `PIN_CHECKOUT` is refused at `--trunk` — but the
    /// non-recursion property has to hold for any value, so this exercises it
    /// through a value that legitimately contains another token's text.
    #[test]
    fn substitution_is_never_recursive() {
        // A trunk named `main-main`: a recursive implementation would rewrite
        // the substituted value's own `main` occurrences and loop or double.
        let rendered = table("main-main").render("branches: [main]\n");
        assert_eq!(
            rendered, "branches: [main-main]\n",
            "the substituted value must not be rescanned"
        );
    }

    /// "The render is a function of the table, not of the order the table is
    /// walked." One pass with simultaneous matching is what makes that true.
    #[test]
    fn the_render_does_not_depend_on_walk_order() {
        // If this were four sequential replaces, a `dist_base` containing
        // `PIN_CHECKOUT` would be rewritten by the later pass. `dist_base`
        // cannot contain `@`, but it can certainly contain that literal.
        let release = ReleaseManifest::parse(
            &FIXTURE
                .replace(
                    "https://dist.example.invalid/spine",
                    "https://dist.example.invalid/PIN_CHECKOUT",
                )
                .into_bytes(),
        )
        .unwrap();
        let table = Table::build(&release, "main").unwrap();
        let rendered = table.render("@@DIST_BASE@@\n");
        assert_eq!(
            rendered, "https://dist.example.invalid/PIN_CHECKOUT\n",
            "a substituted value is output, never re-entered"
        );
        // And the scan then correctly refuses it, because the *rendered bytes*
        // carry a PIN_ literal — which is the fail-closed direction.
        assert!(matches!(
            scan(&rendered),
            Err(RenderError::UnsubstitutedToken { .. })
        ));
    }

    /// "no occurrence of `@@` — two `U+0040`, **in any context**".
    #[test]
    fn the_scan_refuses_a_bare_double_at_anywhere() {
        assert!(scan("nothing to see").is_ok());
        for bytes in [
            "@@DIST_BASE@@",
            "a @@ b",
            "# comment with @@ in it",
            "uses: actions/checkout@@v4",
        ] {
            assert!(
                matches!(scan(bytes), Err(RenderError::UnsubstitutedToken { .. })),
                "{bytes:?} should be refused"
            );
        }
        // One `@` is fine — it is how every `uses:` line is spelled.
        assert!(scan("uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683").is_ok());
    }

    #[test]
    fn the_scan_refuses_each_pin_literal() {
        for token in PIN_TOKENS {
            let rendered = format!("uses: actions/checkout@{token}");
            match scan(&rendered) {
                Err(RenderError::UnsubstitutedToken { token: found, .. }) => {
                    assert_eq!(found, token);
                }
                other => panic!("expected a refusal for {token}, got {other:?}"),
            }
        }
    }

    /// CI §3.4's residual, refused at `--trunk` rather than at the scan: "a
    /// trunk literally named with `@@` in it, or named for one of the three
    /// `PIN_` literals, renders a *conforming* file that the scan then
    /// refuses."
    #[test]
    fn a_trunk_name_that_collides_with_a_token_is_refused_where_it_is_given() {
        let release = ReleaseManifest::parse(FIXTURE.as_bytes()).unwrap();
        for name in ["@@", "release@@2026", "PIN_CHECKOUT", "x-PIN_UPLOAD_ARTIFACT"] {
            assert_eq!(
                Table::build(&release, name).unwrap_err(),
                RenderError::TrunkNameCollidesWithToken(name.to_string()),
                "{name:?} should be refused at --trunk"
            );
        }
        // The manifest's grammar is unchanged: these are names git accepts.
        assert!(spine_manifest::grammar::check_branch_name("release@@2026").is_ok());
    }

    /// The scan precedes every write and one failure refuses the whole plan.
    /// `render_checked` is the only entry point that can produce bytes, so the
    /// order cannot be got wrong by a caller.
    #[test]
    fn render_checked_yields_nothing_when_a_token_survives() {
        // A template carrying a token the table has no row for.
        let result = table("main").render_checked("uses: foo@PIN_SOMETHING_ELSE\n@@LEFTOVER@@\n");
        assert!(result.is_err());
    }

    #[test]
    fn a_template_with_no_tokens_is_returned_unchanged() {
        let template = "#!/bin/sh\nset -eu\necho hello\n";
        assert_eq!(table("main").render(template), template);
    }

    /// Multi-byte content must survive the byte-oriented scan loop intact.
    #[test]
    fn non_ascii_bytes_pass_through_unharmed() {
        let template = "# Constitution — myrepo · v1\n@@DIST_BASE@@\n";
        let rendered = table("main").render_checked(template).unwrap();
        assert_eq!(
            rendered,
            "# Constitution — myrepo · v1\nhttps://dist.example.invalid/spine\n"
        );
    }
}
