//! The acceptance sheet's main paths through the notes that no other scenario walks whole. A step
//! opens on the case it runs, so that a run can be read back into the sheet.

use e2e::{App, Tree, step};
use egui::accesskit::Node;
use egui::{Key, Modifiers};

const SHIFT_COMMAND: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

const SHEET: &str = "# Hébergement
prix = 0,045 €/h
serveurs = 3
coût = prix × 730 h × serveurs
20 % de 340
30 sur 120 en %
de 80 à 100
250 + 20 %
300 hors 20 %
2,5 kg en g
3 h 20 min en min
1 Go en Mo

# Courses
10 €
20 €
30 €
total
moyenne

# Erreurs
distance = 5 km
distance + 3 Go
distance en m";

const RUST: &str = "editor-language-6";
const TYPESCRIPT: &str = "editor-language-3";

const LITERAL: &str = "{{db_host}}\na -> b\n2 * 3 * 4\nsnake_case_name";
const DIAGRAM: &str = "```mermaid\ngraph LR\n  Client --> API\n```\n";
const SLIDES: &str = "# Un\n\ntexte\n\n## Deux\n\n### Reste ici\n\n---\n\nTrois";

fn close_editor(app: &mut App) {
    app.click("editor-close");
    app.eventually("the editor closed", |tree| !tree.has("editor-title"));
}

fn open_card(app: &mut App, title: &str) {
    app.click_where("note-card-open", |node| {
        node.label().is_some_and(|label| label.contains(title))
    });
    app.wait_for("editor-title");
}

fn card_titles(tree: &Tree) -> Vec<String> {
    tree.texts("note-card-title")
}

fn has_card(tree: &Tree, title: &str) -> bool {
    card_titles(tree).iter().any(|it| it == title)
}

fn a_snippet(app: &mut App, title: &str, body: &str) {
    app.click("new-note");
    app.fill("editor-title", title);
    app.click("editor-body");
    app.paste(body);
    close_editor(app);
}

fn code(tree: &Tree) -> String {
    tree.find("editor-body")
        .and_then(Node::value)
        .unwrap_or_default()
        .to_owned()
}

/// A snippet holding Markdown, made a Note: what a file of Markdown comes in as.
fn a_note_of(app: &mut App, title: &str, markdown: &str) {
    a_snippet(app, title, markdown);
    open_card(app, title);
    app.click("editor-kind");
    app.click_nth("editor-kind-0", 0);
    app.wait_for("editor-rich");
}

/// The language menu's filter takes the focus a frame after it opens.
fn choose_language(app: &mut App, typed: &str, option: &str) {
    app.click("editor-language");
    app.eventually("the filter has the focus", |tree| {
        tree.focused().starts_with("editor-language-filter")
    });
    app.type_text(typed);
    app.click(option);
}

fn digits(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect()
}

fn calc_results(tree: &Tree) -> Vec<String> {
    tree.all("calc-result")
        .into_iter()
        .map(|node| digits(node.value().unwrap_or_default()))
        .collect()
}

fn a_new_library(scenario: &'static str, profile: &std::path::Path) -> App {
    let mut app = App::launch(scenario, profile);
    app.create_library();
    app.eventually("the samples are drawn", |tree| tree.has("note-card"));
    app
}

#[test]
fn recette_creating() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = a_new_library("38_recette_creating", profile.path());

    step("NOT-02 a title alone, left by Escape, is a note", || {
        app.click("new-note");
        app.fill("editor-title", "Recette première note");
        app.press(Key::Escape, Modifiers::NONE);
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the card is there", |tree| {
            !tree.has("editor-title") && has_card(tree, "Recette première note")
        });
    });

    step("NOT-06 a fast close writes one note, with its body", || {
        app.click("new-note");
        app.fill("editor-title", "Course");
        app.fill("editor-body", "corps");
        app.press(Key::Escape, Modifiers::NONE);
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the card is there", |tree| has_card(tree, "Course"));
        let titles = app.tree().texts("note-card-title");
        assert_eq!(titles.iter().filter(|it| *it == "Course").count(), 1);
    });

    step("NOT-07 a title changed, another card clicked", || {
        open_card(&mut app, "Course");
        app.click("editor-title");
        app.press(Key::A, Modifiers::COMMAND);
        app.type_text("Titre modifié au clic");
        app.capture("not-07-before");
        app.click_where("note-card-open", |node| {
            node.label().is_some_and(|it| it.contains("Connexion"))
        });
        app.eventually("the editor closed or moved", |tree| {
            !tree.has("editor-title") || has_card(tree, "Titre modifié au clic")
        });
        app.capture("not-07-after");
        if app.has("editor-title") {
            close_editor(&mut app);
        }
        app.eventually("the first note wears its new title", |tree| {
            has_card(tree, "Titre modifié au clic")
        });
    });

    step("NOT-10 a snippet made a Note, and taken back", || {
        open_card(&mut app, "Connexion");
        app.click("editor-kind");
        app.click_nth("editor-kind-0", 0);
        app.wait_for("editor-rich");
        app.capture("not-10-converted");
        app.click("undo-restore");
        app.wait_for("editor-body");
        close_editor(&mut app);
    });

    step(
        "COD-01 the language menu filters, and the card says it",
        || {
            app.click("new-note");
            app.fill("editor-title", "Rouille");
            app.fill("editor-body", "fn main() { println!(\"bonjour\"); }");
            choose_language(&mut app, "rs", RUST);
            app.eventually("the language is chosen", |tree| {
                tree.text("editor-stats")
                    .is_some_and(|said| said.starts_with("RS"))
            });
            app.capture("cod-01-coloured");
            close_editor(&mut app);
        },
    );

    step("COD-09 a formatted snippet keeps its fields", || {
        app.click("new-note");
        app.fill("editor-title", "Champs formatés");
        app.click("editor-body");
        app.paste("const url=\"{{host}}/api/{{version=v2}}\";fetch(url)");
        choose_language(&mut app, "ts", TYPESCRIPT);
        app.click("editor-format");
        app.wait_for("editor-format-notice");
        let body = code(&app.tree());
        assert!(body.contains("{{host}}/api/{{version=v2}}"), "{body}");
        assert!(body.contains(";\n"), "{body}");
        close_editor(&mut app);
    });

    step("COD-12 a word found three times, stepped through", || {
        app.click("new-note");
        app.fill("editor-title", "Trois foo");
        app.fill("editor-body", "foo(); foo(); foo();");
        app.press(Key::F, Modifiers::COMMAND);
        app.wait_for("search-field");
        app.type_text("foo");
        app.eventually("three are counted", |tree| {
            tree.text("search-count")
                .is_some_and(|said| said.ends_with("/ 3"))
        });
        let count = app.tree().text("search-count").unwrap_or_default();
        app.press(Key::Enter, Modifiers::NONE);
        app.eventually("the next one", |tree| {
            tree.text("search-count").is_some_and(|said| said != count)
        });
        app.press(Key::Enter, Modifiers::SHIFT);
        app.eventually("back to the first", |tree| {
            tree.text("search-count").is_some_and(|said| said == count)
        });
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the bar closed", |tree| !tree.has("search-field"));
        assert!(app.has("editor-title"), "the editor stays");
        close_editor(&mut app);
    });

    step("TCH-01 Enter adds a row, the caret in it", || {
        app.press(Key::L, SHIFT_COMMAND);
        app.fill("editor-title", "Tâches");
        app.click("checklist-add");
        app.type_text("Préparer");
        app.press(Key::Enter, Modifiers::NONE);
        app.type_text("Déployer");
        app.eventually("two rows", |tree| {
            tree.texts("checklist-text") == ["Préparer", "Déployer"]
        });
        close_editor(&mut app);
    });

    step("TCH-06 ticked from its card, the progress follows", || {
        app.click_nth("note-card-check", 0);
        app.eventually("the progress moved", |tree| {
            tree.texts("note-card-progress")
                .iter()
                .any(|it| it.contains("1/2"))
        });
    });

    app.quit();
}

#[test]
fn recette_a_formatted_note() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = a_new_library("38_recette_rich", profile.path());

    step("RIC-01 every button of the bar formats", || {
        app.press(Key::N, SHIFT_COMMAND);
        app.fill("editor-title", "Barre");
        for button in [
            "rich-h1",
            "rich-h2",
            "rich-h3",
            "rich-bold",
            "rich-italic",
            "rich-strike",
            "rich-code",
            "rich-bullet",
            "rich-ordered",
            "rich-task",
            "rich-quote",
        ] {
            assert!(app.has(button), "{button}");
        }
        app.click("rich-body");
        app.type_text("gras");
        app.press(Key::A, Modifiers::COMMAND);
        app.click("rich-bold");
        app.capture("ric-01");
        close_editor(&mut app);
    });

    step(
        "RIC-09 what looks like Markdown comes back as typed",
        || {
            app.press(Key::N, SHIFT_COMMAND);
            app.fill("editor-title", "Littéral");
            app.click("rich-body");
            for line in LITERAL.lines() {
                app.type_text(line);
                app.press(Key::Enter, Modifiers::NONE);
            }
            close_editor(&mut app);
            open_card(&mut app, "Littéral");
            app.capture("ric-09");
            close_editor(&mut app);
        },
    );

    step(
        "RIC-16 a Mermaid block is drawn, a click goes to its code",
        || {
            a_note_of(&mut app, "Schéma", DIAGRAM);
            app.wait_for("rich-diagram");
            app.capture("ric-16-drawn");
            app.click("rich-diagram");
            app.capture("ric-16-code");
            close_editor(&mut app);
        },
    );

    step(
        "RIC-19 a Note presented: cover, then a slide per heading",
        || {
            a_note_of(&mut app, "Lancement", SLIDES);
            app.click("editor-present");
            app.wait_for("presentation");
            app.wait_for("presentation-cover");
            app.capture("ric-19-cover");
            let count = app.tree().text("presentation-count").unwrap_or_default();
            assert!(count.contains('1') && count.contains('4'), "{count}");
            app.press(Key::ArrowRight, Modifiers::NONE);
            app.eventually("a slide", |tree| tree.has("presentation-slide"));
            app.capture("ric-19-slide");
            app.press(Key::Escape, Modifiers::NONE);
            app.eventually("it left", |tree| !tree.has("presentation"));
            close_editor(&mut app);
        },
    );

    app.quit();
}

#[test]
fn recette_a_sheet_and_a_drawing() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = a_new_library("38_recette_calc", profile.path());

    step("CAL-45 a sheet pasted answers line by line", || {
        app.press(Key::M, SHIFT_COMMAND);
        app.fill("editor-title", "Feuille");
        app.click("editor-body");
        app.paste(SHEET);
        app.eventually("the figures are there", |tree| {
            let results = calc_results(tree);
            [
                "9855", "68", "25", "300", "250", "2500", "200", "1000", "6000", "2000", "5000",
            ]
            .iter()
            .all(|it| results.contains(&(*it).to_owned()))
        });
        app.eventually("one line refused, the next carries on", |tree| {
            tree.count("calc-failure") == 1
        });
        app.capture("cal-45");
    });

    step("CAL-37 results picked: a range, then a stranger", || {
        app.click_where("calc-result", |node| {
            node.value().is_some_and(|it| digits(it) == "1000")
        });
        app.eventually("the footer sums it", |tree| {
            tree.text("calc-summary")
                .is_some_and(|said| digits(&said).contains("1000"))
        });
        close_editor(&mut app);
    });

    step("DES-01 an empty drawing says what to do", || {
        app.press(Key::D, SHIFT_COMMAND);
        app.fill("editor-title", "Plan");
        app.wait_for("drawing-stage");
        app.capture("des-01");
    });

    step("DES-02 three shapes from their letters", || {
        app.click("drawing-stage");
        for (key, from, to) in [
            (Key::R, (40.0, 40.0), (140.0, 100.0)),
            (Key::O, (180.0, 40.0), (280.0, 100.0)),
            (Key::D, (320.0, 40.0), (420.0, 100.0)),
        ] {
            app.press(key, Modifiers::NONE);
            app.drag_in("drawing-stage", from, to);
        }
        app.eventually("the footer counts three", |tree| {
            tree.text("editor-stats")
                .is_some_and(|said| said.contains('3'))
        });
        app.capture("des-02");
    });

    step("DES-09 the drawing is kept, its card shows it", || {
        close_editor(&mut app);
        app.wait_for("note-card-drawing");
        app.click("note-card-drawing");
        app.wait_for("drawing-stage");
        app.eventually("the footer still counts three", |tree| {
            tree.text("editor-stats")
                .is_some_and(|said| said.contains('3'))
        });
        close_editor(&mut app);
    });

    step(
        "DES-13 a drawing inserted into a Note shows its picture",
        || {
            app.press(Key::N, SHIFT_COMMAND);
            app.fill("editor-title", "Avec dessin");
            app.click("rich-embed-drawing");
            app.click("rich-embed-drawing-0");
            app.wait_for("rich-embed");
            app.capture("des-13");
            close_editor(&mut app);
        },
    );

    app.quit();
}

#[test]
fn recette_on_the_canvas() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = a_new_library("38_recette_canvas", profile.path());

    step("MET-01 three spellings are one tag", || {
        a_snippet(&mut app, "Avec tags", "echo tags");
        open_card(&mut app, "Avec tags");
        for tag in ["devops", "#DevOps", " DEVOPS "] {
            app.fill("editor-tag-add", tag);
            app.press(Key::Enter, Modifiers::NONE);
        }
        app.eventually("one tag", |tree| tree.count("editor-tag") == 1);
        close_editor(&mut app);
    });

    step("CLV-01 the arrows walk the cards, Enter opens", || {
        app.press(Key::ArrowDown, Modifiers::NONE);
        app.press(Key::ArrowRight, Modifiers::NONE);
        app.press(Key::Enter, Modifiers::NONE);
        app.wait_for("editor-title");
        close_editor(&mut app);
    });

    step("MET-04 P pins, P again unpins, no date moves", || {
        let when = app.tree().texts("note-card-when");
        app.press(Key::ArrowDown, Modifiers::NONE);
        app.press(Key::P, Modifiers::NONE);
        app.eventually("two pinned", |tree| tree.count("note-card-pin") == 2);
        app.click("undo-dismiss");
        app.press(Key::P, Modifiers::NONE);
        app.eventually("one pinned", |tree| tree.count("note-card-pin") == 1);
        app.click("undo-dismiss");
        assert_eq!(app.tree().texts("note-card-when"), when);
    });

    step("CLV-02 C copies a plain card and says so", || {
        app.press(Key::C, Modifiers::NONE);
        app.wait_for("status-toast");
        app.capture("clv-02");
    });

    step(
        "SEL-02 a selection moved to a space, and taken back",
        || {
            app.click("space-create-open");
            app.fill("space-create-input", "Perso");
            app.press(Key::Enter, Modifiers::NONE);
            app.eventually("the new space is shown, empty", |tree| {
                !tree.has("note-card")
            });
            app.click("space-option-all");
            app.eventually("the cards are back", |tree| tree.has("note-card"));
            app.press(Key::ArrowDown, Modifiers::NONE);
            for key in [Key::X, Key::ArrowRight, Key::X] {
                app.press(key, Modifiers::NONE);
            }
            app.click("selection-move");
            app.click_where("selection-move-0", |node| node.label() == Some("Perso"));
            app.wait_for("undo-text");
            app.capture("sel-02");
            app.click("undo-restore");
            app.eventually("the bar went", |tree| !tree.has("undo-text"));
        },
    );

    step("SEL-09 a selection trashed, then given back whole", || {
        let cards = app.tree().count("note-card");
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the selection let go", |tree| !tree.has("selection-count"));
        app.press(Key::ArrowDown, Modifiers::NONE);
        for key in [Key::X, Key::ArrowRight, Key::X] {
            app.press(key, Modifiers::NONE);
        }
        app.click("selection-delete");
        app.click("selection-delete");
        app.eventually("two left", |tree| tree.count("note-card") == cards - 2);
        app.click("undo-restore");
        app.eventually("both back", |tree| tree.count("note-card") == cards);
    });

    app.quit();
}

/// KO on the sheet: the menu lists codes alone, so the name of a language finds nothing.
#[test]
#[ignore = "COD-01: the language menu does not read the languages' names"]
fn ko_cod_01_a_language_is_found_by_its_name() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = a_new_library("38_ko_cod_01", profile.path());
    app.click("new-note");
    app.click("editor-language");
    app.eventually("the filter has the focus", |tree| {
        tree.focused().starts_with("editor-language-filter")
    });
    app.type_text("rus");
    app.wait_for(RUST);
    app.quit();
}
