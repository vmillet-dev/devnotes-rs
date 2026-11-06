//! Mustache: the syntax and its examples here, what each does in the reference's words.

use i18n::{Language, keys};

use super::reference::{Reference, Table, columns_of, groups_of, word};
use crate::tools::Tool;
use crate::tools::ui::{Cell, ReferenceRow};

const GROUPS: [&str; 3] = ["values", "sections", "more"];
const WIDE_CARD: f32 = 760.0;

struct Entry {
    syntax: &'static str,
    group: &'static str,
    template: &'static str,
    /// What the template is rendered with: its view as JSON, a partial.
    view: &'static str,
    output: &'static str,
}

const ENTRIES: [Entry; 13] = [
    Entry {
        syntax: "{{name}}",
        group: "values",
        template: "Bonjour {{name}}",
        view: "{ \"name\": \"Ada\" }",
        output: "Bonjour Ada",
    },
    Entry {
        syntax: "{{{name}}}",
        group: "values",
        template: "{{{html}}}",
        view: "{ \"html\": \"<b>Ada</b>\" }",
        output: "<b>Ada</b>",
    },
    Entry {
        syntax: "{{& name}}",
        group: "values",
        template: "{{& html}}",
        view: "{ \"html\": \"<b>Ada</b>\" }",
        output: "<b>Ada</b>",
    },
    Entry {
        syntax: "{{a.b}}",
        group: "values",
        template: "{{user.name}}",
        view: "{ \"user\": { \"name\": \"Ada\" } }",
        output: "Ada",
    },
    Entry {
        syntax: "{{missing}}",
        group: "values",
        template: "[{{missing}}]",
        view: "{}",
        output: "[]",
    },
    Entry {
        syntax: "{{#list}}…{{/list}}",
        group: "sections",
        template: "{{#items}}\n- {{label}}\n{{/items}}",
        view: "{ \"items\": [{ \"label\": \"un\" }, { \"label\": \"deux\" }] }",
        output: "- un\n- deux",
    },
    Entry {
        syntax: "{{#flag}}…{{/flag}}",
        group: "sections",
        template: "{{#admin}}Accès complet{{/admin}}",
        view: "{ \"admin\": true }",
        output: "Accès complet",
    },
    Entry {
        syntax: "{{#object}}…{{/object}}",
        group: "sections",
        template: "{{#user}}{{name}} ({{role}}){{/user}}",
        view: "{ \"user\": { \"name\": \"Ada\", \"role\": \"admin\" } }",
        output: "Ada (admin)",
    },
    Entry {
        syntax: "{{^list}}…{{/list}}",
        group: "sections",
        template: "{{^items}}Aucun élément{{/items}}",
        view: "{ \"items\": [] }",
        output: "Aucun élément",
    },
    Entry {
        syntax: "{{.}}",
        group: "sections",
        template: "{{#tags}}#{{.}} {{/tags}}",
        view: "{ \"tags\": [\"api\", \"web\"] }",
        output: "#api #web",
    },
    Entry {
        syntax: "{{! … }}",
        group: "more",
        template: "a{{! jamais rendu }}b",
        view: "{}",
        output: "ab",
    },
    Entry {
        syntax: "{{> partial}}",
        group: "more",
        template: "{{> header}}",
        view: "{ \"title\": \"Docs\" }, header = \"# {{title}}\"",
        output: "# Docs",
    },
    Entry {
        syntax: "{{=<% %>=}}",
        group: "more",
        template: "{{=<% %>=}}<% name %>",
        view: "{ \"name\": \"Ada\" }",
        output: "Ada",
    },
];

fn row(entry: &Entry, language: Language) -> ReferenceRow {
    let syntax = entry.syntax;
    let what = word(
        language,
        &format!("tools.mustache-sheet.words.entries.{syntax}"),
    );
    let heading = word(
        language,
        &format!("tools.mustache-sheet.groups.{}", entry.group),
    );
    let with = i18n::translate(language, keys::tools::mustache_sheet::WITH, &[]);
    let example =
        Cell::mono(entry.template).below(format!("{with} {} → {}", entry.view, entry.output));
    let cells = vec![Cell::mono(syntax), Cell::text(what.clone()), example];
    ReferenceRow::new(syntax, entry.group, cells)
        .searched_by([syntax, &what, &heading])
        .found_as(syntax, what)
}

pub(crate) fn table(language: Language) -> Table {
    Table {
        rows: ENTRIES.iter().map(|it| row(it, language)).collect(),
        groups: groups_of(language, "tools.mustache-sheet.groups", &GROUPS),
        columns: columns_of(
            language,
            "tools.mustache-sheet.columns",
            &[
                ("syntax", Some(0.25)),
                ("what", Some(0.32)),
                ("example", None),
            ],
        ),
        caption: keys::tools::mustache_sheet::NAME,
        placeholder: keys::tools::mustache_sheet::PLACEHOLDER,
        card_width: WIDE_CARD,
        matches: None,
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Reference::of(table)
}
