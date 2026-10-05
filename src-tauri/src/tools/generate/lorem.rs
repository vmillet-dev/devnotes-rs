//! Lorem ipsum: words, sentences or paragraphs, from the classic vocabulary.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::{Draw, seed_or_draw};

const OPENING: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit";

const WORDS: &[&str] = &[
    "lorem",
    "ipsum",
    "dolor",
    "sit",
    "amet",
    "consectetur",
    "adipiscing",
    "elit",
    "sed",
    "do",
    "eiusmod",
    "tempor",
    "incididunt",
    "ut",
    "labore",
    "et",
    "dolore",
    "magna",
    "aliqua",
    "enim",
    "ad",
    "minim",
    "veniam",
    "quis",
    "nostrud",
    "exercitation",
    "ullamco",
    "laboris",
    "nisi",
    "aliquip",
    "ex",
    "ea",
    "commodo",
    "consequat",
    "duis",
    "aute",
    "irure",
    "in",
    "reprehenderit",
    "voluptate",
    "velit",
    "esse",
    "cillum",
    "fugiat",
    "nulla",
    "pariatur",
    "excepteur",
    "sint",
    "occaecat",
    "cupidatat",
    "non",
    "proident",
    "sunt",
    "culpa",
    "qui",
    "officia",
    "deserunt",
    "mollit",
    "anim",
    "id",
    "est",
    "laborum",
];

const MAX_WORDS: u32 = 5000;
const MAX_BLOCKS: u32 = 200;

pub(crate) fn word(draw: &mut Draw) -> &'static str {
    draw.pick(WORDS)
}

pub(crate) fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

pub(crate) fn words(draw: &mut Draw, count: usize, opening: bool) -> String {
    let mut out: Vec<&str> = if opening {
        OPENING.split(' ').take(count).collect()
    } else {
        Vec::new()
    };
    while out.len() < count {
        out.push(word(draw));
    }
    out.join(" ")
}

/// Six to fourteen words, a comma now and then, a capital and a full stop.
fn sentence(draw: &mut Draw, opening: bool) -> String {
    let length = usize::try_from(draw.between(6, 14)).unwrap_or(8);
    let mut words: Vec<String> = words(draw, length, opening)
        .split(' ')
        .map(str::to_owned)
        .collect();
    if length > 8 && !opening {
        let at = usize::try_from(draw.between(3, 5)).unwrap_or(3);
        words[at].push(',');
    }
    capitalised(&words.join(" ").replace(",,", ",")) + "."
}

fn paragraph(draw: &mut Draw, opening: bool) -> String {
    let count = draw.between(3, 6);
    (0..count)
        .map(|index| sentence(draw, opening && index == 0))
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LoremUnit {
    Words,
    Sentences,
    Paragraphs,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LoremRequest {
    pub unit: LoremUnit,
    pub count: u32,
    /// Opens on "Lorem ipsum dolor sit amet…", as the reader expects to recognise it.
    pub opening: bool,
    pub seed: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LoremAnswer {
    pub text: String,
    pub seed: u32,
    /// How many were made: the count asked, held within `at_most`, which the page says.
    pub count: u32,
    pub at_most: u32,
}

pub fn lorem(request: &LoremRequest) -> LoremAnswer {
    let seed = seed_or_draw(request.seed);
    let mut draw = Draw::new(seed);
    let at_most = match request.unit {
        LoremUnit::Words => MAX_WORDS,
        LoremUnit::Sentences | LoremUnit::Paragraphs => MAX_BLOCKS,
    };
    let count = request.count.clamp(1, at_most);
    let text = match request.unit {
        LoremUnit::Words => words(
            &mut draw,
            usize::try_from(count).unwrap_or(1),
            request.opening,
        ),
        LoremUnit::Sentences => (0..count)
            .map(|index| sentence(&mut draw, request.opening && index == 0))
            .collect::<Vec<_>>()
            .join(" "),
        LoremUnit::Paragraphs => (0..count)
            .map(|index| paragraph(&mut draw, request.opening && index == 0))
            .collect::<Vec<_>>()
            .join("\n\n"),
    };
    LoremAnswer {
        text,
        seed,
        count,
        at_most,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(unit: LoremUnit, count: u32, opening: bool) -> String {
        lorem(&LoremRequest {
            unit,
            count,
            opening,
            seed: Some(11),
        })
        .text
    }

    #[test]
    fn words_come_in_the_number_asked_and_may_open_on_the_classic() {
        assert_eq!(
            text(LoremUnit::Words, 5, true),
            "Lorem ipsum dolor sit amet,"
        );
        let drawn = text(LoremUnit::Words, 12, false);
        assert_eq!(drawn.split(' ').count(), 12);
        assert!(!drawn.starts_with("Lorem ipsum"));
    }

    #[test]
    fn sentences_are_capitalised_and_end_with_a_full_stop() {
        let drawn = text(LoremUnit::Sentences, 4, false);
        let sentences: Vec<&str> = drawn.split_inclusive(". ").collect();

        assert_eq!(sentences.len(), 4);
        assert!(
            sentences
                .iter()
                .all(|sentence| sentence.chars().next().unwrap().is_uppercase())
        );
        assert!(drawn.ends_with('.'));
    }

    #[test]
    fn paragraphs_are_apart_and_the_first_may_open_on_the_classic() {
        let drawn = text(LoremUnit::Paragraphs, 3, true);

        assert_eq!(drawn.split("\n\n").count(), 3);
        assert!(drawn.starts_with("Lorem ipsum dolor sit amet, consectetur"));
    }

    #[test]
    fn a_seed_draws_the_same_text_again() {
        assert_eq!(
            text(LoremUnit::Paragraphs, 2, false),
            text(LoremUnit::Paragraphs, 2, false)
        );
    }

    #[test]
    fn a_count_past_the_bound_is_held_to_it_and_says_so() {
        let answer = lorem(&LoremRequest {
            unit: LoremUnit::Paragraphs,
            count: 1000,
            opening: false,
            seed: Some(7),
        });
        assert_eq!((answer.count, answer.at_most), (MAX_BLOCKS, MAX_BLOCKS));
        assert_eq!(answer.text.split("\n\n").count(), MAX_BLOCKS as usize);

        let words = lorem(&LoremRequest {
            unit: LoremUnit::Words,
            count: 0,
            opening: true,
            seed: Some(7),
        });
        assert_eq!((words.count, words.at_most), (1, MAX_WORDS));
    }
}
