//! A selection the document no longer reaches, as one held a frame too long: every command
//! answers it rather than panicking.

use richtext::{Document, ListKind, Position, Selection, edit, read, table};

fn beyond() -> Selection {
    Selection::caret(Position {
        path: vec![5, 3],
        offset: 40,
    })
}

#[test]
fn every_command_answers_a_selection_past_the_end() {
    let commands: [fn(&mut Document, &Selection); 12] = [
        |d, s| drop(edit::toggle_quote(d, s)),
        |d, s| drop(edit::insert_rule(d, s)),
        |d, s| drop(edit::replace_with_rule(d, s)),
        |d, s| drop(edit::toggle_list(d, s, ListKind::Bullet)),
        |d, s| drop(edit::toggle_code_block(d, s)),
        |d, s| edit::set_heading(d, s, Some(1)),
        |d, s| drop(edit::split(d, s)),
        |d, s| drop(edit::delete_backward(d, s)),
        |d, s| drop(edit::delete_forward(d, s)),
        |d, s| drop(table::insert(d, s)),
        |d, s| drop(table::add_row_after(d, s)),
        |d, s| drop(table::step(d, s, false)),
    ];
    let mut fell = Vec::new();
    for (at, command) in commands.iter().enumerate() {
        let mut document = read("one line");
        let answered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            command(&mut document, &beyond());
        }));
        if answered.is_err() {
            fell.push(at);
        }
    }
    assert!(fell.is_empty(), "commands {fell:?} panicked");
}
