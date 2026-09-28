use crate::closed_enum::closed_enum;

closed_enum! {
    /// Closed, like `Language`: the front receives a generated union, so an unknown
    /// value stops compiling there.
    pub enum NoteKind {
        /// Default, and what every note written before todo lists reads back as.
        #[default]
        Snippet = "snippet",
        Checklist = "checklist",
        /// Prose, written in the rich editor and stored as Markdown.
        Note = "note",
    }
}

impl NoteKind {
    /// Only a snippet's body is code: the others are read for no language, and pick none.
    pub fn has_language(self) -> bool {
        self == Self::Snippet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_serialized_form_matches_the_stored_one() {
        for kind in NoteKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();

            assert_eq!(json, format!("\"{}\"", kind.as_str()));
            assert_eq!(kind.as_str().parse::<NoteKind>().unwrap(), kind);
        }
    }

    #[test]
    fn an_unknown_stored_kind_is_refused_rather_than_guessed() {
        assert!("kanban".parse::<NoteKind>().is_err());
    }

    #[test]
    fn a_note_written_before_todo_lists_reads_back_as_a_snippet() {
        assert_eq!(NoteKind::default(), NoteKind::Snippet);
    }

    #[test]
    fn only_a_snippet_has_a_language() {
        let with_language: Vec<_> = NoteKind::ALL
            .into_iter()
            .filter(|kind| kind.has_language())
            .collect();

        assert_eq!(with_language, [NoteKind::Snippet]);
    }
}
