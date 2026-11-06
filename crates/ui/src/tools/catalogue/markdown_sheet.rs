//! Markdown: the syntax and its examples here, what each does in the reference's words.

use i18n::{Language, keys};

use super::reference::{Reference, Table, columns_of, groups_of, word};
use crate::tools::Tool;
use crate::tools::ui::{Cell, ReferenceRow};

const GROUPS: [&str; 7] = [
    "headings", "emphasis", "lists", "links", "code", "blocks", "raw",
];
const WIDE_CARD: f32 = 760.0;
/// A space that matters is drawn so, and copied as a space.
const SPACE: char = '␣';

struct Entry {
    syntax: &'static str,
    group: &'static str,
    commonmark: bool,
    /// GitHub Flavored Markdown, and what github.com adds to it: footnotes, emoji.
    github: bool,
    example: &'static str,
}

const ENTRIES: [Entry; 32] = [
    Entry {
        syntax: "# …",
        group: "headings",
        commonmark: true,
        github: true,
        example: "## Installation",
    },
    Entry {
        syntax: "…\n===",
        group: "headings",
        commonmark: true,
        github: true,
        example: "Installation\n============",
    },
    Entry {
        syntax: "**…**",
        group: "emphasis",
        commonmark: true,
        github: true,
        example: "**important**",
    },
    Entry {
        syntax: "__…__",
        group: "emphasis",
        commonmark: true,
        github: true,
        example: "__important__",
    },
    Entry {
        syntax: "*…*",
        group: "emphasis",
        commonmark: true,
        github: true,
        example: "*nuance*",
    },
    Entry {
        syntax: "_…_",
        group: "emphasis",
        commonmark: true,
        github: true,
        example: "_nuance_",
    },
    Entry {
        syntax: "***…***",
        group: "emphasis",
        commonmark: true,
        github: true,
        example: "***crucial***",
    },
    Entry {
        syntax: "~~…~~",
        group: "emphasis",
        commonmark: false,
        github: true,
        example: "~~obsolète~~",
    },
    Entry {
        syntax: "- …",
        group: "lists",
        commonmark: true,
        github: true,
        example: "- un\n- deux",
    },
    Entry {
        syntax: "1. …",
        group: "lists",
        commonmark: true,
        github: true,
        example: "1. premier\n2. second",
    },
    Entry {
        syntax: "␣␣- …",
        group: "lists",
        commonmark: true,
        github: true,
        example: "- un\n  - sous-élément",
    },
    Entry {
        syntax: "- [ ] …",
        group: "lists",
        commonmark: false,
        github: true,
        example: "- [ ] à faire\n- [x] fait",
    },
    Entry {
        syntax: "[…](url)",
        group: "links",
        commonmark: true,
        github: true,
        example: "[la doc](https://exemple.fr)",
    },
    Entry {
        syntax: "[…](url \"…\")",
        group: "links",
        commonmark: true,
        github: true,
        example: "[la doc](https://exemple.fr \"Guide\")",
    },
    Entry {
        syntax: "<url>",
        group: "links",
        commonmark: true,
        github: true,
        example: "<https://exemple.fr>",
    },
    Entry {
        syntax: "https://…",
        group: "links",
        commonmark: false,
        github: true,
        example: "Voir https://exemple.fr",
    },
    Entry {
        syntax: "[…][ref]",
        group: "links",
        commonmark: true,
        github: true,
        example: "[la doc][1]\n\n[1]: https://exemple.fr",
    },
    Entry {
        syntax: "![…](url)",
        group: "links",
        commonmark: true,
        github: true,
        example: "![logo](logo.png)",
    },
    Entry {
        syntax: "`…`",
        group: "code",
        commonmark: true,
        github: true,
        example: "`npm install`",
    },
    Entry {
        syntax: "```lang",
        group: "code",
        commonmark: true,
        github: true,
        example: "```js\nconst port = 8080;\n```",
    },
    Entry {
        syntax: "␣␣␣␣…",
        group: "code",
        commonmark: true,
        github: true,
        example: "    code indenté",
    },
    Entry {
        syntax: "> …",
        group: "blocks",
        commonmark: true,
        github: true,
        example: "> citation\n>\n> > imbriquée",
    },
    Entry {
        syntax: "---",
        group: "blocks",
        commonmark: true,
        github: true,
        example: "au-dessus\n\n---\n\nen dessous",
    },
    Entry {
        syntax: "| … |",
        group: "blocks",
        commonmark: false,
        github: true,
        example: "| Nom | Port |\n| --- | --- |\n| api | 8080 |",
    },
    Entry {
        syntax: "| :-- | :-: | --: |",
        group: "blocks",
        commonmark: false,
        github: true,
        example: "| Gauche | Centre | Droite |\n| :-- | :-: | --: |",
    },
    Entry {
        syntax: "…␣␣",
        group: "raw",
        commonmark: true,
        github: true,
        example: "ligne  \nsuivante",
    },
    Entry {
        syntax: "…\\",
        group: "raw",
        commonmark: true,
        github: true,
        example: "ligne\\\nsuivante",
    },
    Entry {
        syntax: "\\*",
        group: "raw",
        commonmark: true,
        github: true,
        example: "\\*pas en italique\\*",
    },
    Entry {
        syntax: "<tag>",
        group: "raw",
        commonmark: true,
        github: true,
        example: "<kbd>Ctrl</kbd>+<kbd>C</kbd>",
    },
    Entry {
        syntax: "<!-- … -->",
        group: "raw",
        commonmark: true,
        github: true,
        example: "<!-- jamais affiché -->",
    },
    Entry {
        syntax: "[^1]",
        group: "raw",
        commonmark: false,
        github: true,
        example: "Texte[^1]\n\n[^1]: La note.",
    },
    Entry {
        syntax: ":emoji:",
        group: "raw",
        commonmark: false,
        github: true,
        example: "Bravo :tada:",
    },
];

fn works(yes: bool) -> Cell {
    if yes {
        Cell::checked()
    } else {
        Cell::text("—")
    }
}

fn row(entry: &Entry, language: Language) -> ReferenceRow {
    let syntax = entry.syntax;
    let what = word(
        language,
        &format!("tools.markdown-sheet.words.entries.{syntax}"),
    );
    let heading = word(
        language,
        &format!("tools.markdown-sheet.groups.{}", entry.group),
    );
    let cells = vec![
        Cell::mono(syntax),
        Cell::text(what.clone()),
        Cell::mono(entry.example),
        works(entry.commonmark),
        works(entry.github),
    ];
    let mut row = ReferenceRow::new(syntax, entry.group, cells)
        .searched_by([syntax, &what, &heading])
        .found_as(syntax, what);
    if syntax.contains(SPACE) {
        row.copy = Some((syntax.replace(SPACE, " "), syntax.to_owned()));
    }
    row
}

pub(crate) fn table(language: Language) -> Table {
    Table {
        rows: ENTRIES.iter().map(|it| row(it, language)).collect(),
        groups: groups_of(language, "tools.markdown-sheet.groups", &GROUPS),
        columns: columns_of(
            language,
            "tools.markdown-sheet.columns",
            &[
                ("syntax", Some(0.15)),
                ("what", None),
                ("example", Some(0.3)),
                ("commonmark", Some(0.13)),
                ("github", Some(0.1)),
            ],
        ),
        caption: keys::tools::markdown_sheet::NAME,
        placeholder: keys::tools::markdown_sheet::PLACEHOLDER,
        card_width: WIDE_CARD,
        matches: None,
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Reference::of(table)
}
