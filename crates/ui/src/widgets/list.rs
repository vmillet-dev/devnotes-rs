//! What can run to thousands of rows: drawn only where the viewport is (plan § 3.6), over rows
//! of one height. The code viewer is one of them, a line a row.

use egui::{Color32, FontFamily, Rect, Ui, Vec2};

use super::text;
use crate::theme::palette;

/// `count` rows of `height`, `row(ui, index)` called for the rows in view alone.
pub fn virtual_list(
    ui: &mut Ui,
    id: &str,
    height: f32,
    count: usize,
    mut row: impl FnMut(&mut Ui, usize),
) {
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, false])
        .show_rows(ui, height, count, |ui, range| {
            for index in range {
                row(ui, index);
            }
        });
}

/// A read-only listing: line numbers in `line_no`, the code in the monospace face. The lines
/// are the caller's, split once.
pub fn code_viewer(ui: &mut Ui, id: &str, lines: &[&str]) {
    let p = palette(ui.ctx());
    let digits = lines.len().max(1).ilog10() + 1;
    let gutter = text(
        ui,
        &"8".repeat(digits as usize),
        12.5,
        400.0,
        FontFamily::Monospace,
        p.line_no,
    )
    .size()
    .x + 16.0;
    egui::Frame::new()
        .fill(p.bg_1)
        .corner_radius(egui::CornerRadius::same(8))
        .show(ui, |ui| {
            virtual_list(ui, id, ROW, lines.len(), |ui, index| {
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), ROW),
                    egui::Sense::hover(),
                );
                let number = text(
                    ui,
                    &(index + 1).to_string(),
                    12.5,
                    400.0,
                    FontFamily::Monospace,
                    p.line_no,
                );
                let numbered = egui::pos2(
                    rect.left() + gutter - 8.0 - number.size().x,
                    rect.top() + 2.0,
                );
                ui.painter().galley(numbered, number, Color32::TRANSPARENT);
                let code = text(
                    ui,
                    lines[index],
                    12.5,
                    400.0,
                    FontFamily::Monospace,
                    p.text_0,
                );
                let clip =
                    Rect::from_min_max(egui::pos2(rect.left() + gutter, rect.top()), rect.max);
                ui.painter().with_clip_rect(clip).galley(
                    egui::pos2(clip.left(), rect.top() + 2.0),
                    code,
                    Color32::TRANSPARENT,
                );
            });
        });
}

const ROW: f32 = 20.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Drawn(Vec<usize>);

    #[test]
    fn only_the_rows_in_view_are_drawn() {
        let mut harness = crate::testing::harness()
            .with_size(egui::vec2(300.0, 200.0))
            .build_ui_state(
                |ui, drawn: &mut Drawn| {
                    drawn.0.clear();
                    virtual_list(ui, "rows", 20.0, 30_000, |ui, index| {
                        drawn.0.push(index);
                        ui.label(format!("row {index}"));
                    });
                },
                Drawn::default(),
            );
        harness.run();

        let drawn = &harness.state().0;
        assert!(
            !drawn.is_empty() && drawn.len() < 20,
            "{} rows drawn",
            drawn.len()
        );
        assert_eq!(drawn[0], 0);
    }

    #[test]
    fn a_listing_numbers_its_lines() {
        let lines = ["SELECT 1;", "SELECT 2;"];
        let mut harness = crate::testing::harness()
            .with_size(egui::vec2(300.0, 120.0))
            .build_ui(|ui| code_viewer(ui, "code", &lines));

        harness.run();
    }
}
