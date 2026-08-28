//! The envelope as bytes — EV §2's three regions, the fence, the 16 KiB cap,
//! and the derived subject.
//!
//! PB §5.5: "The disposal rule deletes the file. It must not delete the truth.
//! The record is a git object: the landing commit." Everything here reads that
//! one byte string and refuses rather than repairs — "Detection, never repair"
//! (EV §2.2).

use crate::digest;
use crate::payload::{Event, Gates, Lane, Seal, Strategy};
use crate::refusal::{EnvelopeError, Refusal};
use crate::trailer::{TrailerName, check_envelope_version, is_spine_line, parse_line, show};
use spine_canon::ObjectFormat;

/// EV §2.9's comparison value. "PB §5.5, as amended 2026-08-26: 'The fenced
/// intent plus the signed lines are capped at 16 KiB.'"
pub const CAP: usize = 16384;

const FENCE_BEGIN: &[u8] = b"-----BEGIN SPINE-INTENT ";
const FENCE_END: &[u8] = b"-----END SPINE-INTENT-----";

/// EV §2.6's fenced intent block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fence {
    pub blob: String,
    /// "`bytes=` counts **bytes, not characters**. Vector A's intent is 765
    /// bytes and 762 characters" (EV §2.6).
    pub bytes: usize,
    pub body: Vec<u8>,
}

/// EV §13.10's five landing shapes. "The shape is read from `Spine-Event` and
/// `Spine-Lane`, which are mandatory on every landing, sit above the seal, and
/// are inside `envelope=` — so the input a verifier selects the derivation by
/// is itself sealed."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Gated,
    Tombstone,
    Quick,
    /// "a lifecycle landing rides the quick lane" (EV §13.10) — `Spine-Lane:
    /// quick` with a copied `Spine-Upgrade`.
    Lifecycle,
    Reseal,
}

/// A parsed landing commit message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    message: Vec<u8>,
    subject: Vec<u8>,
    fence: Option<Fence>,
    /// The trailer block, in message order, through `Spine-Seal-Sig`.
    trailers: Vec<(TrailerName, Vec<u8>)>,
    /// EV §2.8's permitted case: non-`Spine-*` lines after `Spine-Seal-Sig`.
    foreign: Vec<Vec<u8>>,
}

impl Envelope {
    /// Parse and structurally validate a commit message, per EV §2 and the
    /// conformance items §18.1–§18.9.
    ///
    /// Read it out of git with `git cat-file commit`, "never `git log`, so no
    /// cleanup rule ever touches the fenced bytes" (PB §5.5), and strip the
    /// commit headers first: this function takes the message alone.
    pub fn parse(message: &[u8]) -> Result<Self, EnvelopeError> {
        // EV §2.2: "`spine check --land` refuses to build a message containing
        // `0x0D` or `0x00` anywhere, and G9 refuses a landing whose message
        // contains either. Both refusals are `envelope-malformed`."
        if let Some(i) = message.iter().position(|&b| b == 0x0D || b == 0x00) {
            return Err(EnvelopeError::malformed(format!(
                "message holds 0x{:02X} at byte {i}",
                message[i]
            )));
        }
        if message.last() != Some(&b'\n') {
            return Err(EnvelopeError::malformed(
                "message does not end with 0x0A (EV §2.1)",
            ));
        }

        let mut pos = 0usize;
        let subject = take_line(message, &mut pos)?.to_vec();
        // "**Blank line** — exactly one, exactly `0x0A` on its own."
        let blank = take_line(message, &mut pos)?;
        if !blank.is_empty() {
            return Err(EnvelopeError::malformed(
                "no blank line after the subject (EV §2.1)",
            ));
        }

        let fence = if message[pos..].starts_with(FENCE_BEGIN) {
            let fence = parse_fence(message, &mut pos)?;
            let blank = take_line(message, &mut pos)?;
            if !blank.is_empty() {
                return Err(EnvelopeError::malformed(
                    "no blank line after the fenced intent (EV §2.1)",
                ));
            }
            Some(fence)
        } else {
            None
        };

        // The trailer block: "one `Spine-*` line per line, no blank lines inside
        // it, ending with `Spine-Seal-Sig`" (EV §2.1).
        let mut trailers: Vec<(TrailerName, Vec<u8>)> = Vec::new();
        let mut seen_seal_sig = false;
        let mut foreign: Vec<Vec<u8>> = Vec::new();
        while pos < message.len() {
            let line = take_line(message, &mut pos)?;
            if seen_seal_sig {
                // EV §2.8: a non-`Spine-*` line after the seal's `-Sig` is
                // "outside `envelope=`, outside every signature, and permitted.
                // This is the case PB §5.5 describes and it is the only
                // permitted one." A `Spine-*` line there is refused — PB §5.5's
                // word "ignored" is wrong for it (EV §14 D7).
                if is_spine_line(line) {
                    return Err(EnvelopeError::malformed(format!(
                        "Spine-* line below the seal: {}",
                        show(line)
                    )));
                }
                foreign.push(line.to_vec());
                continue;
            }
            if line.is_empty() {
                return Err(EnvelopeError::malformed(
                    "blank line inside the trailer block (EV §2.8)",
                ));
            }
            if !is_spine_line(line) {
                return Err(EnvelopeError::malformed(format!(
                    "non-Spine-* line inside the trailer block: {}",
                    show(line)
                )));
            }
            let t = parse_line(line)?;
            if t.name == TrailerName::Envelope {
                check_envelope_version(t.payload)?;
            }
            seen_seal_sig = t.name == TrailerName::SealSig;
            trailers.push((t.name, t.payload.to_vec()));
        }

        let env = Envelope {
            message: message.to_vec(),
            subject,
            fence,
            trailers,
            foreign,
        };
        env.check_structure()?;
        Ok(env)
    }

    pub fn message(&self) -> &[u8] {
        &self.message
    }

    pub fn subject(&self) -> &[u8] {
        &self.subject
    }

    pub fn fence(&self) -> Option<&Fence> {
        self.fence.as_ref()
    }

    pub fn foreign_trailers(&self) -> &[Vec<u8>] {
        &self.foreign
    }

    pub fn trailers(&self) -> &[(TrailerName, Vec<u8>)] {
        &self.trailers
    }

    /// The payload of the single line with this name, or `None`.
    pub fn first(&self, name: TrailerName) -> Option<&[u8]> {
        self.trailers
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, p)| p.as_slice())
    }

    pub fn all(&self, name: TrailerName) -> Vec<&[u8]> {
        self.trailers
            .iter()
            .filter(|(n, _)| *n == name)
            .map(|(_, p)| p.as_slice())
            .collect()
    }

    pub fn seal(&self) -> Result<Seal, EnvelopeError> {
        let payload = self
            .first(TrailerName::Seal)
            .ok_or_else(|| EnvelopeError::malformed("no Spine-Seal line"))?;
        Seal::parse(payload)
    }

    pub fn event(&self) -> Result<Event, EnvelopeError> {
        let p = self
            .first(TrailerName::Event)
            .ok_or_else(|| EnvelopeError::malformed("no Spine-Event line"))?;
        Event::parse(p).ok_or_else(|| {
            EnvelopeError::malformed(format!("Spine-Event outside its set: {}", show(p)))
        })
    }

    pub fn lane(&self) -> Result<Lane, EnvelopeError> {
        let p = self
            .first(TrailerName::Lane)
            .ok_or_else(|| EnvelopeError::malformed("no Spine-Lane line"))?;
        Lane::parse(p).ok_or_else(|| {
            EnvelopeError::malformed(format!("Spine-Lane outside its set: {}", show(p)))
        })
    }

    pub fn strategy(&self) -> Result<Strategy, EnvelopeError> {
        let p = self
            .first(TrailerName::Strategy)
            .ok_or_else(|| EnvelopeError::malformed("no Spine-Strategy line"))?;
        Strategy::parse(p).ok_or_else(|| {
            EnvelopeError::malformed(format!("Spine-Strategy outside its set: {}", show(p)))
        })
    }

    pub fn gates(&self) -> Result<Gates, EnvelopeError> {
        let p = self
            .first(TrailerName::Gates)
            .ok_or_else(|| EnvelopeError::malformed("no Spine-Gates line"))?;
        Gates::parse(p)
    }

    /// EV §13.10's shape selector, read from sealed inputs only.
    pub fn shape(&self) -> Result<Shape, EnvelopeError> {
        Ok(match (self.event()?, self.lane()?) {
            (Event::Reseal, _) => Shape::Reseal,
            (Event::Withdraw, _) => Shape::Tombstone,
            (_, Lane::Gated) => Shape::Gated,
            (_, Lane::Quick) => {
                if self.first(TrailerName::Upgrade).is_some() {
                    Shape::Lifecycle
                } else {
                    Shape::Quick
                }
            }
        })
    }

    /// `envelope=` recomputed from this message (EV §3.1).
    pub fn envelope_digest(&self) -> Result<String, EnvelopeError> {
        digest::envelope_digest(&self.message)
    }

    /// `freeze=` recomputed from this message's own copied lines — G9's squash
    /// branch (EV §4.5). Under merge the envelope carries no manifest and the
    /// digest is taken from the approval commit instead.
    pub fn freeze_digest(&self) -> String {
        digest::freeze_digest(&self.message)
    }

    /// EV §18 item 11 and §12's `digest-mismatch`: the seal's `envelope=` must
    /// equal the recomputation. "This is the case the digest exists for"
    /// (EV §3.4) — a provider that rebuilt the commit under PB §5.4
    /// configuration (b) with a different trailer set fails here.
    pub fn check_envelope_digest(&self) -> Result<(), EnvelopeError> {
        let sealed = self.seal()?.envelope;
        let computed = self.envelope_digest()?;
        if sealed == computed {
            Ok(())
        } else {
            Err(EnvelopeError::new(
                Refusal::DigestMismatch,
                format!("seal says envelope={sealed}, recomputed {computed}"),
            ))
        }
    }

    /// EV §2.9's capped quantity: "the byte length of the whole commit message,
    /// **excluding every `Spine-Frozen` and `Spine-Test` line together with its
    /// terminating `0x0A`**".
    ///
    /// A superset of PB §5.5's literal "fenced intent plus the signed lines",
    /// deliberately: "A superset is the safe direction: it can only refuse
    /// earlier, never later, and it is the quantity that actually bounds the
    /// object."
    pub fn capped_quantity(&self) -> usize {
        let frozen = format!("{}: ", TrailerName::Frozen);
        let test = format!("{}: ", TrailerName::Test);
        digest::lines(&self.message)
            .iter()
            .filter(|l| !l.starts_with(frozen.as_bytes()) && !l.starts_with(test.as_bytes()))
            .map(|l| l.len() + 1)
            .sum()
    }

    /// EV §2.9, as the owner settled it on 2026-08-26: "**A reseal envelope is
    /// not capped.** … for that shape no cap is evaluated, no projection is
    /// made at `--approve`, and `envelope-too-large` (§12) is never raised."
    ///
    /// The exemption is read from `Spine-Event`, which "sits above the seal and
    /// is inside `envelope=` … so the exemption needs no parse of any payload
    /// and cannot be claimed by a landing that is not a reseal."
    pub fn check_cap(&self) -> Result<(), EnvelopeError> {
        if self.event()? == Event::Reseal {
            return Ok(());
        }
        let n = self.capped_quantity();
        if n > CAP {
            return Err(EnvelopeError::new(
                Refusal::EnvelopeTooLarge,
                format!("{n} of {CAP}"),
            ));
        }
        Ok(())
    }

    /// EV §13.10's derivation, for the three shapes it is defined for.
    ///
    /// `None` for the two quick-lane shapes: "the summary is free text", and
    /// "Nothing else about a quick-lane subject is decidable, and G9 does not
    /// pretend otherwise."
    pub fn derive_subject(&self) -> Result<Option<Vec<u8>>, EnvelopeError> {
        match self.shape()? {
            // "the fenced block's first line with the leading `# ` removed" —
            // a derivation and not a template: taking the whole first line
            // minus its `# ` "needs no parse, so two implementations cannot
            // disagree about where the id ends" (EV §13.10).
            Shape::Gated | Shape::Tombstone => {
                let fence = self.fence.as_ref().ok_or_else(|| {
                    EnvelopeError::new(
                        Refusal::SubjectMismatch,
                        "no fenced intent to derive the subject from",
                    )
                })?;
                let first = fence.body.split(|&b| b == b'\n').next().unwrap_or(&[]);
                let title = first.strip_prefix(b"# ").ok_or_else(|| {
                    EnvelopeError::new(
                        Refusal::SubjectMismatch,
                        "the fenced intent's first line does not begin '# '",
                    )
                })?;
                Ok(Some(title.to_vec()))
            }
            // "`reseal: ` and the full object id of the orphan tip `O`, which is
            // the seal's `head=`" — full, never abbreviated: `core.abbrev` is a
            // per-clone setting, so an abbreviation would make a derived
            // subject a function of the reader's configuration (EV §13.10).
            Shape::Reseal => {
                let mut s = b"reseal: ".to_vec();
                s.extend_from_slice(self.seal()?.head.as_bytes());
                Ok(Some(s))
            }
            Shape::Quick | Shape::Lifecycle => Ok(None),
        }
    }

    /// EV §13.10, "What G9 checks, exactly": a byte-for-byte comparison on the
    /// three derivable shapes, and on the two quick-lane ones only that "the
    /// line begins with the seven bytes `quick: `, that at least one further
    /// byte follows".
    pub fn check_subject(&self) -> Result<(), EnvelopeError> {
        match self.derive_subject()? {
            Some(derived) => {
                if derived == self.subject {
                    Ok(())
                } else {
                    Err(EnvelopeError::new(
                        Refusal::SubjectMismatch,
                        format!(
                            "subject is {:?}, derivation gives {:?}",
                            show(&self.subject),
                            show(&derived)
                        ),
                    ))
                }
            }
            None => {
                let rest = self.subject.strip_prefix(b"quick: ").ok_or_else(|| {
                    EnvelopeError::new(
                        Refusal::SubjectMismatch,
                        "a quick-lane subject does not begin 'quick: '",
                    )
                })?;
                if rest.is_empty() {
                    return Err(EnvelopeError::new(
                        Refusal::SubjectMismatch,
                        "a quick-lane subject has an empty summary",
                    ));
                }
                Ok(())
            }
        }
    }

    /// EV §2.4, §2.8 and §18 items 3–9.
    fn check_structure(&self) -> Result<(), EnvelopeError> {
        // "**Exactly one `Spine-Seal`, exactly one `Spine-Seal-Sig`, and the
        // `-Sig` immediately after the seal.** A second `Spine-Seal` line
        // anywhere is `envelope-malformed`. This closes the append-a-second-seal
        // shape before any digest is consulted."
        let seals = self.all(TrailerName::Seal).len();
        let seal_sigs = self.all(TrailerName::SealSig).len();
        if seals != 1 || seal_sigs != 1 {
            return Err(EnvelopeError::malformed(format!(
                "{seals} Spine-Seal and {seal_sigs} Spine-Seal-Sig lines; each must appear once"
            )));
        }
        match self.trailers.last().map(|(n, _)| *n) {
            Some(TrailerName::SealSig) => {}
            _ => {
                return Err(EnvelopeError::malformed(
                    "the trailer block does not end with Spine-Seal-Sig",
                ));
            }
        }

        // Cardinality is **per name**, never per rank: `Spine-Signoff` and
        // `Spine-Signoff-Sig` share rank 5 and both appear on the same landing,
        // so a rank-indexed counter would refuse every gated envelope.
        let mut counts = [0usize; TrailerName::ALL.len()];
        let mut prev_rank = 0u8;
        for (i, (name, _)) in self.trailers.iter().enumerate() {
            // "Rank ascending; a trailer absent from a landing consumes no
            // rank" (EV §2.4), and §18 item 6: "Trailer ranks are
            // non-decreasing".
            if name.rank() < prev_rank {
                return Err(EnvelopeError::malformed(format!(
                    "{name} is out of rank order"
                )));
            }
            prev_rank = name.rank();

            // "a `-Sig` line immediately follows the line it signs, on the next
            // line, with nothing between."
            if let Some(signed) = name.signs() {
                match i.checked_sub(1).and_then(|j| self.trailers.get(j)) {
                    Some((prev, _)) if *prev == signed => {}
                    _ => {
                        return Err(EnvelopeError::malformed(format!(
                            "{name} does not immediately follow {signed}"
                        )));
                    }
                }
            }
            let slot = TrailerName::ALL
                .iter()
                .position(|n| n == name)
                .expect("closed set");
            counts[slot] += 1;
            if !name.is_repeatable() && counts[slot] > 1 {
                return Err(EnvelopeError::malformed(format!(
                    "{name} appears more than once"
                )));
            }
        }

        // EV §3.1: "ranks 1, 2, 3, 14 and 15 are mandatory on every landing".
        for required in [
            TrailerName::Envelope,
            TrailerName::Event,
            TrailerName::Lane,
            TrailerName::Gates,
            TrailerName::Strategy,
        ] {
            if self.first(required).is_none() {
                return Err(EnvelopeError::malformed(format!("no {required} line")));
            }
        }

        // EV §18 item 7: "repeated `Spine-Frozen` and `Spine-Test` lines are in
        // §4.2's sort order" — so that "the digest recomputes from the copied
        // lines by `sort`-free concatenation and a reader can check the order as
        // well as the value" (EV §2.4).
        let all = digest::lines(&self.message);
        let manifest = digest::freeze_lines(&all);
        for pair in manifest.windows(2) {
            if digest::freeze_cmp(pair[0], pair[1]) != core::cmp::Ordering::Less {
                return Err(EnvelopeError::malformed(format!(
                    "the frozen manifest is not in freeze= order at {}",
                    show(pair[1])
                )));
            }
        }

        self.check_payloads()?;

        // EV §18 item 9: "Present iff the landing is gated or a tombstone."
        let shape = self.shape()?;
        let wants_fence = matches!(shape, Shape::Gated | Shape::Tombstone);
        if wants_fence != self.fence.is_some() {
            return Err(EnvelopeError::malformed(format!(
                "a {shape:?} landing {} a fenced intent",
                if wants_fence {
                    "needs"
                } else {
                    "must not carry"
                }
            )));
        }
        Ok(())
    }

    /// EV §18 items 3, 8, 14 and 15: every payload against PB §11's grammar.
    ///
    /// Structural validity and payload validity are one refusal —
    /// `envelope-malformed` covers "unknown or malformed `Spine-*` name, wrong
    /// field order" alike (EV §12) — so this runs inside [`Envelope::parse`]
    /// rather than waiting for a caller to ask.
    fn check_payloads(&self) -> Result<(), EnvelopeError> {
        for (name, payload) in &self.trailers {
            if name.is_sig() {
                // EV §18 item 15: "Each `-Sig` payload is a single unbroken
                // base64 run with no armor and no `0x0A`."
                if !payload
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'+' || *b == b'/' || *b == b'=')
                {
                    return Err(EnvelopeError::malformed(format!(
                        "{name} is not one unbroken base64 run"
                    )));
                }
                continue;
            }
            match name {
                TrailerName::Envelope => check_envelope_version(payload)?,
                TrailerName::Event => {
                    self.event()?;
                }
                TrailerName::Lane => {
                    self.lane()?;
                }
                TrailerName::Strategy => {
                    self.strategy()?;
                }
                TrailerName::Gates => {
                    Gates::parse(payload)?;
                }
                TrailerName::Signoff => {
                    crate::payload::Signoff::parse(payload)?;
                }
                TrailerName::Approve => {
                    crate::payload::Approve::parse(payload)?;
                }
                TrailerName::Reopen => {
                    crate::payload::Reopen::parse(payload)?;
                }
                TrailerName::Withdraw => {
                    crate::payload::Withdraw::parse(payload)?;
                }
                TrailerName::Upgrade => {
                    crate::payload::Upgrade::parse(payload)?;
                }
                TrailerName::Review => {
                    crate::payload::Review::parse(payload)?;
                }
                TrailerName::Seal => {
                    Seal::parse(payload)?;
                }
                TrailerName::Frozen => {
                    crate::payload::Frozen::parse(payload)?;
                }
                TrailerName::Test => {
                    crate::payload::Test::parse(payload)?;
                }
                // EV §7 rule 7 governs the three bare-oid payloads.
                TrailerName::Approval => {
                    crate::trailer::oid(payload, "the Spine-Approval commit")?;
                }
                TrailerName::Reverts => {
                    crate::trailer::oid(payload, "Spine-Reverts")?;
                }
                TrailerName::TrustRootPrev => {
                    crate::trailer::oid(payload, "Spine-Trust-Root-Prev")?;
                }
                // `Spine-Intent` and `Spine-Supersedes` carry a bare intent id,
                // whose grammar is `intent-doc.md`'s and not this crate's; the
                // non-empty, CR-free, NUL-free payload rule of EV §2.3 is all
                // this document fixes about them.
                TrailerName::Intent | TrailerName::Supersedes => {}
                _ => {}
            }
        }
        Ok(())
    }
}

/// EV §2.2's line, with its terminator consumed.
fn take_line<'a>(message: &'a [u8], pos: &mut usize) -> Result<&'a [u8], EnvelopeError> {
    let rest = &message[*pos..];
    let nl = rest
        .iter()
        .position(|&b| b == b'\n')
        .ok_or_else(|| EnvelopeError::malformed("message ends without a terminator"))?;
    *pos += nl + 1;
    Ok(&rest[..nl])
}

/// EV §2.6, including its refusal to search for the END delimiter: "A parser
/// reads exactly `n` bytes … so an intent that somehow contained the END line as
/// text could not truncate the block."
fn parse_fence(message: &[u8], pos: &mut usize) -> Result<Fence, EnvelopeError> {
    let header = take_line(message, pos)?;
    let inner = header
        .strip_prefix(FENCE_BEGIN)
        .and_then(|r| r.strip_suffix(b"-----"))
        .ok_or_else(|| EnvelopeError::malformed("the BEGIN SPINE-INTENT line is not exact"))?;
    let space = inner
        .iter()
        .position(|&b| b == b' ')
        .ok_or_else(|| EnvelopeError::malformed("the fence header has one field"))?;
    let blob = crate::trailer::oid(
        inner[..space]
            .strip_prefix(b"blob=")
            .ok_or_else(|| EnvelopeError::malformed("the fence header has no blob="))?,
        "the fence's blob",
    )?;
    let bytes = crate::trailer::counter(
        inner[space + 1..]
            .strip_prefix(b"bytes=")
            .ok_or_else(|| EnvelopeError::malformed("the fence header has no bytes="))?,
        "the fence's bytes",
    )? as usize;

    let body = message
        .get(*pos..*pos + bytes)
        .ok_or_else(|| {
            EnvelopeError::new(
                Refusal::FenceMismatch,
                format!("the message holds fewer than bytes={bytes} after the BEGIN line"),
            )
        })?
        .to_vec();
    *pos += bytes;

    // "The END line follows the `n`-th byte **immediately**, with no inserted
    // separator." PB §3.3's canonical form makes byte `n` the intent's own
    // single trailing `0x0A`, so the END line begins a line.
    let end = take_line(message, pos)?;
    if end != FENCE_END {
        return Err(EnvelopeError::new(
            Refusal::FenceMismatch,
            "the END SPINE-INTENT line does not follow bytes= immediately",
        ));
    }

    // "`git hash-object` over exactly those bytes reproduces `blob=`" (PB §5.5).
    // The format is read from the id's own width, which EV §7 rule 7 fixes at
    // 40 or 64 — there is no other length to disambiguate.
    let format = if blob.len() == 40 {
        ObjectFormat::Sha1
    } else {
        ObjectFormat::Sha256
    };
    let computed = spine_canon::git_blob_id(&body, format);
    if computed != blob {
        return Err(EnvelopeError::new(
            Refusal::FenceMismatch,
            format!("fence says blob={blob}, the bytes hash to {computed}"),
        ));
    }
    Ok(Fence { blob, bytes, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal quick-lane landing, built here so the structural rules can be
    // attacked one at a time. Its digests are meaningless; its shape is not.
    fn quick() -> Vec<u8> {
        let mut m = Vec::new();
        m.extend_from_slice(b"quick: bump a dep\n\n");
        m.extend_from_slice(b"Spine-Envelope: 1\n");
        m.extend_from_slice(b"Spine-Event: land\n");
        m.extend_from_slice(b"Spine-Lane: quick\n");
        m.extend_from_slice(b"Spine-Gates: G1=pass\n");
        m.extend_from_slice(b"Spine-Strategy: squash\n");
        m.extend_from_slice(b"Spine-Seal: quick base=2c6a91d0f4b783e5a19c07d2f6b4830e59c1a72d head=e04b19a7c35d820f6e19b47a0c58d3f27e6a1b90 tree=9d17e0c4a52b836f10e7c94d2a6b58f03c1e7d46 report=sha256:0d84b71fe2a35c609d18a47b2e5c930f6b18d47a02c39e51f8a6b0d43c72e915 tool=1.4.0+sha256:41d0e9b7c2a6538f10bd47e29c05a3f6b81d24e70c9a5b3f68d1027ae4c95b3d git=2.45 mode=solo threat=hostile profile=none envelope=sha256:9764852ed4bd33a9eb42ca0674b88195f03eeac20df829b6e845175a449be44d signer=alice@example.com\n");
        m.extend_from_slice(b"Spine-Seal-Sig: AAAA\n");
        m
    }

    #[test]
    fn a_minimal_quick_landing_parses() {
        let e = Envelope::parse(&quick()).unwrap();
        assert_eq!(e.shape().unwrap(), Shape::Quick);
        assert!(e.fence().is_none());
        assert!(e.check_subject().is_ok());
    }

    #[test]
    fn a_second_seal_is_refused_before_any_digest_is_consulted() {
        let mut m = quick();
        let seal = b"Spine-Seal: quick base=x\n".to_vec();
        m.extend_from_slice(&seal);
        assert!(Envelope::parse(&m).is_err());
    }

    #[test]
    fn a_spine_line_below_the_seal_is_refused_and_a_foreign_one_is_not() {
        // EV §2.8 and §14 D7.
        let mut ok = quick();
        ok.extend_from_slice(b"Co-authored-by: someone <s@example.com>\n");
        let e = Envelope::parse(&ok).unwrap();
        assert_eq!(e.foreign_trailers().len(), 1);

        let mut bad = quick();
        bad.extend_from_slice(b"Spine-Gates: G1=pass\n");
        assert!(Envelope::parse(&bad).is_err());
    }

    #[test]
    fn a_foreign_line_inside_the_trailer_block_is_refused() {
        let m = String::from_utf8(quick())
            .unwrap()
            .replace("Spine-Gates:", "Co-authored-by: x\nSpine-Gates:");
        assert!(Envelope::parse(m.as_bytes()).is_err());
    }

    #[test]
    fn cr_anywhere_refuses() {
        let m = String::from_utf8(quick()).unwrap().replace('\n', "\r\n");
        assert!(Envelope::parse(m.as_bytes()).is_err());
    }

    #[test]
    fn a_sig_must_immediately_follow_what_it_signs() {
        let m = String::from_utf8(quick())
            .unwrap()
            .replace("Spine-Seal-Sig:", "Spine-Reverts: abc\nSpine-Seal-Sig:");
        assert!(Envelope::parse(m.as_bytes()).is_err());
    }

    #[test]
    fn an_unknown_envelope_version_refuses_before_anything_else() {
        let m = String::from_utf8(quick())
            .unwrap()
            .replace("Spine-Envelope: 1", "Spine-Envelope: 2");
        let e = Envelope::parse(m.as_bytes()).unwrap_err();
        assert_eq!(e.refusal(), Refusal::EnvelopeVersionUnknown);
    }

    #[test]
    fn a_quick_landing_must_not_carry_a_fenced_intent() {
        let mut m = b"quick: x\n\n-----BEGIN SPINE-INTENT blob=".to_vec();
        let body = b"# t\n";
        m.extend_from_slice(spine_canon::git_blob_id(body, ObjectFormat::Sha1).as_bytes());
        m.extend_from_slice(b" bytes=4-----\n");
        m.extend_from_slice(body);
        m.extend_from_slice(b"-----END SPINE-INTENT-----\n\n");
        m.extend_from_slice(&quick()[b"quick: bump a dep\n\n".len()..]);
        assert!(Envelope::parse(&m).is_err());
    }

    #[test]
    fn a_quick_subject_is_checked_only_for_its_prefix() {
        let good = String::from_utf8(quick()).unwrap();
        assert!(
            Envelope::parse(good.as_bytes())
                .unwrap()
                .check_subject()
                .is_ok()
        );

        let bad = good.replace("quick: bump a dep", "chore: update deps");
        let e = Envelope::parse(bad.as_bytes()).unwrap();
        assert_eq!(
            e.check_subject().unwrap_err().refusal(),
            Refusal::SubjectMismatch,
            "EV §14 D12(b): `chore: update deps` is a subject the quick-lane form forbids"
        );

        let empty = good.replace("quick: bump a dep", "quick: ");
        assert!(
            Envelope::parse(empty.as_bytes())
                .unwrap()
                .check_subject()
                .is_err()
        );
    }

    #[test]
    fn the_capped_quantity_counts_the_whole_message_when_no_manifest_is_present() {
        let m = quick();
        let e = Envelope::parse(&m).unwrap();
        assert_eq!(e.capped_quantity(), m.len());
        assert!(e.check_cap().is_ok());
    }
}
