use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::checklist::ChecklistItem;
use engine::notes::kind::NoteKind;
use engine::notes::model::{NoteDraft, NotePatch};
use engine::notes::priority::Priority;

use crate::backend::Backend;
use crate::notes::canvas::tests::{Bench, canvas};
use crate::testing;

/// A note written through the core, its draft shaped by `shape` from a snippet's.
fn write(backend: &Backend, shape: impl FnOnce(&mut NoteDraft) + Send + 'static) -> String {
    let id = testing::note(backend, "seed").id.clone();
    let mut draft = testing::snippet(String::new(), String::new());
    shape(&mut draft);
    let patch = NotePatch {
        title: Some(draft.title),
        content: Some(draft.content),
        kind: Some(draft.kind),
        tags: Some(draft.tags),
        items: Some(draft.items),
        pinned: Some(draft.pinned),
        ..NotePatch::default()
    };
    let at = id.clone();
    let mut written = backend.db(move |engine| engine::notes::update_note(engine, at, patch));
    backend.settle();
    written.poll().unwrap().unwrap();
    id
}

fn node<'a>(harness: &'a Harness<'_, Bench>, author_id: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by(move |node| node.author_id() == Some(author_id))
}

fn label(harness: &Harness<'_, Bench>, author_id: &str) -> String {
    let node = node(harness, author_id);
    let node = node.accesskit_node();
    // egui names a label by its value, every other role by its label.
    node.label().or_else(|| node.value()).unwrap_or_default()
}

#[test]
fn a_snippet_shows_its_first_lines_its_language_and_its_marks() {
    let (backend, profile) = testing::unlocked();
    write(&backend, |draft| {
        draft.title = "psql".into();
        draft.content = "SELECT {{table}};\n-- second line".into();
        draft.tags = vec!["db".into(), "prod".into(), "ops".into()];
        draft.pinned = true;
    });

    let harness = canvas(backend, profile);

    assert_eq!(label(&harness, "note-card-title"), "psql");
    assert_eq!(label(&harness, "kind-badge"), "SQL");
    assert!(label(&harness, "note-card-snippet").starts_with("SELECT {{table}};"));
    assert_eq!(label(&harness, "note-card-pin"), "Pinned note");
    assert_eq!(label(&harness, "note-card-fields"), "1 field to fill");
    assert!(label(&harness, "note-card-when").starts_with("modified"));
    assert_eq!(label(&harness, "note-card-tags"), "#db #ops");
    assert_eq!(label(&harness, "note-card-tags-more"), "1 more tag");
}

#[test]
fn a_long_title_is_cut_on_its_second_line() {
    let (backend, profile) = testing::unlocked();
    write(&backend, |draft| draft.title = "word ".repeat(40));

    let harness = canvas(backend, profile);

    let title = node(&harness, "note-card-title");
    assert!(title.rect().height() < 40.0, "{:?}", title.rect());
}

#[test]
fn a_note_shows_its_outline() {
    let (backend, profile) = testing::unlocked();
    write(&backend, |draft| {
        draft.kind = NoteKind::Note;
        draft.content = "# Deploy\n\n- build\n- [x] ship".into();
    });

    let harness = canvas(backend, profile);

    assert_eq!(
        label(&harness, "note-card-outline"),
        "Deploy\n• build\n☑ ship"
    );
    assert_eq!(label(&harness, "kind-badge"), "Note");
}

#[test]
fn an_item_ticked_on_its_card_is_written_and_shown_done() {
    let (backend, profile) = testing::unlocked();
    let items = ["milk", "bread", "eggs"].map(|text| ChecklistItem {
        text: text.into(),
        done: false,
    });
    let id = write(&backend, move |draft| {
        draft.kind = NoteKind::Checklist;
        draft.items = items.to_vec();
    });
    let mut harness = canvas(backend, profile);
    assert_eq!(label(&harness, "note-card-progress"), "0/3");
    assert_eq!(label(&harness, "note-card-more"), "+1 more");

    harness
        .get_all_by(|node| node.author_id() == Some("note-card-item"))
        .next()
        .unwrap()
        .click();
    testing::settle(&mut harness, |bench| &bench.backend);

    assert_eq!(label(&harness, "note-card-progress"), "1/3");
    let note = engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::get_note(engine, id)
    })
    .unwrap();
    assert!(note.items[0].done);
}

#[test]
fn a_sheet_shows_its_last_results() {
    let (backend, profile) = testing::unlocked();
    write(&backend, |draft| {
        draft.kind = NoteKind::Calc;
        draft.content = "loyer = 800 €\ncharges = 120,5 €\ntotal".into();
    });

    let harness = canvas(backend, profile);

    assert!(label(&harness, "note-card-calc").contains("total"));
    assert_eq!(label(&harness, "kind-badge"), "Calc");
}

#[test]
fn a_drawing_without_a_picture_says_so() {
    let (backend, profile) = testing::unlocked();
    write(&backend, |draft| {
        draft.kind = NoteKind::Drawing;
        draft.content = String::new();
    });

    let harness = canvas(backend, profile);

    assert_eq!(label(&harness, "note-card-drawing-empty"), "No preview yet");
}

#[test]
fn a_priority_is_a_pill_among_the_marks() {
    let (backend, profile) = testing::unlocked();
    let id = write(&backend, |draft| draft.title = "urgent fix".into());
    let mut set =
        backend.db(move |engine| engine::notes::set_priority(engine, vec![id], Priority::Urgent));
    backend.settle();
    set.poll().unwrap().unwrap();

    let harness = canvas(backend, profile);

    assert_eq!(label(&harness, "priority-pill"), "Urgent");
}

#[test]
fn a_drawing_shows_its_picture_once_decoded() {
    use base64::Engine as _;
    let (backend, profile) = testing::unlocked();
    let id = write(&backend, |draft| {
        draft.kind = NoteKind::Drawing;
        draft.content = String::new();
    });
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 4, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[200; 4 * 2 * 4]).unwrap();
    }
    let uri = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    );
    let patch = NotePatch {
        pictures: Some(engine::notes::drawing::DrawingPictures {
            thumbnail: uri.clone(),
            picture: uri,
        }),
        ..NotePatch::default()
    };
    let mut written = backend.db(move |engine| engine::notes::update_note(engine, id, patch));
    backend.settle();
    written.poll().unwrap().unwrap();

    let harness = canvas(backend, profile);

    node(&harness, "note-card-drawing");
}
