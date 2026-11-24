//! A number as it is printed and as it is stored.

/// Without what only separates its groups: spaces, hyphens, dots, slashes, brackets. The `+` of a
/// phone number is part of it.
pub fn raw(printed: &str) -> String {
    printed
        .chars()
        .filter(|character| character.is_alphanumeric() || *character == '+')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_separates_the_groups_goes_and_nothing_else() {
        for (printed, stored) in [
            ("+33 1 23 45 67 89", "+33123456789"),
            ("(212) 555-0123", "2125550123"),
            (
                "FR14 2004 1010 0505 0001 3M02 606",
                "FR1420041010050500013M02606",
            ),
            ("CHE-123.456.789", "CHE123456789"),
            ("12/345/67890", "1234567890"),
            ("NWBKGB2L", "NWBKGB2L"),
            ("4 700\u{202f}000", "4700000"),
        ] {
            assert_eq!(raw(printed), stored);
        }
    }
}
