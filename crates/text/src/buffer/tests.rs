use super::*;

fn typed(buffer: &mut Buffer, text: &str) {
    for c in text.chars() {
        buffer.insert(&c.to_string());
    }
}

fn at(text: &str, caret: usize) -> Buffer {
    let mut buffer = Buffer::new(text);
    buffer.select(Selection::caret(caret));
    buffer
}

#[test]
fn a_word_typed_is_undone_in_one_step_and_the_next_word_in_another() {
    let mut buffer = Buffer::new("");
    typed(&mut buffer, "SELECT 1");

    buffer.undo();
    assert_eq!(buffer.text(), "SELECT");
    buffer.undo();
    assert_eq!(buffer.text(), "");
    buffer.redo();
    assert_eq!(buffer.text(), "SELECT");
    assert_eq!(buffer.selection(), Selection::caret(6));
}

#[test]
fn typing_after_the_caret_moved_starts_a_step_of_its_own() {
    let mut buffer = Buffer::new("");
    typed(&mut buffer, "ab");
    buffer.move_to(0, false);
    typed(&mut buffer, "x");

    buffer.undo();
    assert_eq!(buffer.text(), "ab");
}

#[test]
fn a_new_edit_drops_what_could_be_redone() {
    let mut buffer = Buffer::new("");
    typed(&mut buffer, "a");
    buffer.undo();
    typed(&mut buffer, "b");
    assert!(buffer.redo().is_none());
    assert_eq!(buffer.text(), "b");
}

#[test]
fn backspace_takes_a_whole_grapheme_and_a_crlf_at_once() {
    let mut buffer = at("cafe\u{301}", 5);
    buffer.delete(Direction::Backward, Unit::Grapheme);
    assert_eq!(buffer.text(), "caf");

    let mut buffer = at("a\r\nb", 3);
    buffer.delete(Direction::Backward, Unit::Grapheme);
    assert_eq!(buffer.text(), "ab");
}

#[test]
fn erasing_letter_by_letter_is_undone_in_one_step() {
    let mut buffer = at("hello", 5);
    for _ in 0..3 {
        buffer.delete(Direction::Backward, Unit::Grapheme);
    }
    assert_eq!(buffer.text(), "he");
    buffer.undo();
    assert_eq!(buffer.text(), "hello");
}

#[test]
fn a_word_goes_with_the_spaces_before_it() {
    let mut buffer = at("let user_id = 1", 11);
    buffer.delete(Direction::Backward, Unit::Word);
    assert_eq!(buffer.text(), "let  = 1");

    let mut buffer = at("a.b(c)", 0);
    assert_eq!(buffer.step(0, Direction::Forward, Unit::Word), 1);
    assert_eq!(buffer.step(1, Direction::Forward, Unit::Word), 2);
    buffer.move_to(6, false);
    assert_eq!(buffer.step(6, Direction::Backward, Unit::Word), 5);
}

#[test]
fn a_line_deleted_takes_its_line_ending() {
    let mut buffer = at("a\nb\nc", 2);
    buffer.delete(Direction::Forward, Unit::Line);
    assert_eq!(buffer.text(), "a\nc");
}

#[test]
fn nothing_to_delete_is_no_edit() {
    let mut buffer = at("a", 0);
    assert!(buffer.delete(Direction::Backward, Unit::Grapheme).is_none());
    assert!(buffer.undo().is_none());
}

#[test]
fn lines_move_up_and_down_with_the_caret() {
    let mut buffer = at("a\nb\nc", 2);
    buffer.move_lines(Direction::Backward);
    assert_eq!(buffer.text(), "b\na\nc");
    assert_eq!(buffer.selection(), Selection::caret(0));
    buffer.move_lines(Direction::Forward);
    buffer.move_lines(Direction::Forward);
    assert_eq!(buffer.text(), "a\nc\nb");
    assert!(buffer.move_lines(Direction::Forward).is_none());
}

#[test]
fn a_comment_goes_on_every_line_then_comes_off() {
    let mut buffer = Buffer::new("a\n  b\n\nc");
    buffer.select_all();
    buffer.toggle_comment("//");
    assert_eq!(buffer.text(), "// a\n  // b\n\n// c");
    buffer.select_all();
    buffer.toggle_comment("//");
    assert_eq!(buffer.text(), "a\n  b\n\nc");
}

#[test]
fn tab_and_shift_tab_are_one_step_each() {
    let mut buffer = Buffer::new("a\nb");
    buffer.select_all();
    buffer.indent("  ");
    assert_eq!(buffer.text(), "  a\n  b");
    buffer.outdent("  ");
    assert_eq!(buffer.text(), "a\nb");
    buffer.undo();
    assert_eq!(buffer.text(), "  a\n  b");
}

#[test]
fn search_counts_seeks_and_wraps() {
    let query = Query {
        search: "id".into(),
        ..Query::default()
    };
    let mut buffer = at("id x id x id", 4);
    assert!(buffer.seek(&query, 4, Direction::Forward));
    assert_eq!(buffer.selection().range(), 5..7);
    let count = buffer.count(&query);
    assert_eq!((count.total, count.current), (3, Some(2)));
    buffer.seek(&query, 11, Direction::Forward);
    assert_eq!(buffer.selection().range(), 0..2, "wraps round to the start");
    buffer.seek(&query, 0, Direction::Backward);
    assert_eq!(buffer.selection().range(), 10..12);
}

#[test]
fn replace_one_selects_first_then_replaces_and_replace_all_is_one_step() {
    let query = Query {
        search: "a".into(),
        replace: "xyz".into(),
        ..Query::default()
    };
    let mut buffer = at("a b a", 0);
    assert!(buffer.replace_one(&query).is_none());
    assert_eq!(buffer.selection().range(), 0..1);
    buffer.replace_one(&query);
    assert_eq!(buffer.text(), "xyz b a");
    assert_eq!(buffer.selection().range(), 6..7);

    let mut buffer = Buffer::new("a b a");
    let (count, _) = buffer.replace_all(&query);
    assert_eq!((count, buffer.text().as_str()), (2, "xyz b xyz"));
    buffer.undo();
    assert_eq!(buffer.text(), "a b a");
}

#[test]
fn an_edit_says_the_line_it_starts_on() {
    let mut buffer = at("a\nb\nc", 4);
    assert_eq!(buffer.insert("!").first_line, 2);
    assert_eq!(buffer.line_count(), 3);
    assert_eq!(buffer.line(2), "!c");
}
