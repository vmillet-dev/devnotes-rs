//! Folders: made from the rail, refused twice under one name, a selection filed and taken back
//! out, the chip a card wears, a note filed from its card, the notes left standing when the
//! folder goes — then folders inside folders: dragged onto another in the rail and offered back,
//! the list of an opened folder gathering its sub-tree, its own board one level down, a
//! sub-folder made from its panel, the folder dragged onto another space with its notes, and one
//! deleted with its notes, which wait in the trash.
//!
//! A space of its own, made here: the samples are in the system's language.

use e2e::{App, Tree, step};
use egui::accesskit::Node;
use egui::{Key, Modifiers, PointerButton, Pos2};

const SPACE: &str = "Dossiers";
const FIRST: &str = "Requête lente";
const SECOND: &str = "Plan d'exécution";
const NESTED: &str = "Index partiel sur jobs";
const DOOMED: &str = "Brouillon à jeter";

fn named(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label() == Some(name)
}

fn titles(tree: &Tree) -> Vec<String> {
    tree.all("note-card-title")
        .into_iter()
        .map(|node| node.label().or(node.value()).unwrap_or_default().to_owned())
        .collect()
}

/// How many rows of the rail are named `name`: the samples' folders are listed too.
fn rows_named(tree: &Tree, name: &str) -> usize {
    tree.texts("folder-option")
        .iter()
        .filter(|row| *row == name)
        .count()
}

fn crumb(tree: &Tree) -> Option<String> {
    tree.text("folder-breadcrumb-name")
}

fn centre(node: &Node) -> Pos2 {
    let rect = node.bounds().expect("a place on screen");
    #[expect(clippy::cast_possible_truncation, reason = "points on a screen")]
    Pos2::new(
        f64::midpoint(rect.x0, rect.x1) as f32,
        f64::midpoint(rect.y0, rect.y1) as f32,
    )
}

fn write(app: &mut App, title: &str) {
    app.click("new-note");
    app.fill("editor-title", title);
    app.fill("editor-body", "SELECT 1;");
    app.click("editor-close");
    app.eventually("the note is a card", |tree| {
        !tree.has("editor-title") && titles(tree).contains(&title.to_owned())
    });
}

fn make_folder(app: &mut App, name: &str) {
    app.click("folder-create-open");
    app.fill("folder-create-input", name);
    app.press(Key::Enter, Modifiers::NONE);
}

fn file_into(app: &mut App, name: &str) {
    app.click("selection-move");
    app.click_where("selection-move-place", ends_with(name));
}

/// The first place offered, whatever the language calls it: out of every folder.
fn unfile(app: &mut App) {
    app.click("selection-move");
    app.wait_for("selection-move-place");
    app.click("selection-move-place");
}

/// A folder's place: its path ends with its name.
fn ends_with(name: &str) -> impl Fn(&Node) -> bool {
    let end = format!(" › {name}");
    move |node| node.label().is_some_and(|label| label.ends_with(&end))
}

fn tick_all(app: &mut App) {
    app.click("search-input");
    app.press(Key::ArrowDown, Modifiers::NONE);
    for key in [Key::X, Key::ArrowRight, Key::X] {
        app.press(key, Modifiers::NONE);
    }
    app.eventually("two cards are ticked", |tree| {
        tree.text("selection-count")
            .is_some_and(|said| said.contains('2'))
    });
}

fn space_of_its_own(app: &mut App) {
    step(
        "a space of its own, two notes, a folder made from the rail",
        || {
            app.click("space-create-open");
            app.fill("space-create-input", SPACE);
            app.press(Key::Enter, Modifiers::NONE);
            app.eventually("the new space is shown", |tree| {
                tree.texts("space-option").contains(&SPACE.to_owned()) && !tree.has("note-card")
            });
            write(app, FIRST);
            write(app, SECOND);
            make_folder(app, "Perf");
            app.eventually("the folder is listed", |tree| rows_named(tree, "Perf") == 1);
        },
    );

    step("a second folder of the same name is refused", || {
        make_folder(app, "Perf");
        app.eventually("the refusal is said", |tree| tree.has("error-banner"));
        assert_eq!(rows_named(&app.tree(), "Perf"), 1);
        app.click("error-dismiss");
        app.press(Key::Escape, Modifiers::NONE);
    });
}

fn filing(app: &mut App) {
    step(
        "a whole selection is filed in one gesture, and says where",
        || {
            tick_all(app);
            file_into(app, "Perf");
            app.eventually("both cards wear the folder's chip", |tree| {
                tree.count("note-card-folder") == 2
            });
            // While the bar offers the filing back, Escape would take it back.
            app.click("undo-dismiss");
            app.press(Key::Escape, Modifiers::NONE);
            app.eventually("the selection is let go", |tree| {
                !tree.has("selection-count")
            });
        },
    );

    step(
        "the folder narrows the canvas, and the space opens it back",
        || {
            app.click_where("folder-option", named("Perf"));
            app.eventually("its notes alone, no chip naming it", |tree| {
                crumb(tree).as_deref() == Some("Perf")
                    && titles(tree).len() == 2
                    && !tree.has("note-card-folder")
            });
            app.click_where("space-option", named(SPACE));
            app.eventually("the whole space", |tree| crumb(tree).is_none());
        },
    );

    step("a selection taken back out wears no chip at all", || {
        tick_all(app);
        unfile(app);
        app.eventually("no card wears a chip", |tree| !tree.has("note-card-folder"));
        // While the bar offers the filing back, Escape would take it back.
        app.click("undo-dismiss");
        app.press(Key::Escape, Modifiers::NONE);
    });

    step("a note is filed from its card's menu", || {
        app.hover("note-card");
        app.click("note-card-menu");
        app.click("note-card-move");
        app.fill("note-card-move-filter", "perf");
        app.press(Key::Enter, Modifiers::NONE);
        app.eventually("one card wears the chip", |tree| {
            tree.count("note-card-folder") == 1
        });
    });

    step("deleting the folder leaves every note standing", || {
        app.click_where("folder-edit", |node| {
            node.label().is_some_and(|label| label.contains("Perf"))
        });
        app.click("folder-delete");
        app.click("folder-delete");
        app.eventually("the folder is gone, both notes kept", |tree| {
            rows_named(tree, "Perf") == 0
                && titles(tree).len() == 2
                && !tree.has("note-card-folder")
        });
    });
}

fn nested(app: &mut App) {
    step(
        "a folder dragged onto another goes inside it, offered back",
        || {
            make_folder(app, "Bases");
            app.eventually("listed", |tree| rows_named(tree, "Bases") == 1);
            make_folder(app, "Index");
            app.eventually("listed", |tree| rows_named(tree, "Index") == 1);
            app.click_where("folder-option", named("Index"));
            app.wait_for("folder-breadcrumb-name");
            write(app, NESTED);
            app.click_where("space-option", named(SPACE));
            app.eventually("the space", |tree| crumb(tree).is_none());
            let tree = app.tree();
            let index = tree
                .all("folder-option")
                .into_iter()
                .find(|node| node.label() == Some("Index"))
                .map(centre)
                .unwrap();
            let bases = tree
                .all("folder-option")
                .into_iter()
                .find(|node| node.label() == Some("Bases"))
                .map(centre)
                .unwrap();
            app.drag(index, bases, PointerButton::Primary);
            app.eventually(
                "« Index » is drawn inside « Bases », and offered back",
                |tree| {
                    let rows = tree.all("folder-option");
                    let left = |name: &str| {
                        rows.iter()
                            .find(|node| node.label() == Some(name))
                            .and_then(|node| node.bounds())
                            .map(|bounds| bounds.x0)
                    };
                    left("Index") > left("Bases") && tree.has("undo-restore")
                },
            );
            app.capture("rail-nested");
            app.click("undo-dismiss");
            app.eventually("the bar is gone", |tree| !tree.has("undo-bar"));
        },
    );

    step(
        "an opened folder lists its whole sub-tree, each card naming its own",
        || {
            app.click_where("folder-option", named("Bases"));
            app.eventually("the note of « Index » is listed", |tree| {
                crumb(tree).as_deref() == Some("Bases") && titles(tree).contains(&NESTED.to_owned())
            });
            // The chip shows the deepest folder and says its path from the top.
            assert_eq!(
                app.tree().text("note-card-folder").as_deref(),
                Some("Bases › Index")
            );
        },
    );

    step(
        "on the board, a folder that holds folders is a board one level down",
        || {
            app.click("folder-breadcrumb-back");
            app.eventually("the space", |tree| crumb(tree).is_none());
            app.click("view-board");
            app.click_where("board-zone-open", named("Bases"));
            app.eventually("« Index » is its one zone, holding its note", |tree| {
                crumb(tree).as_deref() == Some("Bases")
                    && tree.texts("board-zone-open") == ["Index"]
            });
            app.press(Key::Escape, Modifiers::NONE);
            app.eventually(
                "the space's board, « Index » inside « Bases »'s zone",
                |tree| {
                    crumb(tree).is_none()
                        && tree.texts("board-zone-open").contains(&"Bases".to_owned())
                        && tree.texts("board-zone-child").contains(&"Index".to_owned())
                },
            );
            app.click("view-list");
        },
    );

    step("a folder made inside another from its panel", || {
        app.click_where("folder-edit", |node| {
            node.label().is_some_and(|label| label.contains("Index"))
        });
        app.click("folder-subfolder-open");
        app.fill("folder-subfolder-input", "Partiels");
        app.press(Key::Enter, Modifiers::NONE);
        app.eventually("listed under « Index », the field gone", |tree| {
            tree.texts("folder-option").contains(&"Partiels".to_owned())
                && !tree.has("folder-subfolder-input")
        });
    });
}

/// « Bases », holding « Index » and its note, dropped onto the samples' space: the note is found
/// there, and the undo bar brings everything back.
fn to_another_space(app: &mut App) {
    step(
        "a folder dragged onto another space goes there with its notes, offered back",
        || {
            let tree = app.tree();
            let samples = tree
                .texts("space-option")
                .into_iter()
                .find(|name| name != SPACE)
                .expect("the samples' space");
            let at = |author_id: &str, name: &str| {
                tree.all(author_id)
                    .into_iter()
                    .find(|node| node.label() == Some(name))
                    .map_or_else(|| panic!("no row {name}"), centre)
            };
            let (bases, other) = (at("folder-option", "Bases"), at("space-option", &samples));
            app.drag(bases, other, PointerButton::Primary);
            app.wait_for("undo-restore");
            app.click_where("space-option", named(&samples));
            app.eventually("the note of « Index » is in the samples' space", |tree| {
                titles(tree).contains(&NESTED.to_owned())
            });
            app.click("undo-restore");
            app.click_where("space-option", named(SPACE));
            app.eventually("back in its space, with its note", |tree| {
                titles(tree).contains(&NESTED.to_owned())
            });
        },
    );
}

fn deleted_with_its_notes(app: &mut App) {
    step(
        "a folder deleted with its notes puts them in the trash",
        || {
            make_folder(app, "Jetable");
            app.eventually("the folder is listed", |tree| {
                rows_named(tree, "Jetable") == 1
            });
            app.click_where("folder-option", named("Jetable"));
            app.wait_for("folder-breadcrumb-name");
            write(app, DOOMED);
            app.click_where("folder-edit", |node| {
                node.label().is_some_and(|label| label.contains("Jetable"))
            });
            app.click("folder-delete-held");
            app.click("folder-delete-held-1");
            app.click("folder-delete");
            app.click("folder-delete");
            app.eventually("the folder is gone, its note with it", |tree| {
                rows_named(tree, "Jetable") == 0 && !titles(tree).contains(&DOOMED.to_owned())
            });
            app.click("trash-open");
            app.eventually("the note waits in the trash", |tree| {
                tree.texts("trash-row-title").contains(&DOOMED.to_owned())
            });
            app.press(Key::Escape, Modifiers::NONE);
        },
    );
}

#[test]
fn folders() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("17_folders", profile.path());
    app.create_library();
    app.wait_for("note-card");

    space_of_its_own(&mut app);
    filing(&mut app);
    nested(&mut app);
    to_another_space(&mut app);
    deleted_with_its_notes(&mut app);
    app.quit();
}
