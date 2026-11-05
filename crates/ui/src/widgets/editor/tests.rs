use egui::{Event, ImeEvent, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::*;

struct Screen {
    editor: Editor,
    changed: Vec<usize>,
    read_only: bool,
    wrap: bool,
}

fn screen(text: &str) -> Harness<'static, Screen> {
    let state = Screen {
        editor: Editor::new(text),
        changed: Vec::new(),
        read_only: false,
        wrap: false,
    };
    crate::testing::harness()
        .with_size(egui::vec2(600.0, 300.0))
        .build_ui_state(
            |ui, screen: &mut Screen| {
                let output = EditorView::new(&mut screen.editor, "Code", "code")
                    .indent_unit("    ")
                    .comment(Some("#"))
                    .read_only(screen.read_only)
                    .wrap(screen.wrap)
                    .show(ui);
                screen.changed.extend(output.changed);
            },
            state,
        )
}

fn focus(harness: &mut Harness<'_, Screen>) {
    harness
        .get_by(|node| node.author_id() == Some("code"))
        .click();
    harness.run();
}

fn text(harness: &Harness<'_, Screen>) -> String {
    harness.state().editor.text()
}

fn press(harness: &mut Harness<'_, Screen>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

fn caret(harness: &Harness<'_, Screen>) -> usize {
    harness.state().editor.buffer.selection().head
}

#[test]
fn typing_goes_into_the_text_and_says_from_which_line() {
    let mut harness = screen("a\nb");
    focus(&mut harness);
    harness.state_mut().editor.buffer.move_to(3, false);
    harness
        .get_by(|node| node.author_id() == Some("code"))
        .type_text("c");
    harness.run();

    assert_eq!(text(&harness), "a\nbc");
    assert_eq!(harness.state().changed, [1]);
    let value = harness
        .get_by(|node| node.author_id() == Some("code"))
        .accesskit_node()
        .value()
        .unwrap_or_default();
    assert_eq!(value, "a\nbc");
}

#[test]
fn tab_stays_in_the_text_and_shift_tab_takes_it_back() {
    let mut harness = screen("x");
    focus(&mut harness);
    harness.state_mut().editor.buffer.move_to(0, false);
    press(&mut harness, Modifiers::NONE, Key::Tab);
    assert_eq!(text(&harness), "    x");
    assert!(
        harness
            .get_by(|node| node.author_id() == Some("code"))
            .is_focused(),
        "the focus did not move on"
    );
    press(&mut harness, Modifiers::SHIFT, Key::Tab);
    assert_eq!(text(&harness), "x");
}

#[test]
fn enter_keeps_the_indentation_of_the_line() {
    let mut harness = screen("  if x:");
    focus(&mut harness);
    harness.state_mut().editor.buffer.move_to(7, false);
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(text(&harness), "  if x:\n  ");
}

#[test]
fn shift_arrows_select_and_backspace_takes_the_selection() {
    let mut harness = screen("hello world");
    focus(&mut harness);
    harness.state_mut().editor.buffer.move_to(11, false);
    press(
        &mut harness,
        Modifiers::SHIFT | Modifiers::COMMAND,
        Key::ArrowLeft,
    );
    assert_eq!(harness.state().editor.buffer.selected_text(), "world");
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(text(&harness), "hello ");
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(text(&harness), "hello world");
}

#[test]
fn up_and_down_keep_the_column_across_a_short_line() {
    let mut harness = screen("abcdef\nab\nabcdef");
    focus(&mut harness);
    harness.state_mut().editor.buffer.move_to(5, false);
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(caret(&harness), 9, "the short line's end");
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(caret(&harness), 15, "back to column 5");
    press(&mut harness, Modifiers::NONE, Key::Home);
    assert_eq!(caret(&harness), 10);
    press(&mut harness, Modifiers::NONE, Key::End);
    assert_eq!(caret(&harness), 16);
    press(&mut harness, Modifiers::COMMAND, Key::Home);
    assert_eq!(caret(&harness), 0);
}

#[test]
fn alt_arrows_move_the_line_and_ctrl_slash_comments_it() {
    let mut harness = screen("a\nb");
    focus(&mut harness);
    harness.state_mut().editor.buffer.move_to(2, false);
    press(&mut harness, Modifiers::ALT, Key::ArrowUp);
    assert_eq!(text(&harness), "b\na");
    press(&mut harness, Modifiers::COMMAND, Key::Slash);
    assert_eq!(text(&harness), "# b\na");
}

#[test]
fn a_composition_shows_until_it_commits_and_only_the_commit_is_typed() {
    let mut harness = screen("");
    focus(&mut harness);
    harness.event(Event::Ime(ImeEvent::Preedit {
        text: "e".into(),
        active_range_chars: None,
    }));
    harness.run();
    assert_eq!(text(&harness), "");
    assert_eq!(harness.state().editor.preedit, "e");
    harness.event(Event::Ime(ImeEvent::Commit("ê".into())));
    harness.run();
    assert_eq!(text(&harness), "ê");
    assert!(harness.state().editor.preedit.is_empty());
}

#[test]
fn a_focused_editor_asks_for_the_input_method_at_its_caret() {
    let mut harness = screen("abc");
    focus(&mut harness);
    let ime = harness.output().platform_output.ime;
    assert!(ime.is_some());
}

#[test]
fn a_read_only_text_moves_and_selects_but_takes_no_typing() {
    let mut harness = screen("abc");
    harness.state_mut().read_only = true;
    focus(&mut harness);
    harness
        .get_by(|node| node.author_id() == Some("code"))
        .type_text("x");
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    press(&mut harness, Modifiers::COMMAND, Key::A);
    assert_eq!(text(&harness), "abc");
    assert_eq!(harness.state().editor.buffer.selected_text(), "abc");
}

#[test]
fn the_context_menu_selects_all() {
    let mut harness = screen("one two");
    harness
        .get_by(|node| node.author_id() == Some("code"))
        .click_secondary();
    harness.run();
    harness
        .get_by(|node| node.author_id() == Some("editor-menu-select-all"))
        .click();
    harness.run();
    assert_eq!(harness.state().editor.buffer.selected_text(), "one two");
}

#[test]
fn a_paste_and_a_cut_go_through_the_text() {
    let mut harness = screen("abc");
    focus(&mut harness);
    harness
        .state_mut()
        .editor
        .buffer
        .select(text::Selection { anchor: 0, head: 1 });
    harness.event(Event::Cut);
    harness.run();
    assert_eq!(text(&harness), "bc");
    harness.event(Event::Paste("a".into()));
    harness.run();
    assert_eq!(text(&harness), "abc");
}

#[test]
fn a_long_text_lays_out_only_what_is_on_screen() {
    let lines: Vec<String> = (0..20_000).map(|n| format!("line {n} = {n};")).collect();
    let mut harness = screen(&lines.join("\n"));
    focus(&mut harness);
    press(&mut harness, Modifiers::COMMAND, Key::End);
    harness
        .get_by(|node| node.author_id() == Some("code"))
        .type_text("!");
    harness.run();
    assert!(text(&harness).ends_with("line 19999 = 19999;!"));
    assert_eq!(harness.state().changed, [19_999]);
}

#[test]
fn wrapped_rows_hang_under_their_line_s_indent() {
    let long = format!("    {}", "word ".repeat(60));
    let mut harness = screen(&long);
    harness.state_mut().wrap = true;
    harness.run();
    let ctx = harness.ctx.clone();
    let frame = geometry::Frame {
        font: FontId::new(13.0, FontFamily::Monospace),
        row_height: 16.0,
        char_width: 8.0,
        wrap: Some(300.0),
        text_left: 0.0,
        top: 0.0,
    };
    let mut galley = None;
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
        galley = Some(geometry::galley(
            ui,
            &harness.state().editor.buffer,
            0,
            &frame,
            None,
        ));
    });
    let galley = galley.unwrap();
    assert!(galley.rows.len() > 1);
    let first = galley.rows[0].pos.x;
    assert!((galley.rows[1].pos.x - first - 6.0 * 8.0).abs() < 0.5);
}

/// The budgets of § 6.2, measured by hand: `cargo test -p ui --release budgets -- --ignored
/// --nocapture`. A frame is egui's pass, without painting.
#[test]
#[ignore = "a measure, not a check"]
fn budgets() {
    let source: String = (0..)
        .map(|n| format!("export function f{n}(a: number, b: string) {{ return a + b.length; }}\n"))
        .take(14_000)
        .collect();
    for size in [100 * 1024, 1024 * 1024] {
        let mut text = source.repeat(size / source.len() + 1);
        text.truncate(size);
        let opened = std::time::Instant::now();
        let mut harness = screen(&text);
        harness.run();
        println!("{size} bytes: opened in {:?}", opened.elapsed());
        focus(&mut harness);
        harness
            .state_mut()
            .editor
            .buffer
            .move_to(text.len() / 2, false);
        harness.run();
        let mut frames = Vec::new();
        for _ in 0..50 {
            harness.event(Event::Text("x".into()));
            let started = std::time::Instant::now();
            harness.step();
            frames.push(started.elapsed());
        }
        frames.sort();
        println!(
            "{size} bytes: typing p50 {:?}, p95 {:?}",
            frames[25], frames[47]
        );
    }
}
