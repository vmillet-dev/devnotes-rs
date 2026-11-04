//! The words of a Note as it reads, not as its Markdown is spelled (the reference's
//! `word-count.ts`).

use std::sync::LazyLock;

use regex::Regex;

/// Syntax a reader never sees: a link's address, a task's box, the entity a leading tab is
/// written as.
static UNREAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\]\([^)]*\)|\[[ xX]\]|&#9;").expect("a valid pattern"));
static WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\p{L}\p{N}]+(?:['’-][\p{L}\p{N}]+)*").expect("a valid pattern"));

pub fn count_words(markdown: &str) -> usize {
    let read = UNREAD.replace_all(markdown, " ");
    WORD.find_iter(&read).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_the_words_and_none_of_the_markdown_around_them() {
        assert_eq!(
            count_words("## Standup\n\n- **Ship** it and `tag`\n> quoted _here_"),
            7
        );
        assert_eq!(
            count_words("See [the runbook](https://example.com/run-book)."),
            3
        );
        assert_eq!(count_words("- [x] ship it\n- [ ] tag it\n&#9;indented"), 5);
        assert_eq!(
            count_words("| Host | Port |\n| --- | --- |\n| db | 5432 |"),
            4
        );
    }

    #[test]
    fn keeps_a_word_whole_across_an_apostrophe_or_a_hyphen() {
        assert_eq!(count_words("l’équipe a re-déployé l'API"), 4);
    }

    #[test]
    fn says_zero_for_an_empty_note() {
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("  \n\n  "), 0);
    }
}
