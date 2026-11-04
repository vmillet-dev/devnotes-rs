//! The library's tags managed from the tag rail: renamed, merged onto an existing tag, corrected
//! in case alone, deleted — each said before it runs. The tags are the scenario's own, given
//! from the selection bar: the samples' are in the system's language.

use e2e::{App, step};
use egui::{Key, Modifiers};

const CARDS: usize = 6;

/// The cards at `positions` ticked from their boxes, then given `tag` from the selection bar.
/// The first is ticked by the box its hover shows; the others by their place, below the bar
/// that floats over the first row once one is ticked.
fn tag(app: &mut App, positions: &[usize], tag: &str) {
    let (first, rest) = positions.split_first().expect("a card to tag");
    app.hover_nth("note-card-open", *first);
    app.click("note-card-check");
    app.eventually("the cards show their boxes", |tree| {
        tree.count("note-card-check") > 2
    });
    for at in rest {
        app.click_nth("note-card-check", *at);
    }
    app.eventually("the cards are ticked", |tree| {
        tree.text("selection-count")
            .is_some_and(|said| said.contains(&positions.len().to_string()))
    });
    app.fill("selection-tag", tag);
    app.press(Key::Enter, Modifiers::NONE);
    app.click("undo-dismiss");
    app.click("selection-clear");
    app.eventually("the selection is let go", |tree| {
        !tree.has("selection-count")
    });
}

fn listed(tree: &e2e::Tree, tag: &str) -> bool {
    let name = format!("#{tag}");
    tree.texts("tag-item").contains(&name)
}

fn tick(app: &mut App, tag: &str) {
    let name = format!("#{tag}");
    app.click_where("tag-item", |node| node.label() == Some(name.as_str()));
}

/// The ticked tags under `into`, proposed then confirmed.
fn rename(app: &mut App, into: &str) {
    app.fill("tag-target", into);
    app.click("tag-apply");
    app.click("tag-confirm-apply");
}

#[test]
fn tag_manager() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("14_tag_manager", profile.path());
    app.create_library();
    app.eventually("the samples are drawn", |tree| {
        tree.count("note-card") == CARDS
    });
    tag(&mut app, &[0, 1], "staging");
    tag(&mut app, &[0], "urgent");
    tag(&mut app, &[2], "preprod");

    step("the rail opens the manager, which lists every tag", || {
        app.click("facets-toggle");
        app.click("tag-manage");
        app.eventually("the scenario's tags are listed", |tree| {
            ["staging", "urgent", "preprod"]
                .iter()
                .all(|tag| listed(tree, tag))
        });
        assert!(
            app.tree()
                .find("tag-apply")
                .is_some_and(egui::accesskit::Node::is_disabled),
            "nothing ticked, nothing to apply"
        );
    });

    step(
        "a rename says what it touches, and cancelling writes nothing",
        || {
            tick(&mut app, "staging");
            app.fill("tag-target", "recette");
            app.click("tag-apply");
            app.eventually("two notes carry it, and the sentence says so", |tree| {
                tree.text("tag-confirm-text")
                    .is_some_and(|said| said.contains('2'))
            });
            app.capture("tag-confirm");
            app.click("tag-confirm-cancel");
            assert!(listed(&app.tree(), "staging"));
        },
    );

    step("confirmed, the rename reaches every note", || {
        app.click("tag-apply");
        app.click("tag-confirm-apply");
        app.eventually("recette replaces staging", |tree| {
            listed(tree, "recette") && !listed(tree, "staging")
        });
    });

    step("onto an existing tag it merges", || {
        tick(&mut app, "preprod");
        rename(&mut app, "recette");
        app.eventually("preprod is gone into recette", |tree| {
            listed(tree, "recette") && !listed(tree, "preprod")
        });
    });

    step("a change of case alone is written", || {
        tick(&mut app, "recette");
        rename(&mut app, "Recette");
        app.eventually("the new spelling", |tree| listed(tree, "Recette"));
    });

    step("a deleted tag leaves its notes", || {
        tick(&mut app, "urgent");
        app.click("tag-delete");
        app.click("tag-confirm-apply");
        app.eventually("urgent is gone", |tree| !listed(tree, "urgent"));
        app.click("tag-manager-close");
        app.eventually("every note stays", |tree| {
            tree.count("note-card") == CARDS && !tree.has("tag-manager-close")
        });
    });

    app.quit();
}
