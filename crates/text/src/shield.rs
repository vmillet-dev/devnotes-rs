//! Every `{{…}}` kept out of a formatter's reach (the reference's `field-shield.ts`): this is not
//! what a field is — `notes::placeholder` decides that — but anything between double braces,
//! which a formatter would read as code. Offsets are `char`s.

const OPEN: &str = "{{";
const CLOSE: &str = "}}";

/// No digit and no `x`, which the stand-ins are made of besides the marker.
const MARKER_LETTERS: [char; 6] = ['q', 'z', 'j', 'k', 'v', 'w'];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Swap {
    original: String,
    stand_in: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shielded {
    pub text: String,
    pub cursor: usize,
    marker: String,
    swaps: Vec<Swap>,
}

impl Shielded {
    /// What each `{{…}}` became, in order.
    pub fn stand_ins(&self) -> impl Iterator<Item = &str> {
        self.swaps.iter().map(|swap| swap.stand_in.as_str())
    }
}

/// Every `{{…}}` becomes an identifier of its own length, which every language a formatter
/// reads accepts where a value goes. `None` when no marker is free, which only a text written
/// against this function could cause.
pub fn shield_fields(text: &str, cursor: usize) -> Option<Shielded> {
    let marker = free_marker(text)?;
    let mut swaps = Vec::new();
    let mut shielded = String::new();
    let mut shielded_cursor = cursor;
    let mut rest = 0;
    let len = |s: &str| s.chars().count();
    while let Some(start) = text[rest..].find(OPEN).map(|at| rest + at) {
        let Some(end) = text[start + OPEN.len()..]
            .find(CLOSE)
            .map(|at| start + OPEN.len() + at)
        else {
            break;
        };
        let original = &text[start..end + CLOSE.len()];
        let stand_in = stand_in_for(&marker, swaps.len(), len(original));
        shielded.push_str(&text[rest..start]);
        let start_char = len(&text[..start]);
        if cursor > start_char {
            shielded_cursor = if cursor >= start_char + len(original) {
                shielded_cursor + len(&stand_in) - len(original)
            } else {
                len(&shielded)
            };
        }
        shielded.push_str(&stand_in);
        swaps.push(Swap {
            original: original.to_owned(),
            stand_in,
        });
        rest = start + original.len();
    }
    shielded.push_str(&text[rest..]);
    Some(Shielded {
        text: shielded,
        cursor: shielded_cursor,
        marker,
        swaps,
    })
}

/// The fields back, in order. `None` when one is missing, moved past another or copied: the text
/// is then not the one that was typed, and nothing of it may be applied.
pub fn unshield_fields(
    formatted: &str,
    cursor: usize,
    shielded: &Shielded,
) -> Option<(String, usize)> {
    let mut text = String::new();
    let mut restored = cursor;
    let mut from = 0;
    let len = |s: &str| s.chars().count();
    for Swap { original, stand_in } in &shielded.swaps {
        let at = formatted[from..].find(stand_in.as_str())? + from;
        text.push_str(&formatted[from..at]);
        text.push_str(original);
        if cursor >= len(&formatted[..at]) + len(stand_in) {
            restored = restored + len(original) - len(stand_in);
        }
        from = at + stand_in.len();
    }
    text.push_str(&formatted[from..]);
    (!text.contains(&shielded.marker)).then_some((text, restored))
}

fn free_marker(text: &str) -> Option<String> {
    MARKER_LETTERS.iter().find_map(|first| {
        MARKER_LETTERS
            .iter()
            .filter(|second| *second != first)
            .map(|second| format!("{first}{second}"))
            .find(|marker| !text.contains(marker.as_str()))
    })
}

/// The marker on both sides, so no stand-in is found inside another.
fn stand_in_for(marker: &str, index: usize, length: usize) -> String {
    let core = format!("{marker}{index}");
    let padding = length.saturating_sub(core.len() + marker.len());
    format!("{core}{}{marker}", "x".repeat(padding))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_becomes_an_identifier_of_its_own_length() {
        let text = "host = {{db_host}}; port = {{ port = 5432 }}";
        let shielded = shield_fields(text, 0).unwrap();

        assert!(!shielded.text.contains("{{"));
        assert_eq!(shielded.text.len(), text.len());
        let originals: Vec<_> = shielded
            .swaps
            .iter()
            .map(|swap| swap.original.as_str())
            .collect();
        assert_eq!(originals, ["{{db_host}}", "{{ port = 5432 }}"]);
        assert!(
            shielded
                .stand_ins()
                .all(|stand_in| stand_in.chars().all(|c| c.is_ascii_alphanumeric()))
        );
    }

    #[test]
    fn the_marker_is_one_the_text_does_not_contain() {
        let shielded = shield_fields("qz zq {{a}}", 0).unwrap();
        assert_ne!(shielded.marker, "qz");
        assert!(!"qz zq".contains(&shielded.marker));
    }

    #[test]
    fn an_unterminated_brace_stays_and_a_short_token_is_lengthened() {
        assert_eq!(shield_fields("a {{ b", 0).unwrap().text, "a {{ b");
        assert!(shield_fields("{{}}", 0).unwrap().text.len() > 4);
    }

    #[test]
    fn the_cursor_moves_with_the_text_around_it() {
        let grown = shield_fields("{{}}", 0).unwrap().text.len();
        assert_eq!(shield_fields("{{}} ab", 6).unwrap().cursor, 6 + grown - 4);
        assert_eq!(shield_fields("ab {{field}} cd", 5).unwrap().cursor, 3);
        assert_eq!(shield_fields("ab {{field}}", 1).unwrap().cursor, 1);
    }

    #[test]
    fn no_free_marker_gives_up() {
        let every: Vec<String> = MARKER_LETTERS
            .iter()
            .flat_map(|a| MARKER_LETTERS.iter().map(move |b| format!("{a}{b}")))
            .collect();
        assert!(shield_fields(&every.join(" "), 0).is_none());
    }

    #[test]
    fn the_fields_come_back_with_the_cursor() {
        let shielded = shield_fields("a({{x}},{{y}})", 0).unwrap();
        let ins: Vec<_> = shielded.stand_ins().collect();
        let formatted = format!("a(\n  {},\n  {},\n);\n", ins[0], ins[1]);
        let expected = "a(\n  {{x}},\n  {{y}},\n);\n";

        let back = unshield_fields(&formatted, formatted.len(), &shielded);

        assert_eq!(back, Some((expected.to_owned(), expected.len())));
    }

    #[test]
    fn a_field_lost_copied_or_moved_is_refused() {
        let shielded = shield_fields("{{x}} {{y}}", 0).unwrap();
        let ins: Vec<_> = shielded.stand_ins().map(str::to_owned).collect();
        assert!(unshield_fields(&ins[0], 0, &shielded).is_none());
        assert!(unshield_fields(&format!("{} {}", ins[1], ins[0]), 0, &shielded).is_none());

        let one = shield_fields("{{x}}", 0).unwrap();
        let stand_in = one.stand_ins().next().unwrap().to_owned();
        assert!(unshield_fields(&format!("{stand_in} {stand_in}"), 0, &one).is_none());
    }
}
