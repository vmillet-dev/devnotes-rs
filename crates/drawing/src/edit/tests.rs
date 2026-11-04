use super::*;
use crate::scene::PathCommand;
use crate::text::tests::fonts;

fn editor() -> Editor {
    Editor::open("", Arc::new(fonts()))
}

fn at(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn drag(editor: &mut Editor, from: Point, to: Point) {
    editor.press(from, false);
    editor.drag(to);
    editor.release(to);
}

/// A rectangle from (100, 100) to (200, 160).
fn with_a_rectangle() -> Editor {
    let mut editor = editor();
    editor.set_tool(Tool::Rectangle);
    drag(&mut editor, at(100.0, 100.0), at(200.0, 160.0));
    editor
}

fn object(editor: &Editor, index: usize) -> &Object {
    &editor.drawing().objects[index]
}

#[test]
fn a_shape_covers_the_drag_is_selected_and_the_tool_goes_back() {
    let editor = with_a_rectangle();
    let shape = object(&editor, 0);

    assert_eq!(shape.kind(), Kind::Rect);
    assert_eq!(shape.centre(), at(150.0, 130.0));
    assert_eq!((shape.width(), shape.height()), (100.0, 60.0));
    assert_eq!(shape.corner_radius(), 6.0);
    assert_eq!(editor.tool(), Tool::Select);
    assert_eq!(editor.selected(), [0]);
    assert!(shape.id().is_some());
}

#[test]
fn a_click_with_a_shape_tool_gives_the_size_a_click_gives() {
    let mut editor = editor();
    editor.set_tool(Tool::Ellipse);
    drag(&mut editor, at(10.0, 10.0), at(12.0, 11.0));
    let shape = object(&editor, 0);

    assert_eq!(shape.kind(), Kind::Ellipse);
    assert_eq!((shape.width(), shape.height()), (140.0, 80.0));
    assert_eq!(shape.centre(), at(80.0, 50.0));
}

#[test]
fn a_diamond_has_its_corners_mid_side() {
    let mut editor = editor();
    editor.set_tool(Tool::Diamond);
    drag(&mut editor, at(0.0, 0.0), at(40.0, 20.0));

    assert_eq!(object(&editor, 0).kind(), Kind::Polygon);
    assert_eq!(object(&editor, 0).points()[1], at(40.0, 10.0));
}

#[test]
fn a_new_drawing_is_written_as_a_scene_read_back() {
    let editor = with_a_rectangle();

    let read = Drawing::read(&editor.body()).unwrap();

    assert_eq!(read.objects.len(), 1);
    assert!(
        editor
            .body()
            .starts_with(r#"{"version":1,"scene":{"version":"7.4.0""#)
    );
}

#[test]
fn an_arrow_started_and_ended_on_shapes_stays_on_their_edges() {
    let mut editor = with_a_rectangle();
    editor.set_tool(Tool::Ellipse);
    drag(&mut editor, at(400.0, 100.0), at(500.0, 160.0));
    editor.set_tool(Tool::Arrow);
    drag(&mut editor, at(150.0, 130.0), at(450.0, 130.0));
    let arrow = object(&editor, 2);

    assert_eq!(arrow.from(), object(&editor, 0).id());
    assert_eq!(arrow.to(), object(&editor, 1).id());
    assert_eq!(arrow.ends(), Some([201.0, 130.0, 399.0, 130.0]));
    assert_eq!(arrow.path().last(), Some(&PathCommand::Close));

    drag(&mut editor, at(150.0, 130.0), at(150.0, 130.0));
    drag(&mut editor, at(150.0, 130.0), at(150.0, 330.0));
    let moved = object(&editor, 2).ends().unwrap();
    assert!(moved[1] > 130.0, "the arrow followed: {moved:?}");
}

#[test]
fn an_arrow_on_nothing_ends_where_it_was_let_go() {
    let mut editor = editor();
    editor.set_tool(Tool::Arrow);
    drag(&mut editor, at(0.0, 0.0), at(100.0, 0.0));
    let arrow = object(&editor, 0);

    assert_eq!(arrow.from(), None);
    assert_eq!(arrow.to(), None);
    assert_eq!(arrow.ends(), Some([0.0, 0.0, 100.0, 0.0]));
    assert_eq!(arrow.centre(), at(50.0, 0.0));
}

#[test]
fn an_arrow_cannot_end_on_the_shape_it_started_on_nor_on_a_text() {
    let mut editor = with_a_rectangle();
    editor.set_tool(Tool::Arrow);
    drag(&mut editor, at(120.0, 130.0), at(180.0, 130.0));

    assert_eq!(object(&editor, 1).to(), None);

    editor.set_tool(Tool::Text);
    editor.press(at(300.0, 300.0), false);
    editor.set_text("words");
    editor.leave_text();
    editor.set_tool(Tool::Arrow);
    drag(&mut editor, at(0.0, 0.0), at(350.0, 300.0));
    assert_eq!(object(&editor, 3).to(), None);
}

#[test]
fn a_shape_removed_lets_its_arrows_go() {
    let mut editor = with_a_rectangle();
    editor.set_tool(Tool::Arrow);
    drag(&mut editor, at(150.0, 130.0), at(400.0, 130.0));
    editor.press(at(150.0, 130.0), false);
    editor.release(at(150.0, 130.0));

    assert!(editor.remove_selection());

    assert_eq!(editor.shapes(), 1);
    assert_eq!(object(&editor, 0).from(), None);
    assert!(!editor.remove_selection());
}

#[test]
fn the_pen_leaves_a_smoothed_line_unfilled_and_a_click_leaves_none() {
    let mut editor = editor();
    editor.set_tool(Tool::Pen);
    editor.press(at(0.0, 0.0), false);
    assert!(editor.pen_line().is_some());
    for step in 1..10 {
        editor.drag(at(f64::from(step) * 5.0, f64::from(step % 2) * 4.0));
    }
    editor.release(at(50.0, 0.0));
    let line = object(&editor, 0);

    assert_eq!(line.kind(), Kind::Path);
    assert_eq!(line.line_cap(), "round");
    assert_eq!(line.stroke_width(), 2.5);
    assert!(matches!(line.path()[1], PathCommand::Quad(..)));
    assert_eq!(editor.tool(), Tool::Pen);

    drag(&mut editor, at(9.0, 9.0), at(9.0, 9.0));
    assert_eq!(editor.shapes(), 1);
}

#[test]
fn a_text_is_typed_where_clicked_and_left_empty_it_goes() {
    let mut editor = editor();
    editor.set_tool(Tool::Text);
    editor.press(at(10.0, 20.0), false);

    assert_eq!(editor.tool(), Tool::Select);
    assert!(editor.editing().is_some());
    assert_eq!(editor.frame(), None);
    assert_eq!(object(&editor, 0).centre(), at(100.0, 20.0));

    editor.leave_text();
    assert_eq!(editor.shapes(), 0);
    assert!(!editor.can_undo());
}

#[test]
fn a_text_grows_with_its_lines_and_a_second_click_types_in_it_again() {
    let mut editor = editor();
    editor.set_tool(Tool::Text);
    editor.press(at(10.0, 20.0), false);
    editor.set_text("one\ntwo");
    editor.leave_text();

    assert_eq!(object(&editor, 0).height(), 43.93);
    assert!(editor.can_undo());

    let inside = object(&editor, 0).centre();
    editor.press(inside, false);
    editor.release(inside);
    assert!(editor.editing().is_some());
}

#[test]
fn a_click_selects_the_topmost_and_on_the_paper_drops_the_selection() {
    let mut editor = with_a_rectangle();
    editor.set_tool(Tool::Rectangle);
    drag(&mut editor, at(120.0, 120.0), at(220.0, 180.0));

    drag(&mut editor, at(150.0, 140.0), at(150.0, 140.0));
    assert_eq!(editor.selected(), [1]);

    drag(&mut editor, at(105.0, 105.0), at(105.0, 105.0));
    assert_eq!(editor.selected(), [0]);

    drag(&mut editor, at(600.0, 600.0), at(600.0, 600.0));
    assert!(editor.selected().is_empty());
}

#[test]
fn shift_adds_and_takes_away_and_a_marquee_selects_what_it_touches() {
    let mut editor = with_a_rectangle();
    editor.set_tool(Tool::Rectangle);
    drag(&mut editor, at(300.0, 100.0), at(400.0, 160.0));
    editor.deselect();

    editor.press(at(150.0, 130.0), true);
    editor.release(at(150.0, 130.0));
    editor.press(at(350.0, 130.0), true);
    editor.release(at(350.0, 130.0));
    assert_eq!(editor.selected(), [0, 1]);

    editor.press(at(150.0, 130.0), true);
    editor.release(at(150.0, 130.0));
    assert_eq!(editor.selected(), [1]);

    editor.deselect();
    editor.press(at(50.0, 50.0), false);
    editor.drag(at(120.0, 120.0));
    assert!(editor.marquee().is_some());
    editor.release(at(120.0, 120.0));
    assert_eq!(editor.selected(), [0]);
}

#[test]
fn what_is_selected_moves_together() {
    let mut editor = with_a_rectangle();

    drag(&mut editor, at(150.0, 130.0), at(170.0, 140.0));

    assert_eq!(object(&editor, 0).centre(), at(170.0, 140.0));
    assert!(editor.can_undo());
}

#[test]
fn a_corner_keeps_the_proportions_from_the_opposite_corner() {
    let mut editor = with_a_rectangle();
    let frame = editor.frame().unwrap();
    let corner = frame.grip(Grip::BottomRight);
    let anchor = frame.grip(Grip::TopLeft);

    drag(
        &mut editor,
        corner,
        at(anchor.x + frame.width * 2.0, anchor.y + frame.height * 2.0),
    );

    let shape = object(&editor, 0);
    assert_eq!((shape.scale_x(), shape.scale_y()), (2.0, 2.0));
    let moved = editor.frame().unwrap().grip(Grip::TopLeft);
    assert!((moved.x - anchor.x).abs() < 2.0 && (moved.y - anchor.y).abs() < 2.0);
}

#[test]
fn a_side_scales_one_way_and_a_text_gets_wider_instead() {
    let mut editor = with_a_rectangle();
    let frame = editor.frame().unwrap();
    let side = frame.grip(Grip::Right);

    drag(&mut editor, side, at(side.x + frame.width, side.y));
    assert_eq!(object(&editor, 0).scale_x(), 2.0);
    assert_eq!(object(&editor, 0).scale_y(), 1.0);

    let mut editor = self::editor();
    editor.set_tool(Tool::Text);
    editor.press(at(0.0, 100.0), false);
    editor.set_text("words enough to wrap across two lines at least");
    editor.leave_text();
    let height = object(&editor, 0).height();
    let frame = editor.frame().unwrap();

    let side = frame.grip(Grip::Right);
    drag(&mut editor, side, at(side.x - 90.0, side.y));

    assert_eq!(object(&editor, 0).scale_x(), 1.0);
    assert!(object(&editor, 0).width() < 100.0);
    assert!(object(&editor, 0).height() > height);
}

#[test]
fn the_rotation_handle_turns_around_the_centre() {
    let mut editor = with_a_rectangle();
    let frame = editor.frame().unwrap();

    drag(
        &mut editor,
        frame.grip(Grip::Rotate),
        at(frame.centre.x + 100.0, frame.centre.y),
    );

    assert_eq!(object(&editor, 0).angle(), 90.0);
    assert_eq!(object(&editor, 0).centre(), frame.centre);
}

#[test]
fn several_objects_turn_around_their_frame() {
    let mut editor = with_a_rectangle();
    editor.set_tool(Tool::Rectangle);
    drag(&mut editor, at(300.0, 100.0), at(400.0, 160.0));
    editor.press(at(150.0, 130.0), true);
    editor.release(at(150.0, 130.0));
    let frame = editor.frame().unwrap();

    drag(
        &mut editor,
        frame.grip(Grip::Rotate),
        at(frame.centre.x, frame.centre.y + 100.0),
    );

    assert_eq!(object(&editor, 0).angle(), 180.0);
    assert!(object(&editor, 0).centre().x > object(&editor, 1).centre().x);
}

#[test]
fn a_hue_is_for_what_is_drawn_next_and_what_is_selected() {
    let mut editor = with_a_rectangle();

    editor.set_hue(Hue::Red);
    assert_eq!(object(&editor, 0).hue(), Hue::Red);

    editor.deselect();
    editor.set_hue(Hue::Blue);
    assert_eq!(object(&editor, 0).hue(), Hue::Red);
    editor.set_tool(Tool::Rectangle);
    drag(&mut editor, at(300.0, 300.0), at(340.0, 340.0));
    assert_eq!(object(&editor, 1).hue(), Hue::Blue);
    assert_eq!(editor.hue(), Hue::Blue);
}

#[test]
fn undo_and_redo_walk_the_history_and_a_change_drops_what_was_undone() {
    let mut editor = with_a_rectangle();
    drag(&mut editor, at(150.0, 130.0), at(250.0, 130.0));

    editor.undo();
    assert_eq!(object(&editor, 0).centre(), at(150.0, 130.0));
    assert!(editor.can_redo());

    editor.redo();
    assert_eq!(object(&editor, 0).centre(), at(250.0, 130.0));

    editor.undo();
    editor.undo();
    assert_eq!(editor.shapes(), 0);
    assert!(!editor.can_undo());
    editor.redo();
    editor.set_tool(Tool::Ellipse);
    drag(&mut editor, at(0.0, 0.0), at(40.0, 40.0));
    assert!(!editor.can_redo());
}

#[test]
fn the_history_keeps_sixty_states() {
    let mut editor = with_a_rectangle();
    for step in 0..70 {
        let from = object(&editor, 0).centre();
        drag(
            &mut editor,
            from,
            at(from.x + 1.0, from.y + f64::from(step % 2)),
        );
    }

    let mut back = 0;
    while editor.can_undo() {
        editor.undo();
        back += 1;
    }

    assert_eq!(back, 59);
}

#[test]
fn a_click_that_moves_nothing_is_no_state() {
    let mut editor = with_a_rectangle();
    editor.undo();
    editor.redo();
    let _ = editor.take_unsaved();

    drag(&mut editor, at(150.0, 130.0), at(150.0, 130.0));

    assert!(!editor.take_unsaved());
    assert!(!editor.can_redo());
}

#[test]
fn escape_gives_back_the_selection_a_tool_took_over() {
    let mut editor = with_a_rectangle();

    editor.set_tool(Tool::Arrow);
    assert!(editor.selected().is_empty());

    editor.back_to_selection();
    assert_eq!(editor.tool(), Tool::Select);
    assert_eq!(editor.selected(), [0]);

    assert!(editor.deselect());
    assert!(!editor.deselect());
}

#[test]
fn what_it_opened_comes_back_unchanged_until_it_is_touched() {
    let body = r#"{"version":1,"scene":{"objects":[{"type":"Rect","left":60,"top":45,"width":100,"height":50,"later":true}]}}"#;
    let mut editor = Editor::open(body, Arc::new(fonts()));

    assert_eq!(editor.body(), body);
    assert_eq!(editor.shapes(), 1);
    assert!(!editor.take_unsaved());

    drag(&mut editor, at(60.0, 45.0), at(70.0, 45.0));

    assert!(editor.take_unsaved());
    assert!(editor.body().contains(r#""later":true"#));
    assert!(editor.body().contains(r#""left":70"#));
}
