//! The words of a Note as it reads, not as its Markdown is spelled (the reference's
//! `word-count.ts`), and what a double click or a step by word takes.

use std::ops::Range;
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

/// What a word is made of, for every editor: 0 a space, 1 punctuation, 2 a word character
/// (a letter, a digit, `_`). A run of one class is one word.
pub(crate) fn class(c: char) -> u8 {
    if c.is_alphanumeric() || c == '_' {
        2
    } else {
        u8::from(!c.is_whitespace())
    }
}

/// What a double click on the `char` at `at` takes: the run of word characters, punctuation or
/// spaces holding it. Past the end of its line, the line's last run; a line end stops it.
pub fn word_around(text: &str, at: usize) -> Range<usize> {
    let chars: Vec<char> = text.chars().collect();
    let kind = |i: usize| chars.get(i).filter(|c| **c != '\n').map(|c| class(*c));
    let at = at.min(chars.len());
    let probe = if kind(at).is_some() {
        at
    } else if at > 0 && kind(at - 1).is_some() {
        at - 1
    } else {
        return at..at;
    };
    let run = kind(probe);
    let mut start = probe;
    while start > 0 && kind(start - 1) == run {
        start -= 1;
    }
    let mut end = probe + 1;
    while kind(end) == run {
        end += 1;
    }
    start..end
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

    fn word(text: &str, offset: usize) -> &str {
        let range = word_around(text, offset);
        let start = text
            .char_indices()
            .nth(range.start)
            .map_or(text.len(), |(at, _)| at);
        let end = text
            .char_indices()
            .nth(range.end)
            .map_or(text.len(), |(at, _)| at);
        &text[start..end]
    }

    #[test]
    fn a_double_click_takes_the_run_it_touches() {
        assert_eq!(word("déjà-vu", 3), "déjà");
        assert_eq!(word("déjà-vu", 4), "-");
        assert_eq!(word("déjà-vu", 5), "vu");
        assert_eq!(word("foo_bar baz", 3), "foo_bar");
        assert_eq!(word("a.b", 0), "a");
        assert_eq!(word("a.b", 1), ".");
        assert_eq!(word("a.b", 2), "b");
        assert_eq!(word("hello world", 5), " ");
        assert_eq!(word("a   b", 2), "   ");
        assert_eq!(word("ok 🙂🙂 go", 3), "🙂🙂");
        assert_eq!(word("one\ntwo", 3), "one", "past the end of its line");
        assert_eq!(word("one\ntwo", 4), "two");
        assert_eq!(word("", 0), "");
        assert_eq!(word("end", 9), "end");
    }

    #[test]
    fn says_zero_for_an_empty_note() {
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("  \n\n  "), 0);
    }
}
