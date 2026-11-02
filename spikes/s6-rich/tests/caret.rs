//! S6: the caret crosses blocks with the arrows, and the input rules turn a paragraph into a block.

use egui::Key;
use egui_kittest::Harness;
use s6_rich::markdown::parse;
use s6_rich::model::Block;
use s6_rich::view::{Caret, RichView};

fn harness(markdown: &str) -> Harness<'static, RichView> {
    let view = RichView::new(parse(markdown));
    Harness::builder().with_size(egui::vec2(400.0, 400.0)).build_ui_state(|ui, view: &mut RichView| view.ui(ui), view)
}

fn press(h: &mut Harness<'static, RichView>, key: Key) {
    h.key_press(key);
    h.run();
}

#[test]
fn arrows_cross_blocks() {
    let mut h = harness("# Title\n\nFirst paragraph\n\n- item one\n- item two\n\nLast");
    h.run();
    press(&mut h, Key::ArrowDown);
    assert_eq!(h.state().caret.block, 1);
    press(&mut h, Key::ArrowDown);
    press(&mut h, Key::ArrowDown);
    assert_eq!(h.state().caret.block, 3, "into the second list item");
    press(&mut h, Key::ArrowUp);
    assert_eq!(h.state().caret.block, 2);
    h.state_mut().caret = Caret { block: 0, offset: 5 };
    h.run();
    press(&mut h, Key::ArrowRight);
    assert_eq!(h.state().caret, Caret { block: 1, offset: 0 }, "right at the end goes to the next block");
    press(&mut h, Key::ArrowLeft);
    assert_eq!(h.state().caret, Caret { block: 0, offset: 5 }, "left at the start comes back");
}

#[test]
fn a_long_paragraph_walks_its_rows_before_leaving() {
    let long = "word ".repeat(80);
    let mut h = harness(&format!("{long}\n\nnext"));
    h.run();
    press(&mut h, Key::ArrowDown);
    assert_eq!(h.state().caret.block, 0, "still in the wrapped paragraph");
    assert!(h.state().caret.offset > 0);
    for _ in 0..20 {
        press(&mut h, Key::ArrowDown);
    }
    assert_eq!(h.state().caret.block, 1);
}

#[test]
fn input_rules() {
    let mut h = harness("x");
    h.state_mut().blocks = vec![Block::Paragraph(vec![])];
    h.state_mut().caret = Caret { block: 0, offset: 0 };
    h.run();
    h.event(egui::Event::Text("## ".into()));
    h.run();
    assert!(matches!(h.state().blocks[0], Block::Heading(2, _)), "{:?}", h.state().blocks[0]);
    h.event(egui::Event::Text("Plan".into()));
    h.run();
    assert_eq!(h.state().blocks[0].text(), "Plan");
    assert_eq!(h.state().caret.offset, 4);
}
