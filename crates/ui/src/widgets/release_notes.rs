//! A release's notes, as the core cut them: sections, then bullets of plain, strong and code
//! runs — drawn, never interpreted.

use egui::text::LayoutJob;
use egui::{Color32, FontFamily, FontId, TextFormat, Ui};
use engine::changelog::model::{ChangelogRelease, SpanKind};

use crate::theme::palette;

pub fn release_notes(ui: &mut Ui, release: &ChangelogRelease) {
    let p = palette(ui.ctx());
    for section in &release.sections {
        if !section.title.is_empty() {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(section.title.to_uppercase())
                    .size(11.0)
                    .variation(*b"wght", 600.0)
                    .color(p.text_2),
            );
        }
        for entry in &section.items {
            let mut job = LayoutJob::default();
            job.append("•  ", 0.0, format(p.text_2, FontFamily::Proportional, None));
            for span in entry {
                let (colour, family, fill) = match span.kind {
                    SpanKind::Plain => (p.text_1, FontFamily::Proportional, None),
                    SpanKind::Strong => (p.text_0, FontFamily::Proportional, None),
                    SpanKind::Code => (p.text_0, FontFamily::Monospace, Some(p.bg_3)),
                };
                let mut written = format(colour, family, fill);
                if span.kind == SpanKind::Strong {
                    written.coords.push(*b"wght", 600.0);
                }
                job.append(&span.text, 0.0, written);
            }
            job.wrap.max_width = ui.available_width();
            ui.label(job);
        }
    }
}

fn format(colour: Color32, family: FontFamily, fill: Option<Color32>) -> TextFormat {
    let size = if family == FontFamily::Monospace {
        11.5
    } else {
        12.5
    };
    TextFormat {
        font_id: FontId::new(size, family),
        color: colour,
        background: fill.unwrap_or(Color32::TRANSPARENT),
        line_height: Some(20.0),
        ..TextFormat::default()
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::kittest::Queryable;

    use super::*;

    #[test]
    fn a_release_shows_its_sections_and_bullets() {
        let release = engine::changelog::app_changelog().remove(0);
        let first = release.sections[0].items[0]
            .iter()
            .map(|span| span.text.as_str())
            .collect::<String>();
        let mut harness = crate::testing::harness()
            .with_size(egui::vec2(600.0, 2000.0))
            .build_ui(|ui| release_notes(ui, &release));

        harness.run();

        assert!(harness.query_by_label_contains(&first).is_some(), "{first}");
    }
}
