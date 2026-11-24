use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::kind::NoteKind;
use engine::notes::model::{DisplayNote, NotePatch};

use super::{css, file_name};
use crate::notes::bench::{Bench, click_id, expanded_area, settle};
use crate::testing;

type Harness = egui_kittest::Harness<'static, Bench>;

/// A drawing holding `content`, open in the editor.
fn opened(content: &str) -> (Harness, String) {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "sketch").id.clone();
    let patch = NotePatch {
        kind: Some(NoteKind::Drawing),
        content: Some(content.to_owned()),
        ..NotePatch::default()
    };
    let at = id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::update_note(engine, &at, &patch)
    })
    .unwrap();
    let mut harness = expanded_area(backend, profile);
    harness.state_mut().state.editor.open(id.clone());
    settle(&mut harness);
    (harness, id)
}

fn stored(harness: &Harness, id: &str) -> DisplayNote {
    let id = id.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::get_note(engine, &id)
    })
    .unwrap()
}

fn thumbnail(harness: &Harness, id: &str) -> Option<String> {
    let id = id.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::drawing_thumbnail(engine, &id)
    })
    .unwrap()
}

fn stage(harness: &Harness) -> Rect {
    let node = harness.get_by(|node| node.author_id() == Some("drawing-stage"));
    let bounds = node.rect();
    Rect::from_min_max(bounds.min, bounds.max)
}

fn shapes(harness: &Harness) -> String {
    harness
        .get_by(|node| node.author_id() == Some("drawing-stage"))
        .value()
        .unwrap_or_default()
}

/// Inside the stage, from its corner.
fn point(harness: &Harness, x: f32, y: f32) -> Pos2 {
    stage(harness).min + egui::vec2(x, y)
}

fn drag(harness: &mut Harness, from: Pos2, to: Pos2) {
    harness.event(Event::PointerMoved(from));
    harness.run_steps(1);
    pointer(harness, from, true);
    harness.run_steps(1);
    harness.event(Event::PointerMoved(to));
    harness.run_steps(1);
    pointer(harness, to, false);
    harness.run_steps(2);
}

fn pointer(harness: &mut Harness, pos: Pos2, pressed: bool) {
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
}

fn press(harness: &mut Harness, key: Key, modifiers: Modifiers) {
    harness.key_press_modifiers(modifiers, key);
    harness.run_steps(2);
}

fn draw_rectangle(harness: &mut Harness) {
    click_id(harness, "drawing-tool-rectangle");
    let (from, to) = (point(harness, 40.0, 40.0), point(harness, 140.0, 100.0));
    drag(harness, from, to);
}

fn close(harness: &mut Harness) {
    click_id(harness, "editor-close");
    settle(harness);
}

#[test]
fn a_tool_draws_on_the_stage_and_the_close_writes_it_with_its_pictures() {
    let (mut harness, id) = opened("");

    draw_rectangle(&mut harness);
    assert_eq!(shapes(&harness), "1");
    close(&mut harness);

    let note = stored(&harness, &id);
    assert!(
        note.content.contains(r#""type":"Rect""#),
        "{}",
        note.content
    );
    assert!(note.content.contains(r#""hue":"ink""#));
    assert!(thumbnail(&harness, &id).is_some());
}

#[test]
fn the_card_shows_the_picture_taken_after_the_body_was_written() {
    let (mut harness, _) = opened("");

    draw_rectangle(&mut harness);
    close(&mut harness);

    harness.get_by(|node| node.author_id() == Some("note-card-drawing"));
}

#[test]
fn a_drawing_is_not_copied_as_text() {
    let (mut harness, id) = opened("");
    draw_rectangle(&mut harness);
    close(&mut harness);

    // The menu and the palette reach it; a card shows no copy button for a drawing.
    let bench = harness.state_mut();
    bench.state.notes.copy(&bench.backend, id);
    let mut copied = false;
    for _ in 0..60 {
        harness.step();
        harness.state().backend.settle();
        let commands = &harness.output().platform_output.commands;
        copied |= commands
            .iter()
            .any(|command| matches!(command, egui::OutputCommand::CopyText(_)));
    }

    assert!(!copied);
    let (_, said) = harness.state().state.status.latest().unwrap();
    assert_eq!(said.key, i18n::keys::notes::DRAWING_NOT_COPIED);
}

#[test]
fn opened_and_closed_nothing_moves() {
    let scene = r#"{"version":1,"scene":{"objects":[]}}"#;
    let (mut harness, id) = opened(scene);
    let before = stored(&harness, &id).updated_at;

    close(&mut harness);

    let note = stored(&harness, &id);
    assert_eq!(note.content, scene);
    assert_eq!(note.updated_at, before);
    assert_eq!(thumbnail(&harness, &id), None);
}

#[test]
fn a_drawing_with_no_picture_takes_them_on_being_left_and_no_date_moves() {
    let scene = r#"{"version":1,"scene":{"objects":[{"type":"Rect","left":60,"top":45,"width":100,"height":50}]}}"#;
    let (mut harness, id) = opened(scene);
    let before = stored(&harness, &id).updated_at;

    close(&mut harness);

    assert!(thumbnail(&harness, &id).is_some());
    assert_eq!(stored(&harness, &id).updated_at, before);
    assert_eq!(stored(&harness, &id).content, scene);
}

#[test]
fn the_tools_answer_to_their_letters_and_escape_drops_tool_then_selection_then_steps_back() {
    let (mut harness, _) = opened("");
    draw_rectangle(&mut harness);

    press(&mut harness, Key::A, Modifiers::NONE);
    assert!(pressed(&harness, "drawing-tool-arrow"));

    press(&mut harness, Key::Escape, Modifiers::NONE);
    assert!(pressed(&harness, "drawing-tool-select"));
    assert!(is_open(&harness));

    press(&mut harness, Key::Escape, Modifiers::NONE);
    assert!(is_open(&harness));

    // Expanded, the editor goes back beside the cards first, then closes.
    press(&mut harness, Key::Escape, Modifiers::NONE);
    settle(&mut harness);
    assert!(is_open(&harness) && !harness.state().state.settings.editor_fullscreen);
    press(&mut harness, Key::Escape, Modifiers::NONE);
    settle(&mut harness);
    assert!(!is_open(&harness));
}

fn pressed(harness: &Harness, id: &str) -> bool {
    harness
        .get_by(|node| node.author_id() == Some(id))
        .accesskit_node()
        .toggled()
        == Some(egui::accesskit::Toggled::True)
}

fn is_open(harness: &Harness) -> bool {
    harness
        .query_by(|node| node.author_id() == Some("editor-title"))
        .is_some()
}

/// A window without the system's focus still answers the keys it is sent: a test's has none.
#[test]
fn the_stage_keeps_its_keys_in_a_window_without_the_focus() {
    let (mut harness, _) = opened("");
    draw_rectangle(&mut harness);
    harness.input_mut().focused = false;
    harness.run_steps(2);

    press(&mut harness, Key::A, Modifiers::NONE);

    assert!(pressed(&harness, "drawing-tool-arrow"));
}

#[test]
fn undo_and_redo_by_their_keys_and_buttons_and_delete_removes_the_selection() {
    let (mut harness, _) = opened("");
    draw_rectangle(&mut harness);

    press(&mut harness, Key::Z, Modifiers::COMMAND);
    assert_eq!(shapes(&harness), "0");
    press(&mut harness, Key::Y, Modifiers::COMMAND);
    assert_eq!(shapes(&harness), "1");

    click_id(&mut harness, "drawing-undo");
    assert_eq!(shapes(&harness), "0");
    click_id(&mut harness, "drawing-redo");
    assert_eq!(shapes(&harness), "1");

    let inside = point(&harness, 90.0, 70.0);
    drag(&mut harness, inside, inside);
    press(&mut harness, Key::Delete, Modifiers::NONE);
    assert_eq!(shapes(&harness), "0");
}

#[test]
fn a_hue_paints_what_is_selected() {
    let (mut harness, id) = opened("");
    draw_rectangle(&mut harness);

    click_id(&mut harness, "drawing-hue-red");
    close(&mut harness);

    assert!(stored(&harness, &id).content.contains(r#""hue":"red""#));
}

#[test]
fn a_text_is_typed_where_clicked_and_stored() {
    let (mut harness, id) = opened("");
    click_id(&mut harness, "drawing-tool-text");
    let at = point(&harness, 60.0, 60.0);
    drag(&mut harness, at, at);

    harness
        .get_by(|node| node.author_id() == Some("drawing-text"))
        .type_text("API");
    harness.run_steps(3);
    close(&mut harness);

    assert!(stored(&harness, &id).content.contains(r#""text":"API""#));
}

#[test]
fn a_new_drawing_left_empty_is_never_written() {
    let (backend, profile) = testing::unlocked();
    let space = testing::note(&backend, "seed").space_id.clone();
    let mut harness = expanded_area(backend, profile);
    let state = &mut harness.state_mut().state;
    state.editor.draft(NoteKind::Drawing, space, None);
    settle(&mut harness);
    let before = cards(&harness);

    close(&mut harness);

    assert_eq!(cards(&harness), before);
}

fn cards(harness: &Harness) -> usize {
    harness
        .state()
        .state
        .notes_query
        .view()
        .unwrap()
        .cards()
        .count()
}

#[test]
fn the_footer_counts_the_shapes() {
    let (mut harness, _) = opened("");
    draw_rectangle(&mut harness);

    let stats = harness.get_by(|node| node.author_id() == Some("editor-stats"));
    assert!(stats.value().unwrap_or_default().contains('1'));
}

/// Answered by a file in a folder of its own, kept alive with the test.
fn exporting_to(harness: &mut Harness, name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join(name);
    harness.state_mut().state.dialog = std::sync::Arc::new(platform::dialog::Fixed(path.clone()));
    (folder, path)
}

fn said(harness: &Harness) -> String {
    let state = &harness.state().state;
    let (_, message) = state.status.latest().expect("the export reports");
    message.text(state.language)
}

/// Expanded, the exports wait in the toolbar's menu.
fn export_as(harness: &mut Harness, author_id: &str) {
    click_id(harness, "editor-export");
    click_id(harness, author_id);
}

#[test]
fn a_drawing_is_exported_as_a_png_and_says_where() {
    let (mut harness, _) = opened("");
    draw_rectangle(&mut harness);
    let (_folder, path) = exporting_to(&mut harness, "sketch.png");

    export_as(&mut harness, "drawing-export-png");
    settle(&mut harness);

    assert!(std::fs::read(&path).unwrap().starts_with(b"\x89PNG"));
    assert!(said(&harness).contains("sketch.png"), "{}", said(&harness));
}

#[test]
fn a_drawing_is_exported_as_its_svg() {
    let (mut harness, _) = opened("");
    draw_rectangle(&mut harness);
    let (_folder, path) = exporting_to(&mut harness, "sketch.svg");

    export_as(&mut harness, "drawing-export-svg");
    settle(&mut harness);

    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.starts_with("<svg"), "{written}");
    assert!(written.contains("<rect"));
}

struct Cancelled;

impl platform::dialog::FileDialog for Cancelled {
    fn open(&self) -> Option<std::path::PathBuf> {
        None
    }

    fn save(&self, _name: &str) -> Option<std::path::PathBuf> {
        None
    }
}

#[test]
fn a_dialog_cancelled_writes_nothing_and_says_nothing() {
    let (mut harness, _) = opened("");
    draw_rectangle(&mut harness);
    harness.state_mut().state.dialog = std::sync::Arc::new(Cancelled);

    export_as(&mut harness, "drawing-export-png");
    settle(&mut harness);

    assert!(harness.state().state.status.latest().is_none());
    assert!(harness.state().state.errors.notice().is_none());
}

#[test]
fn a_file_that_cannot_be_written_says_so() {
    let (mut harness, _) = opened("");
    draw_rectangle(&mut harness);
    let (folder, _) = exporting_to(&mut harness, "unused.png");
    let folder_itself = folder.path().to_path_buf();
    harness.state_mut().state.dialog = std::sync::Arc::new(platform::dialog::Fixed(folder_itself));

    export_as(&mut harness, "drawing-export-png");
    settle(&mut harness);

    assert!(harness.state().state.errors.notice().is_some());
    assert!(harness.state().state.status.latest().is_none());
}

#[test]
fn nothing_drawn_has_nothing_to_export() {
    let (mut harness, _) = opened("");
    let (_folder, path) = exporting_to(&mut harness, "sketch.png");

    click_id(&mut harness, "editor-export");
    settle(&mut harness);

    let offered = harness.query_by(|node| node.author_id() == Some("drawing-export-png"));
    assert!(offered.is_none(), "the menu stays shut");
    assert!(!path.exists());
}

#[test]
fn a_file_is_named_after_the_title_with_what_no_system_refuses() {
    use i18n::Language;

    assert_eq!(
        file_name("Archi: v2/final?", Language::En),
        "Archi- v2-final-"
    );
    assert_eq!(file_name("  sketch\u{7} ", Language::En), "sketch");
    assert_eq!(file_name("   ", Language::En), "Drawing");
    assert_eq!(file_name("", Language::Fr), "Dessin");
}

#[test]
fn a_colour_is_written_for_a_picture_as_css() {
    assert_eq!(css(egui::Color32::from_rgb(1, 2, 255)), "#0102ff");
    let soft = egui::Color32::from_rgba_unmultiplied(10, 20, 30, 51);
    assert!(css(soft).starts_with("rgba("), "{}", css(soft));
    assert!(css(soft).ends_with("0.20)"), "{}", css(soft));
}

fn zoom(harness: &Harness) -> String {
    harness
        .get_by(|node| node.author_id() == Some("drawing-zoom"))
        .accesskit_node()
        .label()
        .unwrap_or_default()
}

fn wheel(harness: &mut Harness, at: Pos2, delta: egui::Vec2, modifiers: Modifiers) {
    harness.event(Event::PointerMoved(at));
    harness.run_steps(1);
    harness.event_modifiers(
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers,
        },
        modifiers,
    );
    settle(harness);
}

/// Where the one rectangle of the scene stands.
fn rectangle(harness: &mut Harness, id: &str) -> drawing::Box {
    harness.run_steps(8);
    settle(harness);
    let content = stored(harness, id).content.clone();
    drawing::Drawing::read(&content).unwrap().bounds().unwrap()
}

#[test]
fn the_wheel_moves_the_look_and_with_ctrl_zooms_and_what_is_drawn_lands_under_the_pointer() {
    let (mut harness, _) = opened("");
    assert_eq!(zoom(&harness), "100 %");

    let at = point(&harness, 200.0, 150.0);
    wheel(&mut harness, at, egui::vec2(0.0, 60.0), Modifiers::COMMAND);
    let closer = zoom(&harness);
    assert_ne!(closer, "100 %", "Ctrl and the wheel zoom");
    click_id(&mut harness, "drawing-zoom");
    assert_eq!(zoom(&harness), "100 %");

    let (mut harness, id) = opened("");
    let before = point(&harness, 0.0, 0.0);
    wheel(&mut harness, at, egui::vec2(-30.0, -50.0), Modifiers::NONE);
    assert_eq!(zoom(&harness), "100 %");
    assert_eq!(point(&harness, 0.0, 0.0), before, "the stage stays put");
    click_id(&mut harness, "drawing-tool-rectangle");
    let (from, to) = (point(&harness, 100.0, 100.0), point(&harness, 200.0, 160.0));
    drag(&mut harness, from, to);
    let drawn = rectangle(&mut harness, &id);
    // The scene moved 30 left and 50 up under the pointer, all of it.
    assert!((drawn.left - 130.0).abs() < 3.0, "{drawn:?}");
    assert!((drawn.top - 150.0).abs() < 3.0, "{drawn:?}");
}

#[test]
fn at_another_size_a_shape_is_taken_and_drawn_where_it_shows() {
    let (mut harness, id) = opened("");
    for _ in 0..4 {
        click_id(&mut harness, "drawing-zoom-in");
    }
    assert_eq!(zoom(&harness), "244 %");
    click_id(&mut harness, "drawing-tool-rectangle");
    let (from, to) = (point(&harness, 100.0, 100.0), point(&harness, 344.0, 222.0));
    drag(&mut harness, from, to);
    let drawn = rectangle(&mut harness, &id);
    assert!((drawn.width - 102.0).abs() < 3.0, "{drawn:?}");

    press(&mut harness, Key::Escape, Modifiers::NONE);
    let inside = point(&harness, 222.0, 160.0);
    drag(&mut harness, inside, inside);
    press(&mut harness, Key::Delete, Modifiers::NONE);
    assert_eq!(shapes(&harness), "0");
}

#[test]
fn space_and_a_drag_or_the_middle_button_move_the_look_and_draw_nothing() {
    let (mut harness, _) = opened("");
    draw_rectangle(&mut harness);
    click_id(&mut harness, "drawing-tool-rectangle");
    let held = |pressed| Event::Key {
        key: Key::Space,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    harness.event(held(true));
    let (from, to) = (point(&harness, 300.0, 200.0), point(&harness, 360.0, 240.0));
    drag(&mut harness, from, to);
    harness.event(held(false));
    harness.run_steps(2);
    assert_eq!(shapes(&harness), "1");

    for pressed in [true, false] {
        harness.event(Event::PointerMoved(if pressed { from } else { to }));
        harness.event(Event::PointerButton {
            pos: if pressed { from } else { to },
            button: PointerButton::Middle,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.run_steps(2);
    }
    assert_eq!(shapes(&harness), "1");

    press(&mut harness, Key::Escape, Modifiers::NONE);
    // Moved 60 and 40 twice: the rectangle's corner was at 40, 40.
    let moved = point(&harness, 40.0 + 120.0 + 50.0, 40.0 + 80.0 + 30.0);
    drag(&mut harness, moved, moved);
    press(&mut harness, Key::Delete, Modifiers::NONE);
    assert_eq!(shapes(&harness), "0", "it moved with the look");
}

#[test]
fn a_drawing_larger_than_the_stage_opens_whole_and_f_shows_it_again() {
    let wide = r#"{"version":1,"scene":{"objects":[{"type":"Rect","left":2000,"top":45,"width":4000,"height":50}]}}"#;
    let (mut harness, _) = opened(wide);
    let fitted = zoom(&harness);
    assert_ne!(fitted, "100 %");

    let at = point(&harness, 300.0, 300.0);
    drag(&mut harness, at, at);
    press(&mut harness, Key::Num0, Modifiers::COMMAND);
    assert_eq!(zoom(&harness), "100 %");
    press(&mut harness, Key::F, Modifiers::NONE);
    assert_eq!(zoom(&harness), fitted);
    press(&mut harness, Key::Plus, Modifiers::COMMAND);
    assert_ne!(zoom(&harness), fitted);
    press(&mut harness, Key::Minus, Modifiers::COMMAND);
    press(&mut harness, Key::ArrowRight, Modifiers::COMMAND);
    assert_eq!(zoom(&harness), fitted);
}

#[test]
fn the_eraser_stays_in_hand_after_an_erase_and_escape_puts_it_down() {
    let (mut harness, _) = opened("");
    draw_rectangle(&mut harness);
    click_id(&mut harness, "drawing-tool-rectangle");
    let (from, to) = (point(&harness, 200.0, 40.0), point(&harness, 300.0, 100.0));
    drag(&mut harness, from, to);
    assert_eq!(shapes(&harness), "2");

    press(&mut harness, Key::E, Modifiers::NONE);
    for (x, left) in [(90.0, "1"), (250.0, "0")] {
        let on = point(&harness, x, 70.0);
        drag(&mut harness, on, on);
        assert_eq!(shapes(&harness), left);
        assert!(pressed(&harness, "drawing-tool-eraser"));
    }
    press(&mut harness, Key::Escape, Modifiers::NONE);
    assert!(pressed(&harness, "drawing-tool-select"));
    press(&mut harness, Key::Z, Modifiers::COMMAND);
    assert_eq!(shapes(&harness), "1");
}

/// The toolbar of a panel: nothing over anything else.
#[test]
fn in_a_narrow_panel_the_exports_take_a_line_of_their_own() {
    let (mut harness, _) = opened("");
    harness.state_mut().state.settings.editor_fullscreen = false;
    harness.set_size(egui::vec2(1_220.0, 900.0));
    settle(&mut harness);

    let rect = |id: &str| harness.get_by(|node| node.author_id() == Some(id)).rect();
    let tools = [
        "drawing-tool-select",
        "drawing-hue-purple",
        "drawing-undo",
        "drawing-delete",
    ];
    for export in ["drawing-export-png", "drawing-export-svg"] {
        for tool in tools {
            assert!(!rect(export).intersects(rect(tool)), "{export} over {tool}");
        }
        assert!(rect(export).right() <= 1_220.0);
    }
    assert!(rect("drawing-delete").right() <= 1_220.0);
}
