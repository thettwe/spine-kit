//! Acceptance criteria: ID §5.3's line grammar, the `AC-<n>` id domain, and
//! the ordering the id domain implies.
//!
//! **The AC id is a join key, not a label.** ID §5.3: "the id is the join key
//! to everything downstream — `@verifies INT-042/AC-1` pragmas, `test_AC1_*`
//! names, the `<repo>/INT-042/AC-1` node id (`dump.md` §5.2), G1's coverage
//! clause and G5's orphan clause — and a document with `AC-1, AC-2, AC-7`
//! either has a seventh AC that is not there or a numbering scheme nothing else
//! in the system shares."
//!
//! **The text is never read.** PB §6.2 gives the `ac` kind no attrs. "The
//! mechanical content of this section is the set of AC ids and their count."

use crate::status::Status;

/// ID §5.3's maximum, which is PB §3.1's cap: "maximum 6 — more means split the
/// task".
pub const MAX_ACS: usize = 6;

/// ID §5.3's minimum, resolved by ID §11.4 rather than by the playbook: "A
/// zero-AC intent makes `--approve`'s 'every AC covered by a collected id'
/// guard vacuous, makes G1's coverage clause vacuous, and asks a human to sign
/// a document that promises nothing testable."
pub const MIN_ACS: usize = 1;

/// One acceptance criterion's mechanical content: its number, and the 1-based
/// line it was declared on.
///
/// The text is deliberately absent. ID §5.6 lists it among the members that are
/// "deliberately not members … An implementation may keep them for
/// `spine review`'s packet; two implementations that differ in whether they do
/// still agree on every gate verdict."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ac {
    pub number: u8,
    pub line: usize,
}

impl Ac {
    /// The node id suffix `dump.md` §5.2 gives an `ac` node: `<ID>/AC-<n>`
    /// under the repo, so this is the `AC-<n>` half.
    pub fn label(&self) -> String {
        format!("AC-{}", self.number)
    }
}

/// ID §5.3's grammar:
///
/// ```text
/// ac-line := "AC-" number ": " text
/// number  := a decimal integer 1 … 6, no leading zeros
/// text    := non-empty, no U+000A
/// ```
///
/// A line whose first three bytes are `AC-` and which does not match is
/// `malformed-ac`. "That clause is load-bearing: it is what stops
/// `AC-3 the total is right` (no colon) from being silently reclassified as
/// prose and dropped."
pub fn parse_ac_line(line: &str, line_no: usize) -> Result<Ac, Status> {
    let rest = line.strip_prefix("AC-").ok_or(Status::MalformedAc)?;
    let (digits, text) = rest.split_once(": ").ok_or(Status::MalformedAc)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Status::MalformedAc);
    }
    // "no leading zeros" — so `AC-01` is not a second spelling of `AC-1`.
    if digits.len() > 1 && digits.starts_with('0') {
        return Err(Status::MalformedAc);
    }
    let number: u8 = digits.parse().map_err(|_| Status::MalformedAc)?;
    if !(1..=MAX_ACS as u8).contains(&number) {
        return Err(Status::MalformedAc);
    }
    if text.is_empty() {
        return Err(Status::MalformedAc);
    }
    Ok(Ac {
        number,
        line: line_no,
    })
}

/// ID §5.3's three shape bounds, in that table's order: minimum, maximum,
/// numbering.
///
/// "**Numbering is contiguous from 1 and in order.** Deleting AC-3 means
/// renumbering." Which also "makes the maximum mechanical: with contiguous
/// numbering, `AC-7` cannot exist."
pub fn check_bounds(acs: &[Ac]) -> Result<(), Status> {
    if acs.len() < MIN_ACS {
        return Err(Status::NoAcceptanceCriteria);
    }
    if acs.len() > MAX_ACS {
        return Err(Status::TooManyAcs);
    }
    for (i, ac) in acs.iter().enumerate() {
        if usize::from(ac.number) != i + 1 {
            return Err(Status::AcNumbering);
        }
    }
    Ok(())
}

/// The AC labels of a document, in the order a **byte** sort puts them.
///
/// This is the ordering `AC-10` would precede `AC-2` under — the wire
/// comparator's, `gate-report.md` §6.2's "ascending by unsigned byte value over
/// the whole token". At template version 2 the domain is bounded at 6 by
/// [`MAX_ACS`], so byte order and numeric order coincide here and the function
/// exists to be *stable* if the cap ever moves: a caller that needs the wire
/// order gets it from this rather than from a numeric sort that happens to
/// agree today.
pub fn labels_in_byte_order(acs: &[Ac]) -> Vec<String> {
    let mut labels: Vec<String> = acs.iter().map(Ac::label).collect();
    labels.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_9_1s_three_ac_lines_parse_to_their_numbers() {
        for (line, n) in [
            ("AC-1: Given an invoice with taxable lines, when it is rendered, then the total", 1u8),
            ("AC-2: Given an invoice whose lines are all zero-rated, when it is rendered, then", 2),
            ("AC-3: Given an invoice issued before this ships, when it is re-rendered, then its", 3),
        ] {
            assert_eq!(parse_ac_line(line, 1).unwrap().number, n);
        }
    }

    /// The clause ID §5.3 calls load-bearing.
    #[test]
    fn an_ac_line_with_no_colon_is_malformed_not_reclassified_as_prose() {
        assert_eq!(
            parse_ac_line("AC-3 the total is right", 1),
            Err(Status::MalformedAc)
        );
    }

    #[test]
    fn the_number_domain_is_one_to_six_with_no_second_spelling() {
        for n in 1..=6u8 {
            let line = format!("AC-{n}: x");
            assert_eq!(parse_ac_line(&line, 1).unwrap().number, n);
        }
        for digits in ["0", "7", "01", "10", "", "+1", "1x"] {
            let line = format!("AC-{digits}: x");
            assert_eq!(parse_ac_line(&line, 1), Err(Status::MalformedAc), "{digits}");
        }
    }

    #[test]
    fn an_ac_line_with_empty_text_is_malformed() {
        // `AC-1: ` cannot occur — §2.1 rule 9 forbids the trailing space — so
        // the reachable empty-text spelling is a line that ends at the colon.
        assert_eq!(parse_ac_line("AC-1:", 1), Err(Status::MalformedAc));
    }

    #[test]
    fn a_line_not_beginning_ac_is_not_an_ac_line() {
        assert_eq!(parse_ac_line("- a non-goal", 1), Err(Status::MalformedAc));
    }

    fn acs(numbers: &[u8]) -> Vec<Ac> {
        numbers
            .iter()
            .enumerate()
            .map(|(i, &number)| Ac {
                number,
                line: i + 1,
            })
            .collect()
    }

    #[test]
    fn the_three_bounds_fire_in_id_5_3s_table_order() {
        assert_eq!(check_bounds(&[]), Err(Status::NoAcceptanceCriteria));
        // Seven items with a duplicate number breaks both the maximum and the
        // numbering; the maximum is checked first.
        assert_eq!(
            check_bounds(&acs(&[1, 2, 3, 4, 5, 6, 1])),
            Err(Status::TooManyAcs)
        );
        assert_eq!(check_bounds(&acs(&[1, 2, 4])), Err(Status::AcNumbering));
        assert_eq!(check_bounds(&acs(&[2, 1])), Err(Status::AcNumbering));
        assert_eq!(check_bounds(&acs(&[1, 1])), Err(Status::AcNumbering));
        assert!(check_bounds(&acs(&[1, 2, 3])).is_ok());
        assert!(check_bounds(&acs(&[1])).is_ok());
    }

    /// ID §5.3: with contiguous numbering "`AC-7` cannot exist" — and it cannot
    /// even be written, because 7 is outside the line grammar's number domain.
    #[test]
    fn ac_seven_is_unwritable_as_well_as_uncountable() {
        assert_eq!(parse_ac_line("AC-7: x", 1), Err(Status::MalformedAc));
    }

    /// R1's comparator, pinned on the shape it would bite: byte order over the
    /// whole token, so `AC-10` precedes `AC-2`. Version 2's cap of six keeps
    /// the two orders in agreement, and this test says so rather than leaving a
    /// reader to assume a numeric sort.
    #[test]
    fn byte_order_and_numeric_order_agree_only_because_the_cap_is_six() {
        assert_eq!(
            labels_in_byte_order(&acs(&[1, 2, 3, 4, 5, 6])),
            ["AC-1", "AC-2", "AC-3", "AC-4", "AC-5", "AC-6"]
        );
        // The comparator itself, shown on tokens the cap forbids.
        let mut wider = ["AC-2", "AC-10"];
        wider.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        assert_eq!(wider, ["AC-10", "AC-2"]);
    }

    #[test]
    fn an_acs_label_is_the_node_ids_own_suffix() {
        assert_eq!(Ac { number: 1, line: 9 }.label(), "AC-1");
    }
}
