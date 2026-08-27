//! IR §6.3 step 2's **declarative subset** of YAML, and nothing else.
//!
//! "The YAML is read as the **declarative subset**: block mappings, block
//! sequences and plain or single/double-quoted scalars only. Anchors (`&`),
//! aliases (`*`), merge keys (`<<`), tags (`!`), multi-document streams (`---`
//! more than once) and flow mappings that nest more than one level →
//! unclassifiable, reason `pubspec-not-declarative`."
//!
//! Refusing outside the subset is the same move IR §5.3 makes for
//! `tsconfig.json` and §7.3 for `Package.swift`: a full YAML implementation is
//! the single most divergent parser in common use, and a `pubspec.yaml` is
//! candidate-controlled.

/// What the subset can hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Yaml {
    Scalar(String),
    /// Members in file order.
    Map(Vec<(String, Yaml)>),
    Seq(Vec<Yaml>),
    /// A key with no value and no block beneath it — `dependencies:` followed
    /// by nothing. Distinct from an empty scalar so a caller can tell "declared
    /// and empty" from "declared as the empty string".
    Empty,
}

impl Yaml {
    pub fn get(&self, key: &str) -> Option<&Yaml> {
        match self {
            Yaml::Map(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            Yaml::Scalar(s) => Some(s),
            _ => None,
        }
    }
}

/// Parse the declarative subset. `None` is `pubspec-not-declarative`.
pub fn parse(source: &str) -> Option<Yaml> {
    let mut lines: Vec<Line> = Vec::new();
    let mut documents = 0usize;
    for raw in source.split('\n') {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let text = strip_comment(raw);
        if text.trim().is_empty() {
            continue;
        }
        if text.trim_end() == "---" {
            documents += 1;
            // "multi-document streams (`---` more than once)". One leading
            // document marker is an ordinary single-document file.
            if documents > 1 {
                return None;
            }
            continue;
        }
        let indent = text.len() - text.trim_start().len();
        lines.push(Line {
            indent,
            text: text.trim().to_string(),
        });
    }
    let mut i = 0usize;
    let value = block(&lines, &mut i, 0)?;
    if i != lines.len() { None } else { Some(value) }
}

struct Line {
    indent: usize,
    text: String,
}

/// A `#` opens a comment only at the start of a line or after whitespace, which
/// is YAML's own rule and is what keeps a `#` inside a plain scalar (a URL
/// fragment, a colour) from truncating it.
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'#' && (i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t') {
            return &line[..i];
        }
    }
    line
}

fn block(lines: &[Line], i: &mut usize, indent: usize) -> Option<Yaml> {
    let first = lines.get(*i)?;
    if first.indent < indent {
        return Some(Yaml::Empty);
    }
    let indent = first.indent;
    if first.text.starts_with("- ") || first.text == "-" {
        let mut items = Vec::new();
        while let Some(line) = lines.get(*i) {
            if line.indent != indent || !(line.text.starts_with("- ") || line.text == "-") {
                break;
            }
            let rest = line.text[1..].trim().to_string();
            *i += 1;
            if rest.is_empty() {
                items.push(block(lines, i, indent + 1)?);
            } else {
                items.push(scalar_or_flow(&rest)?);
            }
        }
        return Some(Yaml::Seq(items));
    }

    let mut members: Vec<(String, Yaml)> = Vec::new();
    while let Some(line) = lines.get(*i) {
        if line.indent != indent {
            break;
        }
        let (key, rest) = split_key(&line.text)?;
        // "merge keys (`<<`)"
        if key == "<<" {
            return None;
        }
        *i += 1;
        let value = if rest.is_empty() {
            match lines.get(*i) {
                Some(next) if next.indent > indent => block(lines, i, next.indent)?,
                _ => Yaml::Empty,
            }
        } else {
            scalar_or_flow(rest)?
        };
        members.push((key, value));
    }
    if members.is_empty() {
        return None;
    }
    Some(Yaml::Map(members))
}

/// Split `key: rest` at the first `: ` or a trailing `:`. A key is a plain or
/// quoted scalar; anything else is outside the subset.
fn split_key(text: &str) -> Option<(String, &str)> {
    let bytes = text.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] != b':' {
            continue;
        }
        let is_end = i + 1 == bytes.len();
        let followed_by_space = bytes.get(i + 1) == Some(&b' ');
        if is_end || followed_by_space {
            let key = unquote(text[..i].trim())?;
            let rest = if is_end { "" } else { text[i + 1..].trim() };
            return Some((key, rest));
        }
    }
    None
}

/// A scalar, or a one-level flow mapping/sequence.
///
/// "flow mappings that nest more than one level" leave the subset, so a `{` or
/// `[` inside a flow collection is refused rather than parsed.
fn scalar_or_flow(text: &str) -> Option<Yaml> {
    if let Some(inner) = text.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
        if inner.contains('{') || inner.contains('[') {
            return None;
        }
        let mut members = Vec::new();
        for part in inner.split(',') {
            if part.trim().is_empty() {
                continue;
            }
            let (key, rest) = split_key(part.trim())?;
            members.push((key, plain(rest)?));
        }
        return Some(Yaml::Map(members));
    }
    if let Some(inner) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        if inner.contains('{') || inner.contains('[') {
            return None;
        }
        let mut items = Vec::new();
        for part in inner.split(',') {
            if part.trim().is_empty() {
                continue;
            }
            items.push(plain(part.trim())?);
        }
        return Some(Yaml::Seq(items));
    }
    plain(text)
}

/// A plain or quoted scalar, with the four constructs the subset excludes
/// refused by their opening byte.
fn plain(text: &str) -> Option<Yaml> {
    let text = text.trim();
    match text.as_bytes().first() {
        // "Anchors (`&`), aliases (`*`) … tags (`!`)". Case D13.
        Some(b'&') | Some(b'*') | Some(b'!') => None,
        // Block scalars (`|`, `>`) are not in the subset's list of admitted
        // forms — "plain or single/double-quoted scalars **only**".
        Some(b'|') | Some(b'>') => None,
        _ => Some(Yaml::Scalar(unquote(text)?)),
    }
}

fn unquote(text: &str) -> Option<String> {
    let text = text.trim();
    for quote in ['\'', '"'] {
        if text.len() >= 2 && text.starts_with(quote) && text.ends_with(quote) {
            let inner = &text[1..text.len() - 1];
            if inner.contains(quote) {
                // An escaped or doubled quote is outside the subset; refusing
                // is cheaper than deciding which of YAML's two escape dialects
                // applies.
                return None;
            }
            return Some(inner.to_string());
        }
    }
    if text.starts_with('\'') || text.starts_with('"') {
        return None;
    }
    Some(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pubspec_shaped_block_mapping_parses() {
        let source = "name: billing\nversion: 1.0.0\ndependencies:\n  meta: ^1.0.0\n  other:\n    path: ../other\n";
        let yaml = parse(source).expect("declarative");
        assert_eq!(yaml.get("name").unwrap().as_scalar(), Some("billing"));
        let deps = yaml.get("dependencies").unwrap();
        assert_eq!(deps.get("meta").unwrap().as_scalar(), Some("^1.0.0"));
        assert_eq!(
            deps.get("other").unwrap().get("path").unwrap().as_scalar(),
            Some("../other")
        );
    }

    /// IR §6.3 step 3's own spelling, `<pkg>: { path: <p> }`, as a one-level
    /// flow mapping.
    #[test]
    fn a_one_level_flow_mapping_parses_and_a_nested_one_does_not() {
        let yaml = parse("dependencies:\n  other: {path: ../other}\n").unwrap();
        assert_eq!(
            yaml.get("dependencies")
                .unwrap()
                .get("other")
                .unwrap()
                .get("path")
                .unwrap()
                .as_scalar(),
            Some("../other")
        );
        assert_eq!(parse("a: {b: {c: d}}\n"), None);
    }

    /// Case D13, and the rest of §6.3 step 2's excluded list.
    #[test]
    fn every_construct_outside_the_declarative_subset_is_refused() {
        for source in [
            "a: &anchor v\nb: *anchor\n",       // anchor and alias
            "a: !tag v\n",                       // tag
            "base: &b\n  x: 1\nc:\n  <<: *b\n",  // merge key
            "---\na: 1\n---\nb: 2\n",            // multi-document
            "a: |\n  block\n",                   // block scalar
        ] {
            assert_eq!(parse(source), None, "{source:?}");
        }
    }

    /// A `#` opens a comment only at a line start or after whitespace.
    #[test]
    fn a_hash_inside_a_plain_scalar_does_not_truncate_it() {
        let yaml = parse("# leading comment\nname: a#b  # trailing\n").unwrap();
        assert_eq!(yaml.get("name").unwrap().as_scalar(), Some("a#b"));
    }

    /// One leading `---` is an ordinary single-document file.
    #[test]
    fn a_single_document_marker_is_not_a_stream() {
        let yaml = parse("---\nname: a\n").unwrap();
        assert_eq!(yaml.get("name").unwrap().as_scalar(), Some("a"));
    }

    #[test]
    fn a_key_declared_with_nothing_under_it_is_empty_and_not_a_scalar() {
        let yaml = parse("dev_dependencies:\nname: a\n").unwrap();
        assert_eq!(yaml.get("dev_dependencies"), Some(&Yaml::Empty));
    }
}
