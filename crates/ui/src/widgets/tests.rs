//! The spinner never rests, so these frames are counted rather than settled.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::*;
use crate::theme::icons::Icon;

/// Every widget of the kit once, in a state a test can change.
#[derive(Default)]
struct Bench {
    clicks: u32,
    armed: bool,
    checked: bool,
    density: u8,
    typed: String,
    searched: String,
}

fn bench() -> Harness<'static, Bench> {
    crate::testing::harness().build_ui_state(
        |ui, bench: &mut Bench| {
            if Button::new("Enregistrer", "save")
                .kind(Kind::Filled)
                .show(ui)
                .clicked()
            {
                bench.clicks += 1;
            }
            Button::new("Supprimer", "delete")
                .kind(Kind::Destructive)
                .armed(bench.armed)
                .show(ui);
            Button::new("Plus tard", "later").enabled(false).show(ui);
            IconButton::new(Icon::Trash, "Corbeille", "trash").show(ui);
            SplitButton::new("Exporter", "Autres formats", "export").show(ui);
            RailToggle::new("Masquer le rail", "rail-toggle", true).show(ui);
            Chip::new("psql", "tag-chip", true).show(ui);
            Checkbox::new(&mut bench.checked, "Fait", "item-done").show(ui);
            Segmented::new(&[(0, "Confortable"), (1, "Compacte")], "density")
                .show(ui, &mut bench.density);
            TextField::new(&mut bench.typed, "Nom", "name-field")
                .hint("Nom de la bibliothèque")
                .show(ui);
            SearchField::new(&mut bench.searched, "Rechercher", "search")
                .key("Ctrl K")
                .show(ui);
            hue_badge(ui, "sql", crate::theme::palette(ui.ctx()).blue);
            priority_dot(ui, engine::notes::priority::Priority::High);
            Kbd("Ctrl").show(ui);
            spinner(ui, 16.0, "Chargement");
        },
        Bench::default(),
    )
}

#[test]
fn every_control_is_named_and_addressed() {
    let mut harness = bench();
    harness.run_steps(4);

    for (author_id, role, name) in [
        ("save", Role::Button, "Enregistrer"),
        ("delete", Role::Button, "Supprimer"),
        ("trash", Role::Button, "Corbeille"),
        ("export", Role::Button, "Exporter"),
        ("export-more", Role::Button, "Autres formats"),
        ("rail-toggle", Role::Button, "Masquer le rail"),
        ("tag-chip", Role::CheckBox, "psql"),
        ("item-done", Role::CheckBox, "Fait"),
        ("density-0", Role::RadioButton, "Confortable"),
        ("name-field", Role::TextInput, "Nom"),
        ("search", Role::TextInput, "Rechercher"),
    ] {
        let node = harness.get_by(|node| node.author_id() == Some(author_id));
        assert_eq!(node.accesskit_node().role(), role, "{author_id}");
        assert_eq!(
            node.accesskit_node().label().as_deref(),
            Some(name),
            "{author_id}"
        );
    }
}

#[test]
fn no_interactive_control_is_under_24_pixels() {
    let mut harness = bench();
    harness.run_steps(4);

    for role in [Role::Button, Role::CheckBox, Role::RadioButton] {
        for node in harness.query_all_by_role(role) {
            let size = node.rect().size();
            assert!(
                size.x >= MIN_TARGET && size.y >= MIN_TARGET,
                "{:?} is {size:?}",
                node.accesskit_node().label()
            );
        }
    }
}

#[test]
fn a_click_acts_and_a_disabled_button_does_not() {
    let mut harness = bench();
    harness.run_steps(4);

    harness
        .get_by(|node| node.author_id() == Some("save"))
        .click();
    harness
        .get_by(|node| node.author_id() == Some("later"))
        .click();
    harness.run_steps(4);

    assert_eq!(harness.state().clicks, 1);
    let later = harness.get_by(|node| node.author_id() == Some("later"));
    assert!(later.accesskit_node().is_disabled());
}

#[test]
fn a_checkbox_and_a_segment_change_their_value() {
    let mut harness = bench();
    harness.run_steps(4);

    harness
        .get_by(|node| node.author_id() == Some("item-done"))
        .click();
    harness
        .get_by(|node| node.author_id() == Some("density-1"))
        .click();
    harness.run_steps(4);

    assert!(harness.state().checked);
    assert_eq!(harness.state().density, 1);
    let on = harness.get_by(|node| node.author_id() == Some("density-1"));
    assert_eq!(
        on.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
}

#[test]
fn a_field_takes_what_is_typed() {
    let mut harness = bench();
    harness.run_steps(4);

    harness
        .get_by(|node| node.author_id() == Some("name-field"))
        .click();
    harness.run_steps(4);
    harness
        .get_by(|node| node.author_id() == Some("name-field"))
        .type_text("Travail");
    harness.run_steps(4);

    assert_eq!(harness.state().typed, "Travail");
}

/// Not an assertion: the bench drawn in both themes, for a look by eye
/// (`DEVNOTES_LOOK=<directory> cargo test -p ui widgets::tests::look`).
#[test]
fn look() {
    let Some(directory) = std::env::var_os("DEVNOTES_LOOK") else {
        return;
    };
    for (name, theme) in [
        ("dark", crate::theme::Appearance::Dark),
        ("light", crate::theme::Appearance::Light),
    ] {
        let mut harness = bench();
        crate::theme::apply(&harness.ctx, theme, crate::theme::Density::Comfortable);
        harness.set_size(egui::vec2(420.0, 620.0));
        harness.run_steps(4);
        let image = harness.render().unwrap();
        image
            .save(std::path::Path::new(&directory).join(format!("widgets-{name}.png")))
            .unwrap();
    }
}
