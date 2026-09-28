use crate::closed_enum::closed_enum;

closed_enum! {
    /// Not sealed: SQL filters and sorts on it. Declared from the least to the most pressing,
    /// the order of the keys 0 to 4.
    pub enum Priority {
        #[default]
        None = "none",
        Low = "low",
        Medium = "medium",
        High = "high",
        Urgent = "urgent",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_serialized_form_matches_the_stored_one() {
        for priority in Priority::ALL {
            let json = serde_json::to_string(&priority).unwrap();

            assert_eq!(json, format!("\"{}\"", priority.as_str()));
            assert_eq!(priority.as_str().parse::<Priority>().unwrap(), priority);
        }
    }

    #[test]
    fn a_note_without_one_has_none() {
        assert_eq!(Priority::default(), Priority::None);
    }
}
