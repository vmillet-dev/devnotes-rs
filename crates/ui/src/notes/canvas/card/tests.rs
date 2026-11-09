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
    assert_eq!(label(&harness, "kind-icon"), "Snippet");
    assert_eq!(label(&harness, "note-format"), "SQL");
    assert!(label(&harness, "note-card-snippet").starts_with("SELECT {{table}};"));
    assert_eq!(label(&harness, "note-card-pin"), "Pinned note");
    assert_eq!(label(&harness, "note-card-fields"), "1 field to fill");
    assert!(label(&harness, "note-card-when").starts_with("modified"));
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("note-card-tags"))
            .is_none(),
        "a card shows no tag"
    );
}

/// A title bar, the preview, a status line: stacked, none over another. The icon, a snippet's
/// format and the title share the first; the fields and the date the last.
#[test]
fn a_card_reads_in_three_bands_top_to_bottom() {
    let (backend, profile) = testing::unlocked();
    write(&backend, |draft| {
        draft.title = "psql".into();
        draft.content = "a {{host}}\nb\nc\nd\ne\nf".into();
    });

    let harness = canvas(backend, profile);

    let card = node(&harness, "note-card").rect();
    let rect = |author_id| node(&harness, author_id).rect();
    let (icon, title) = (rect("kind-icon"), rect("note-card-title"));
    let (preview, when) = (rect("note-card-snippet"), rect("note-card-when"));
    assert!(
        (icon.center().y - title.center().y).abs() < 1.0,
        "{icon:?} {title:?}"
    );
    let format = rect("note-format");
    assert!(icon.right() < format.left() && format.right() < title.left());
    assert!(
        (format.center().y - title.center().y).abs() < 1.0,
        "{format:?}"
    );
    assert!(title.bottom() <= preview.top() && preview.bottom() <= when.top());
    let fields = rect("note-card-fields");
    assert!(
        (fields.center().y - when.center().y).abs() < 1.0,
        "{fields:?}"
    );
    assert!(fields.right() <= when.left());
    assert!(card.contains_rect(icon) && card.contains_rect(when));
}

/// A snippet names its format; a Note has none, its icon says what it is.
#[test]
fn only_a_snippet_has_a_format() {
    let (backend, profile) = testing::unlocked();
    write(&backend, |draft| {
        draft.kind = NoteKind::Note;
        draft.title = "plan".into();
    });

    let harness = canvas(backend, profile);

    assert_eq!(label(&harness, "kind-icon"), "Note");
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("note-format"))
            .is_none()
    );
}

/// Never « Untitled » on the card: the icon stands alone in its title bar.
#[test]
fn an_untitled_card_writes_no_title() {
    let (backend, profile) = testing::unlocked();
    write(&backend, |draft| {
        draft.content = "1\n2\n3".into();
    });

    let harness = canvas(backend, profile);

    assert!(
        harness
            .query_by(|node| node.author_id() == Some("note-card-title"))
            .is_none()
    );
    assert_eq!(label(&harness, "note-card-open"), "Untitled");
}

/// 3, 4 or 5 lines: the card is 120, 140 or 160 px, the library's choice.
#[test]
fn the_card_follows_the_librarys_preview_lines() {
    use engine::folders::board::PreviewLines;
    let (backend, profile) = testing::unlocked();
    write(&backend, |draft| {
        draft.title = "long".into();
        draft.content = "1\n2\n3\n4\n5\n6\n7".into();
    });
    let mut harness = canvas(backend, profile);

    for (lines, height) in [
        (PreviewLines::Three, 120.0),
        (PreviewLines::Four, 140.0),
        (PreviewLines::Five, 160.0),
    ] {
        harness.state_mut().state.preview_lines = lines;
        harness.run();
        let card = node(&harness, "note-card").rect().height();
        assert!((card - height).abs() < 0.5, "{lines}: {card}");
        let shown = node(&harness, "note-card-snippet").rect().height();
        #[expect(clippy::cast_precision_loss, reason = "3 to 5 lines")]
        let room = lines.count() as f32 * 16.0;
        assert!(shown >= room, "{lines}: {shown} for {room}");
    }
}

#[test]
fn a_long_title_stays_on_its_one_line() {
    let (backend, profile) = testing::unlocked();
    write(&backend, |draft| draft.title = "word ".repeat(40));

    let harness = canvas(backend, profile);

    let title = node(&harness, "note-card-title");
    assert!(title.rect().height() <= 18.0, "{:?}", title.rect());
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
    assert_eq!(label(&harness, "kind-icon"), "Note");
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
        draft.title = "shopping".into();
        draft.items = items.to_vec();
    });
    let mut harness = canvas(backend, profile);
    assert_eq!(label(&harness, "note-card-progress"), "0/3");
    // Four lines, a task fewer: the three of them.
    let shown = harness.query_all_by(|node| node.author_id() == Some("note-card-item"));
    assert_eq!(shown.count(), 3);

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
    assert_eq!(label(&harness, "kind-icon"), "Calc");
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
fn a_priority_is_its_bars_in_the_top_band() {
    let (backend, profile) = testing::unlocked();
    let id = write(&backend, |draft| draft.title = "urgent fix".into());
    let mut set =
        backend.db(move |engine| engine::notes::set_priority(engine, vec![id], Priority::Urgent));
    backend.settle();
    set.poll().unwrap().unwrap();

    let harness = canvas(backend, profile);

    assert_eq!(label(&harness, "priority-bars"), "Urgent");
    let bars = node(&harness, "priority-bars").rect();
    assert!(
        bars.width() > 20.0,
        "« Urgent » is written beside its bars: {bars:?}"
    );
    let title = node(&harness, "note-card-title").rect();
    assert!((bars.center().y - title.center().y).abs() < 1.0);
    assert!(title.right() <= bars.left());
}

/// While a card is ticked its box ends the title bar; the pin moves over for it.
#[test]
fn a_ticked_cards_box_ends_its_title_bar() {
    let (backend, profile) = testing::unlocked();
    let id = write(&backend, |draft| {
        draft.title = "pick me".into();
        draft.pinned = true;
    });
    let mut harness = canvas(backend, profile);
    let icon = node(&harness, "kind-icon").rect();
    let pin = node(&harness, "note-card-pin").rect();

    harness.state_mut().state.selection.toggle(&id);
    harness.run();

    let card = node(&harness, "note-card").rect();
    let tick = node(&harness, "note-card-check").rect();
    assert!(card.right() - tick.right() < 4.0, "{tick:?} in {card:?}");
    assert!((tick.center().y - icon.center().y).abs() < 1.0);
    let moved = node(&harness, "note-card-pin").rect();
    assert!(moved.right() <= tick.left() && moved.left() < pin.left());
    assert_eq!(node(&harness, "kind-icon").rect(), icon);
}

/// A first Delete turns the bottom band into its two keys.
#[test]
fn an_armed_cards_bottom_band_says_both_keys() {
    let (backend, profile) = testing::unlocked();
    let id = write(&backend, |draft| draft.title = "doomed".into());
    let mut harness = canvas(backend, profile);

    harness.state_mut().state.selection.arm(&id);
    harness.run();

    assert_eq!(
        label(&harness, "note-card-arming"),
        "Del again · Esc to cancel"
    );
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("note-card-when"))
            .is_none()
    );
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
