//! `manifest.md` §8.7 — the published keyring, reproduced.
//!
//! The three fingerprints below were checked against `ssh-keygen -lf` on
//! OpenSSH before being written here, which is the check `gate-report.md` §8.2
//! exists to remind everyone about: a published fingerprint that is not in the
//! value space of any key sat inside two digests for three review rounds, and
//! only a round-trip catches that.

use spine_manifest::keyring::{Keyring, Lint, Mode};

/// MF §8.7, byte-identical to EV §8.1. 411 bytes, blob
/// `6d4db08390092d7d5d96476eddca6355815bc49f`.
const KEYRING: &str = concat!(
    "alice@example.com namespaces=\"spine-signoff@v1,spine-review@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla\n",
    "bob@example.com namespaces=\"spine-signoff@v1,spine-review@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINZJBgqcpDmx19xO9D29xeFtCCUMyfe/ti+lY7c+rvim\n",
    "ci@example.com namespaces=\"spine-seal@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICE3UkO6VDz+9ag4xQScwsfdP8PHJhLh+QWkIwzUjWze\n",
);

const ALICE_FP: &str = "SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM";
const BOB_FP: &str = "SHA256:V2dasTIGWUnhlaxa7vr3Qmqpe/qYBoxi+C7Cs6yxxEs";
const CI_FP: &str = "SHA256:eQ0ZoC+rlhZstRuvhLXwJlwqLAreBcmnyFedpvPmTVY";

#[test]
fn the_published_keyring_is_411_bytes_with_the_published_blob() {
    assert_eq!(KEYRING.len(), 411);
    assert_eq!(
        spine_canon::git_blob_id(KEYRING.as_bytes(), spine_canon::ObjectFormat::Sha1),
        "6d4db08390092d7d5d96476eddca6355815bc49f"
    );
}

/// MF §8.7's own walk of the lint, assertion by assertion: "three entry lines,
/// no blanks, no comments, no CR; one principal each; `namespaces=` present on
/// all three and the only option; every namespace in the domain; every keytype
/// `ssh-ed25519`; three distinct fingerprints under three distinct principals
/// … two distinct signoff keys, so `mode = team` … Clean."
#[test]
fn it_lints_clean_in_team_mode() {
    let k = Keyring::parse(KEYRING.as_bytes());
    assert!(k.is_clean(), "expected a clean lint, got {:?}", k.findings);
    assert_eq!(k.entries.len(), 3);
    assert_eq!(k.mode, Mode::Team);
}

/// The fingerprints are `ssh-keygen -lf` over `<keytype> <keyblob>`: SHA-256 of
/// the decoded blob, unpadded base64. This is what `reviewer != signer`
/// compares, never the principal (PB §7.2, GR §5.5).
#[test]
fn the_fingerprints_are_the_published_ones() {
    let k = Keyring::parse(KEYRING.as_bytes());
    let fingerprints: Vec<&str> = k.entries.iter().map(|e| e.fingerprint.as_str()).collect();
    assert_eq!(fingerprints, vec![ALICE_FP, BOB_FP, CI_FP]);

    assert_eq!(
        k.by_fingerprint(ALICE_FP).unwrap().principal,
        "alice@example.com"
    );
    assert_eq!(
        k.fingerprints_under("spine-signoff@v1"),
        {
            let mut expected = vec![ALICE_FP, BOB_FP];
            expected.sort_unstable();
            expected
        },
        "two distinct signoff keys is what makes this team mode"
    );
    assert_eq!(k.fingerprints_under("spine-seal@v1"), vec![CI_FP]);
}

/// MF §4.6, DM §7.2: a `signer` node's `roles` attr is the entry's namespaces
/// **ascending by bytes**.
#[test]
fn roles_are_sorted_ascending_by_bytes() {
    let k = Keyring::parse(KEYRING.as_bytes());
    assert_eq!(
        k.entries[0].namespaces,
        vec!["spine-review@v1", "spine-signoff@v1"],
        "the line spells signoff first; the exported order is byte order"
    );
    assert_eq!(k.entries[2].namespaces, vec!["spine-seal@v1"]);
}

fn lints(text: &str) -> Vec<Lint> {
    Keyring::parse(text.as_bytes())
        .findings
        .into_iter()
        .map(|f| f.lint)
        .collect()
}

const ALICE: &str = "alice@example.com namespaces=\"spine-signoff@v1,spine-review@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla";
const BOB: &str = "bob@example.com namespaces=\"spine-signoff@v1,spine-review@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINZJBgqcpDmx19xO9D29xeFtCCUMyfe/ti+lY7c+rvim";
const CI: &str = "ci@example.com namespaces=\"spine-seal@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICE3UkO6VDz+9ag4xQScwsfdP8PHJhLh+QWkIwzUjWze";

#[test]
fn blanks_and_comments_are_accepted_and_a_final_newline_is_optional() {
    let with_noise = format!("# a comment\n\n  \n{ALICE}\n{BOB}\n{CI}");
    let k = Keyring::parse(with_noise.as_bytes());
    assert!(k.is_clean(), "{:?}", k.findings);
    assert_eq!(k.entries.len(), 3);
    // Line numbers are 1-based over the *file*, because DM's signer provenance
    // is `git:<sha>:.spine/allowed_signers:<line>`.
    assert_eq!(k.entries[0].line_no, 4);
}

/// MF §4.1: "The keyring has no canonical byte form … requiring canonical bytes
/// would make re-indenting a gate failure." The opposite of the manifest's rule.
#[test]
fn re_indenting_is_not_a_finding() {
    let indented = format!("   {ALICE}\n\t{BOB}\n{CI}\n");
    assert!(Keyring::parse(indented.as_bytes()).is_clean());
}

#[test]
fn a_cr_anywhere_is_a_finding() {
    let crlf = KEYRING.replace('\n', "\r\n");
    assert!(lints(&crlf).contains(&Lint::KeyringCr));
}

#[test]
fn the_option_lints_each_have_their_own_token() {
    // MF §4.2's `entry` production admits none of these lines, yet §4.4 gives
    // each a distinct status — which is why the parser field-splits first and
    // classifies second.
    let no_ns = format!(
        "dave@example.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla\n{BOB}\n{CI}"
    );
    assert!(lints(&no_ns).contains(&Lint::KeyringNoNamespaces));

    let empty = format!(
        "{}\n{BOB}\n{CI}",
        ALICE.replace(
            "namespaces=\"spine-signoff@v1,spine-review@v1\"",
            "namespaces=\"\"",
        )
    );
    assert!(lints(&empty).contains(&Lint::KeyringNamespaceEmpty));

    // A typo "silently removes a signer's authority while leaving the line
    // looking correct" — so it is refused, never ignored.
    let typo = format!(
        "{}\n{BOB}\n{CI}",
        ALICE.replace("spine-signoff@v1", "spine-signof@v1")
    );
    assert!(lints(&typo).contains(&Lint::KeyringNamespaceUnknown));

    let ca = format!(
        "{}\n{BOB}\n{CI}",
        ALICE.replace("namespaces=", "cert-authority,namespaces=")
    );
    assert!(lints(&ca).contains(&Lint::KeyringCertAuthority));

    // MF §4.6: "Both are commits, not times — the chain is the clock."
    let validity = format!(
        "{}\n{BOB}\n{CI}",
        ALICE.replace("namespaces=", "valid-after=20260101,namespaces=")
    );
    assert!(lints(&validity).contains(&Lint::KeyringValidityOption));

    let unknown = format!(
        "{}\n{BOB}\n{CI}",
        ALICE.replace("namespaces=", "agent-forwarding,namespaces=")
    );
    assert!(lints(&unknown).contains(&Lint::KeyringOptionUnknown));

    // MF §4.2: `ssh-rsa` is absent from the keytype list because OpenSSH 8.2
    // deprecated SHA-1 RSA signatures.
    let rsa = format!("{}\n{BOB}\n{CI}", ALICE.replace("ssh-ed25519", "ssh-rsa"));
    assert!(lints(&rsa).contains(&Lint::KeyringKeytypeUnknown));

    // MF §4.2 R12: one entry, one principal.
    let multi = format!(
        "{}\n{BOB}\n{CI}",
        ALICE.replace("alice@example.com", "alice@example.com,alias@example.com")
    );
    assert!(lints(&multi).contains(&Lint::KeyringMultiPrincipal));
}

/// R17's second limb: "a key blob that is not base64, **or that does not decode
/// to a key of the declared type**". A decode-and-typecheck, not a charset test.
#[test]
fn a_blob_of_the_wrong_key_type_is_caught_not_just_bad_base64() {
    let not_base64 = format!("{}\n{BOB}\n{CI}", ALICE.replace("AAAAC3", "!!!!!!"));
    assert!(lints(&not_base64).contains(&Lint::KeyringKeyNotBase64));

    // Valid base64 for an ed25519 key, declared as an ecdsa key. Charset-only
    // checking passes this; the corpus does not.
    let mistyped = format!(
        "{}\n{BOB}\n{CI}",
        ALICE.replace("ssh-ed25519 AAAA", "ecdsa-sha2-nistp256 AAAA")
    );
    assert!(lints(&mistyped).contains(&Lint::KeyringKeyNotBase64));
}

#[test]
fn the_three_identity_lints_are_distinguished() {
    // Same principal, same key.
    let dup_line = format!("{ALICE}\n{ALICE}\n{BOB}\n{CI}");
    assert!(lints(&dup_line).contains(&Lint::KeyringDuplicateLine));

    // Same principal, different key — an unrepresentable `signer` node
    // (DM §5.2), remedied by enrolling `alice+yubikey@example.com`.
    let dup_principal = format!(
        "{ALICE}\n{}\n{CI}",
        BOB.replace("bob@example.com", "alice@example.com")
    );
    assert!(lints(&dup_principal).contains(&Lint::KeyringDuplicatePrincipal));

    // One key, two principals — "would satisfy `reviewer != signer` under one
    // name and fail under the other".
    let two_principals = format!(
        "{ALICE}\n{}\n{CI}",
        ALICE.replace("alice@example.com", "alias@example.com")
    );
    assert!(lints(&two_principals).contains(&Lint::KeyringKeyTwoPrincipals));
}

/// MF §4.5 R20/R21: both seal lints are **team-only**, and `keyring-seal-mixed`
/// is refused in either direction.
#[test]
fn the_seal_lints_are_team_only_and_bidirectional() {
    // Team, seal key also holding a human namespace.
    let mixed = format!(
        "{ALICE}\n{BOB}\n{}",
        CI.replace(
            "namespaces=\"spine-seal@v1\"",
            "namespaces=\"spine-review@v1,spine-seal@v1\""
        )
    );
    let k = Keyring::parse(mixed.as_bytes());
    assert_eq!(k.mode, Mode::Team);
    assert!(k.findings.iter().any(|f| f.lint == Lint::KeyringSealMixed));

    // Team, no seal principal at all.
    let no_seal = format!("{ALICE}\n{BOB}\n");
    let k = Keyring::parse(no_seal.as_bytes());
    assert_eq!(k.mode, Mode::Team);
    assert!(k.findings.iter().any(|f| f.lint == Lint::KeyringNoSeal));

    // Solo: one signoff key holding all three namespaces. "In solo mode the
    // rule is inverted by definition", so neither seal lint fires.
    let solo = ALICE.replace(
        "namespaces=\"spine-signoff@v1,spine-review@v1\"",
        "namespaces=\"spine-signoff@v1,spine-review@v1,spine-seal@v1\"",
    );
    let k = Keyring::parse(solo.as_bytes());
    assert_eq!(k.mode, Mode::Solo);
    assert!(
        k.is_clean(),
        "a solo keyring holding all three namespaces is clean, got {:?}",
        k.findings
    );
}

/// MF §4.5: mode is the count of distinct signoff **fingerprints**, never the
/// count of lines and never `C-A1`.
#[test]
fn mode_counts_distinct_fingerprints_not_lines() {
    // One human with two enrolled principals is still one signoff *key* — but
    // that is `keyring-key-two-principals`, so the reachable shape is one
    // principal. A single signoff key plus a seal key is solo.
    let solo = format!("{ALICE}\n{CI}\n");
    assert_eq!(Keyring::parse(solo.as_bytes()).mode, Mode::Solo);

    let team = format!("{ALICE}\n{BOB}\n{CI}\n");
    assert_eq!(Keyring::parse(team.as_bytes()).mode, Mode::Team);
}

#[test]
fn an_absent_or_entryless_keyring_is_its_own_finding() {
    assert_eq!(
        Keyring::missing().findings[0].lint,
        Lint::KeyringMissing,
        "there is no authority without it"
    );
    assert!(lints("# only a comment\n").contains(&Lint::KeyringEmpty));
    assert!(lints("").contains(&Lint::KeyringEmpty));
}

/// MF §4.2: a trailing comment after the blob "is where `ssh-keygen` puts a
/// key's own comment, and humans put names there" — accepted and ignored.
#[test]
fn a_trailing_comment_is_accepted_and_ignored() {
    let commented = format!("{ALICE} alice's yubikey\n{BOB}\n{CI}\n");
    let k = Keyring::parse(commented.as_bytes());
    assert!(k.is_clean(), "{:?}", k.findings);
    assert_eq!(k.entries[0].fingerprint, ALICE_FP);
}
