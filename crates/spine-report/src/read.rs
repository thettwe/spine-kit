//! Reading a report back — GR §3.2's closed schema, and nothing wider.
//!
//! "A reader that does not know a report's `report_version` **refuses**: status
//! `report-version-unknown`, exit 3. It never partially parses, never ignores
//! unknown members, and never guesses. […] A reader that meets an unknown
//! **member name** inside a version it does know refuses the same way. The
//! schema is closed: forward compatibility is bought with a version bump, not
//! with tolerance, **because a tolerant reader and a strict one compute
//! different digests over the same document and the whole artifact is a
//! digest**."
//!
//! That last clause is the whole reason this module tracks which members it
//! consumed and refuses on a leftover. A reader that skipped an unknown member
//! would round-trip the report to *different bytes*, and `--verify` would then
//! report `report-mismatch` against a sound landing.

use core::fmt;

use spine_canon::{ObjectFormat, Value, parse as parse_json, unesc};

use crate::gate::{Gate, GateResult};
use crate::ids::{Fingerprint, IntentId, Oid, Sha256Digest};
use crate::report::{
    Authority, Automerge, Collector, Evidence, Objects, Policy, REPORT_VERSION, Report, Rules, Run,
    Statement, Subject, Tool,
};
use crate::vocab::{
    AutoMerge, Event, GateStatus, Lane, Mode, Namespace, PreconditionStatus, Reverify, RuleMode,
    SealProfile, Strategy, WireClass, WireKind,
};
use crate::wire::{Wire, WireSet};

/// Why a report could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// GR §3.2's refusal, and GR §4.3's exit 3. One variant for both causes the
    /// spec gives it, because it fixes **one** status token for both: an
    /// unrecognized `report_version`, and an unknown member name inside a
    /// version the reader does know.
    ReportVersionUnknown(VersionUnknownCause),
    /// DERIVED. GR fixes no status token for a document that is canonical JSON,
    /// carries version 1 and exactly its member names, and still holds a value
    /// the schema cannot represent — a `gate` of `"G17"`, a `status` of
    /// `"warn"`, a string where an integer belongs. Refusing is the fail-closed
    /// reading: GR §3.2's reader "never partially parses, never ignores unknown
    /// members, and never guesses", and guessing is the only alternative.
    Malformed { at: String, why: &'static str },
    /// The bytes are not JSON at all, or violate GR §2.2's parse profile
    /// (duplicate member names, a number out of range, a depth bound).
    NotCanonicalJson(String),
}

/// Which of GR §3.2's two causes produced `report-version-unknown`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionUnknownCause {
    /// A `report_version` this binary has no parser for. "A binary keeps a
    /// parser *and a serializer* for every report version it has ever shipped."
    Version(u64),
    /// A member name this version's schema does not define.
    UnknownMember { at: String, name: String },
    /// `report_version` absent or not an integer — the member that decides
    /// which parser applies cannot itself be version-dependent.
    VersionMemberUnreadable,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // The exact token GR §4.3's table fixes.
            ReadError::ReportVersionUnknown(_) => f.write_str("report-version-unknown"),
            ReadError::Malformed { at, why } => write!(f, "malformed report at {at}: {why}"),
            ReadError::NotCanonicalJson(why) => write!(f, "not canonical JSON: {why}"),
        }
    }
}

impl core::error::Error for ReadError {}

impl Report {
    /// Parse a report from its canonical bytes.
    ///
    /// This is step 4 of GR §4.3's normative order, and it runs **after** the
    /// candidate's bytes have been hashed against the seal's `report=` (step
    /// 3), "because bytes that are not the sealed report are not worth
    /// parsing".
    pub fn from_canonical(bytes: &[u8]) -> Result<Report, ReadError> {
        let value = parse_json(bytes).map_err(|e| ReadError::NotCanonicalJson(e.to_string()))?;
        Report::from_value(&value)
    }

    /// Parse a report from an already-parsed value.
    pub fn from_value(value: &Value) -> Result<Report, ReadError> {
        let mut root = Obj::new("", value)?;

        // The version is read first and read strictly: it decides which parser
        // applies, so it cannot be parsed by a version-dependent rule.
        let version =
            root.required("report_version")?
                .as_u64()
                .ok_or(ReadError::ReportVersionUnknown(
                    VersionUnknownCause::VersionMemberUnreadable,
                ))?;
        if version != REPORT_VERSION {
            return Err(ReadError::ReportVersionUnknown(
                VersionUnknownCause::Version(version),
            ));
        }

        let object_format = {
            let s = string(root.required("object_format")?, "object_format")?;
            ObjectFormat::parse(s).ok_or(ReadError::Malformed {
                at: "object_format".into(),
                why: "not \"sha1\" or \"sha256\"",
            })?
        };

        let subject = read_subject(root.required("subject")?)?;
        let objects = read_objects(root.required("objects")?, object_format)?;
        let tool = read_tool(root.required("tool")?)?;
        let git_version = string(root.required("git_version")?, "git_version")?.to_owned();
        let mode = domain(Mode::parse, root.required("mode")?, "mode")?;
        let threat = domain(
            crate::vocab::Threat::parse,
            root.required("threat")?,
            "threat",
        )?;
        let profile = domain(SealProfile::parse, root.required("profile")?, "profile")?;
        let (policy, floor_source) = read_policy(root.required("policy")?, object_format)?;
        let authority = read_authority(root.required("authority")?)?;
        let self_approved = boolean(root.required("self_approved")?, "self_approved")?;
        let gates = read_gates(root.required("gates")?)?;
        let wires = read_wires(root.required("wires")?)?;
        let floor_hits = byte_list(root.required("floor_hits")?, "floor_hits")?;
        let (automerge, requested, effective) = read_automerge(root.required("automerge")?)?;
        let evidence = root.optional("evidence").map(read_evidence).transpose()?;
        let run = read_run(root.required("run")?)?;
        root.finish()?;

        let report = Report {
            subject,
            objects,
            tool,
            git_version,
            object_format,
            mode,
            profile,
            policy,
            authority,
            gates,
            wires,
            floor_hits,
            automerge,
            evidence,
            run,
        };

        // GR §9.10's "deliberate redundancies … and each is checkable". The
        // serializer computes these four from their definitions, so a document
        // whose stored copy disagrees is one no conforming writer produced, and
        // re-serializing it would change its digest.
        check(
            report.threat() == threat,
            "threat",
            "does not equal policy.rules.c_a3",
        )?;
        check(
            report.self_approved() == self_approved,
            "self_approved",
            "is not the disjunction over authority.reviews",
        )?;
        check(
            report.floor_source() == floor_source,
            "policy.floor_source",
            "is not \"spine:<tool.version>:floor\"",
        )?;
        check(
            Automerge::requested(report.policy.rules.c_m4) == requested,
            "automerge.requested",
            "does not equal policy.rules.c_m4 == \"on\"",
        )?;
        check(
            report.automerge.effective(report.policy.rules.c_m4) == effective,
            "automerge.effective",
            "is not requested and every precondition met or exempt",
        )?;

        Ok(report)
    }
}

fn read_subject(v: &Value) -> Result<Subject, ReadError> {
    let mut o = Obj::new("subject", v)?;
    let lane = domain(Lane::parse, o.required("lane")?, "subject.lane")?;
    let event = domain(Event::parse, o.required("event")?, "subject.event")?;
    let intent = match o.optional("intent") {
        None => None,
        Some(v) => Some(IntentId::parse(string(v, "subject.intent")?).map_err(|_| {
            ReadError::Malformed {
                at: "subject.intent".into(),
                why: "does not match ^(INT|BUG)-[0-9]+$",
            }
        })?),
    };
    let strategy = domain(Strategy::parse, o.required("strategy")?, "subject.strategy")?;
    o.finish()?;
    Ok(Subject {
        lane,
        event,
        intent,
        strategy,
    })
}

fn read_objects(v: &Value, fmt: ObjectFormat) -> Result<Objects, ReadError> {
    let mut o = Obj::new("objects", v)?;
    let base = oid(o.required("base")?, fmt, "objects.base")?;
    let head = oid(o.required("head")?, fmt, "objects.head")?;
    let ref_name = bytes(o.required("ref")?, "objects.ref")?;
    let merge_base = oid(o.required("merge_base")?, fmt, "objects.merge_base")?;
    let tree = oid(o.required("tree")?, fmt, "objects.tree")?;
    let intent_blob = o
        .optional("intent_blob")
        .map(|v| oid(v, fmt, "objects.intent_blob"))
        .transpose()?;
    o.finish()?;
    Ok(Objects {
        base,
        head,
        ref_name,
        merge_base,
        tree,
        intent_blob,
    })
}

fn read_tool(v: &Value) -> Result<Tool, ReadError> {
    let mut o = Obj::new("tool", v)?;
    // `tool.version` is `esc`-encoded (GR §5.3), so it is decoded like any
    // other byte-valued member — and must be UTF-8 to live in a `String`,
    // which every release version is.
    let raw = bytes(o.required("version")?, "tool.version")?;
    let version = String::from_utf8(raw).map_err(|_| ReadError::Malformed {
        at: "tool.version".into(),
        why: "is not UTF-8 after esc-decoding",
    })?;
    let dist_hash = digest(o.required("dist_hash")?, "tool.dist_hash")?;
    o.finish()?;
    Ok(Tool { version, dist_hash })
}

fn read_policy(v: &Value, fmt: ObjectFormat) -> Result<(Policy, String), ReadError> {
    let mut o = Obj::new("policy", v)?;
    let manifest = oid(o.required("manifest")?, fmt, "policy.manifest")?;
    let keyring = oid(o.required("keyring")?, fmt, "policy.keyring")?;
    let constitution = oid(o.required("constitution")?, fmt, "policy.constitution")?;
    let ci_sh = oid(o.required("ci_sh")?, fmt, "policy.ci_sh")?;
    let floor_source = string(o.required("floor_source")?, "policy.floor_source")?.to_owned();
    let floor_extensions = byte_list(o.required("floor_extensions")?, "policy.floor_extensions")?;
    let rules = read_rules(o.required("rules")?)?;
    o.finish()?;
    Ok((
        Policy {
            manifest,
            keyring,
            constitution,
            ci_sh,
            floor_extensions,
            rules,
        },
        floor_source,
    ))
}

fn read_rules(v: &Value) -> Result<Rules, ReadError> {
    let mut o = Obj::new("policy.rules", v)?;
    let c_a1 = domain(RuleMode::parse, o.required("c_a1")?, "policy.rules.c_a1")?;
    let c_a2 = byte_list(o.required("c_a2")?, "policy.rules.c_a2")?;
    let c_a3 = domain(
        crate::vocab::Threat::parse,
        o.required("c_a3")?,
        "policy.rules.c_a3",
    )?;
    let c_m1 = domain(Strategy::parse, o.required("c_m1")?, "policy.rules.c_m1")?;
    let c_m2 = domain(Reverify::parse, o.required("c_m2")?, "policy.rules.c_m2")?;
    let c_m3 = integer(o.required("c_m3")?, "policy.rules.c_m3")?;
    let c_m4 = domain(AutoMerge::parse, o.required("c_m4")?, "policy.rules.c_m4")?;
    let c_q1 = byte_list(o.required("c_q1")?, "policy.rules.c_q1")?;
    let c_q2 = integer(o.required("c_q2")?, "policy.rules.c_q2")?;
    let c_t1 = byte_list(o.required("c_t1")?, "policy.rules.c_t1")?;
    let c_t2 = byte_list(o.required("c_t2")?, "policy.rules.c_t2")?;
    let c_t3 = boolean(o.required("c_t3")?, "policy.rules.c_t3")?;
    o.finish()?;
    Ok(Rules {
        c_a1,
        c_a2,
        c_a3,
        c_m1,
        c_m2,
        c_m3,
        c_m4,
        c_q1,
        c_q2,
        c_t1,
        c_t2,
        c_t3,
    })
}

/// One `authority` statement.
///
/// `carries_self_approved` is not a convenience: GR §5.5 puts `self_approved`
/// on `reviews[]` and on nothing else, so on a `signoff`, an `approve`, a
/// `withdraw` or a `reopen` it is an **unknown member**, and GR §3.2 makes that
/// a refusal — "A reader that meets an unknown member name inside a version it
/// does know refuses the same way. The schema is closed."
///
/// Read as optional everywhere, it was accepted and discarded, and this
/// module's own header says what that costs: "A reader that skipped an unknown
/// member would round-trip the report to different bytes, and `--verify` would
/// then report `report-mismatch` against a sound landing."
fn read_statement(
    v: &Value,
    at: &str,
    carries_self_approved: bool,
) -> Result<(Statement, Option<bool>), ReadError> {
    let mut o = Obj::new(at, v)?;
    let line = bytes(o.required("line")?, at)?;
    let fingerprint =
        Fingerprint::parse(string(o.required("fingerprint")?, at)?).map_err(|_| {
            ReadError::Malformed {
                at: format!("{at}.fingerprint"),
                why: "is not \"SHA256:\" + 43 unpadded base64 characters",
            }
        })?;
    let namespace = domain(Namespace::parse, o.required("namespace")?, at)?;
    let self_approved = if carries_self_approved {
        o.optional("self_approved")
            .map(|v| boolean(v, at))
            .transpose()?
    } else {
        // Left unread, so `finish()` meets it as the unknown member it is.
        None
    };
    o.finish()?;
    Ok((
        Statement {
            line,
            fingerprint,
            namespace,
        },
        self_approved,
    ))
}

fn read_authority(v: &Value) -> Result<Authority, ReadError> {
    let mut o = Obj::new("authority", v)?;
    let opt = |o: &mut Obj<'_>, name: &'static str| -> Result<Option<Statement>, ReadError> {
        match o.optional(name) {
            None => Ok(None),
            Some(v) => Ok(Some(
                read_statement(v, &format!("authority.{name}"), false)?.0,
            )),
        }
    };
    let approve = opt(&mut o, "approve")?;
    let reopens = array(o.required("reopens")?, "authority.reopens")?
        .iter()
        .map(|s| read_statement(s, "authority.reopens[]", false).map(|(st, _)| st))
        .collect::<Result<Vec<_>, _>>()?;

    // The per-review `self_approved` is *derived* (GR §5.5), so it is read only
    // to be checked against the derivation — never to become the value.
    let mut reviews = Vec::new();
    let mut stored_self_approved = Vec::new();
    for s in array(o.required("reviews")?, "authority.reviews")? {
        let (st, sa) = read_statement(s, "authority.reviews[]", true)?;
        let sa = sa.ok_or(ReadError::Malformed {
            at: "authority.reviews[]".into(),
            why: "a review carries self_approved",
        })?;
        reviews.push(st);
        stored_self_approved.push(sa);
    }

    let signoff = opt(&mut o, "signoff")?;
    let upgrade = opt(&mut o, "upgrade")?;
    let withdraw = opt(&mut o, "withdraw")?;
    o.finish()?;

    let authority = Authority {
        signoff,
        approve,
        reopens,
        reviews,
        upgrade,
        withdraw,
    };
    for (review, stored) in authority.reviews.iter().zip(stored_self_approved) {
        check(
            authority.review_is_self_approved(review) == stored,
            "authority.reviews[].self_approved",
            "does not equal fingerprint == the landing's signer key",
        )?;
    }
    Ok(authority)
}

fn read_gates(v: &Value) -> Result<Vec<GateResult>, ReadError> {
    let gates: Vec<GateResult> = array(v, "gates")?
        .iter()
        .map(|e| {
            let mut o = Obj::new("gates[]", e)?;
            let gate = domain(Gate::parse, o.required("gate")?, "gates[].gate")?;
            let status = domain(GateStatus::parse, o.required("status")?, "gates[].status")?;
            o.finish()?;
            Ok(GateResult { gate, status })
        })
        .collect::<Result<_, ReadError>>()?;

    // GR §5.6: "sorts by gate number ascending." The reader refuses rather
    // than sorting, for the reason the module header gives: the serializer
    // *would* sort, so a reader that accepted an unsorted array would round-
    // trip to different bytes and `--verify` would report `report-mismatch`
    // against the very document it was handed.
    if gates
        .windows(2)
        .any(|w| w[0].gate.number() >= w[1].gate.number())
    {
        return Err(ReadError::Malformed {
            at: "gates".into(),
            why: "is not sorted ascending by gate number, or repeats a gate",
        });
    }
    Ok(gates)
}

fn read_wires(v: &Value) -> Result<WireSet, ReadError> {
    let raised = array(v, "wires")?
        .iter()
        .map(|e| {
            let mut o = Obj::new("wires[]", e)?;
            let gate = domain(Gate::parse, o.required("gate")?, "wires[].gate")?;
            let path = o
                .optional("path")
                .map(|v| bytes(v, "wires[].path"))
                .transpose()?;
            let class = domain(WireClass::parse, o.required("class")?, "wires[].class")?;
            let kind = domain(WireKind::parse, o.required("kind")?, "wires[].kind")?;
            o.finish()?;
            Ok(Wire {
                gate,
                path,
                class,
                kind,
            })
        })
        .collect::<Result<Vec<_>, ReadError>>()?;
    // Re-running the collapse over an already-collapsed array is the identity,
    // and it re-establishes GR §6.1's order — so a candidate whose array was
    // written under the numeric order is silently corrected here and then fails
    // the digest comparison in GR §4.3 step 6, which is where a wrong wire
    // comparator is supposed to be caught (GR §8.2.1).
    WireSet::from_raised(raised).map_err(|e| ReadError::Malformed {
        at: "wires".into(),
        why: match e {
            crate::wire::WireSetError::CrossKindCollapse { .. } => {
                "carries one key under two kinds"
            }
        },
    })
}

fn read_automerge(v: &Value) -> Result<(Automerge, bool, bool), ReadError> {
    let mut o = Obj::new("automerge", v)?;
    let requested = boolean(o.required("requested")?, "automerge.requested")?;
    let entries = array(o.required("preconditions")?, "automerge.preconditions")?;
    if entries.len() != 5 {
        return Err(ReadError::Malformed {
            at: "automerge.preconditions".into(),
            why: "is five entries, id ascending",
        });
    }
    let mut preconditions = [PreconditionStatus::Unmet; 5];
    for (want_id, e) in entries.iter().enumerate() {
        let mut po = Obj::new("automerge.preconditions[]", e)?;
        let id = integer(po.required("id")?, "automerge.preconditions[].id")?;
        let status = domain(
            PreconditionStatus::parse,
            po.required("status")?,
            "automerge.preconditions[].status",
        )?;
        po.finish()?;
        if id != want_id as u64 {
            return Err(ReadError::Malformed {
                at: "automerge.preconditions".into(),
                why: "ids are 0..4 ascending",
            });
        }
        preconditions[want_id] = status;
    }
    let effective = boolean(o.required("effective")?, "automerge.effective")?;
    o.finish()?;
    Ok((Automerge { preconditions }, requested, effective))
}

fn read_evidence(v: &Value) -> Result<Evidence, ReadError> {
    let mut o = Obj::new("evidence", v)?;
    let result_sha256 = digest(o.required("result_sha256")?, "evidence.result_sha256")?;
    let collector = {
        let mut c = Obj::new("evidence.collector", o.required("collector")?)?;
        let raw = bytes(c.required("version")?, "evidence.collector.version")?;
        let version = String::from_utf8(raw).map_err(|_| ReadError::Malformed {
            at: "evidence.collector.version".into(),
            why: "is not UTF-8 after esc-decoding",
        })?;
        let dist_hash = digest(c.required("dist_hash")?, "evidence.collector.dist_hash")?;
        c.finish()?;
        Collector { version, dist_hash }
    };
    let keys_visible = boolean(o.required("keys_visible")?, "evidence.keys_visible")?;
    let ids = integer(o.required("ids")?, "evidence.ids")?;
    o.finish()?;
    Ok(Evidence {
        result_sha256,
        collector,
        keys_visible,
        ids,
    })
}

fn read_run(v: &Value) -> Result<Run, ReadError> {
    let mut o = Obj::new("run", v)?;
    let reverifications = integer(o.required("reverifications")?, "run.reverifications")?;
    o.finish()?;
    Ok(Run { reverifications })
}

// ---------------------------------------------------------------------------
// The strict object reader. Every member is claimed by name; anything left over
// when `finish` runs is GR §3.2's unknown member.
// ---------------------------------------------------------------------------

struct Obj<'a> {
    at: String,
    members: &'a [(String, Value)],
    used: Vec<&'a str>,
}

impl<'a> Obj<'a> {
    fn new(at: &str, v: &'a Value) -> Result<Self, ReadError> {
        match v {
            Value::Obj(members) => Ok(Obj {
                at: at.to_owned(),
                members,
                used: Vec::new(),
            }),
            other => Err(ReadError::Malformed {
                at: if at.is_empty() {
                    "<root>".into()
                } else {
                    at.to_owned()
                },
                why: match other {
                    Value::Arr(_) => "is an array, not an object",
                    _ => "is not an object",
                },
            }),
        }
    }

    fn get(&mut self, name: &'static str) -> Option<&'a Value> {
        let members = self.members;
        let found = members.iter().find(|(k, _)| k == name);
        if found.is_some() {
            self.used.push(name);
        }
        found.map(|(_, v)| v)
    }

    fn required(&mut self, name: &'static str) -> Result<&'a Value, ReadError> {
        let at = self.qualify(name);
        self.get(name).ok_or(ReadError::Malformed {
            at,
            why: "is required and absent",
        })
    }

    fn optional(&mut self, name: &'static str) -> Option<&'a Value> {
        self.get(name)
    }

    fn qualify(&self, name: &str) -> String {
        if self.at.is_empty() {
            name.to_owned()
        } else {
            format!("{}.{}", self.at, name)
        }
    }

    /// GR §3.2: "A reader that meets an unknown **member name** inside a
    /// version it does know refuses the same way."
    fn finish(self) -> Result<(), ReadError> {
        for (name, _) in self.members {
            if !self.used.iter().any(|u| u == name) {
                return Err(ReadError::ReportVersionUnknown(
                    VersionUnknownCause::UnknownMember {
                        at: if self.at.is_empty() {
                            "<root>".into()
                        } else {
                            self.at.clone()
                        },
                        name: name.clone(),
                    },
                ));
            }
        }
        Ok(())
    }
}

fn string<'a>(v: &'a Value, at: &str) -> Result<&'a str, ReadError> {
    v.as_str().ok_or_else(|| ReadError::Malformed {
        at: at.to_owned(),
        why: "is not a string",
    })
}

fn integer(v: &Value, at: &str) -> Result<u64, ReadError> {
    v.as_u64().ok_or_else(|| ReadError::Malformed {
        at: at.to_owned(),
        why: "is not an integer",
    })
}

fn boolean(v: &Value, at: &str) -> Result<bool, ReadError> {
    v.as_bool().ok_or_else(|| ReadError::Malformed {
        at: at.to_owned(),
        why: "is not a boolean",
    })
}

fn array<'a>(v: &'a Value, at: &str) -> Result<&'a [Value], ReadError> {
    v.as_arr().ok_or_else(|| ReadError::Malformed {
        at: at.to_owned(),
        why: "is not an array",
    })
}

fn domain<T>(parse: fn(&str) -> Option<T>, v: &Value, at: &str) -> Result<T, ReadError> {
    parse(string(v, at)?).ok_or_else(|| ReadError::Malformed {
        at: at.to_owned(),
        why: "is outside its closed domain",
    })
}

fn oid(v: &Value, fmt: ObjectFormat, at: &str) -> Result<Oid, ReadError> {
    Oid::parse(string(v, at)?, fmt).map_err(|_| ReadError::Malformed {
        at: at.to_owned(),
        why: "is not lowercase hex at the width object_format implies",
    })
}

fn digest(v: &Value, at: &str) -> Result<Sha256Digest, ReadError> {
    Sha256Digest::parse(string(v, at)?).map_err(|_| ReadError::Malformed {
        at: at.to_owned(),
        why: "is not \"sha256:\" + 64 lowercase hex",
    })
}

/// GR §2.3's decode: "`\\` introduces either `\\` (one literal backslash) or
/// `x` plus exactly two lowercase hex digits (one byte). Any other sequence
/// after `\\` is an invalid report."
fn bytes(v: &Value, at: &str) -> Result<Vec<u8>, ReadError> {
    unesc(string(v, at)?).map_err(|_| ReadError::Malformed {
        at: at.to_owned(),
        why: "is not a valid esc encoding",
    })
}

fn byte_list(v: &Value, at: &str) -> Result<Vec<Vec<u8>>, ReadError> {
    array(v, at)?.iter().map(|e| bytes(e, at)).collect()
}

fn check(ok: bool, at: &'static str, why: &'static str) -> Result<(), ReadError> {
    if ok {
        Ok(())
    } else {
        Err(ReadError::Malformed {
            at: at.to_owned(),
            why,
        })
    }
}
