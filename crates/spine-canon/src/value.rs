//! The JSON value model the corpus's canonicalization is defined over.
//!
//! `gate-report.md` §2.2 fixes a *value profile* on top of RFC 8785: integers
//! only, no null, no duplicate member names. The model here is deliberately
//! wider than that profile in one direction only — it can hold any member name
//! and any Unicode string, because §8.3's canonicalizer vector pins ordering
//! with the names `Z` and `_c`, which the profile itself forbids. Profile
//! conformance is a separate check (`profile`), never a property of the type.

use core::fmt;

/// A JSON value restricted to what a spine artifact can contain.
///
/// There is no float variant, and that is not an omission: GR §2.2 says "there
/// is no floating-point value anywhere in a gate report", and RFC 8785's only
/// genuinely hard corner is float serialization. A type that cannot hold one
/// cannot serialize one wrongly.
#[derive(Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    /// GR §2.2: `0 <= n <= 2^53 - 1`. Range is checked by [`profile`], not by
    /// the constructor, so a parser can report the offending position.
    Int(u64),
    Str(String),
    Arr(Vec<Value>),
    /// Members in *insertion* order. Canonical output sorts them (RFC 8785
    /// §3.2.3); holding source order lets a parser report a duplicate name at
    /// the position it appeared.
    Obj(Vec<(String, Value)>),
    /// Parseable so that a malformed document is reported as a profile
    /// violation rather than a syntax error, but never emitted by spine.
    /// GR §2.2: "Null | Never emitted. An absent value is an absent member."
    Null,
}

impl Value {
    pub fn obj(members: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Self {
        Value::Obj(members.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    pub fn str(s: impl Into<String>) -> Self {
        Value::Str(s.into())
    }

    pub fn arr(items: impl IntoIterator<Item = Value>) -> Self {
        Value::Arr(items.into_iter().collect())
    }

    /// Look a member up by name. Returns the *first* occurrence; a document
    /// with two is rejected at parse time, so there is never a second.
    pub fn get(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Obj(members) => members.iter().find(|(k, _)| k == name).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Value]> {
        match self {
            Value::Arr(items) => Some(items),
            _ => None,
        }
    }

    /// The name this kind goes by in an error message. Fixed strings, because
    /// they reach users through refusals.
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Bool(_) => "boolean",
            Value::Int(_) => "integer",
            Value::Str(_) => "string",
            Value::Arr(_) => "array",
            Value::Obj(_) => "object",
            Value::Null => "null",
        }
    }
}

impl fmt::Debug for Value {
    /// Debug output is the canonical form. A `Value` has exactly one correct
    /// spelling and printing a second one in test failures would be a way to
    /// look at the wrong bytes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crate::jcs::canonicalize_to_string(self))
    }
}
