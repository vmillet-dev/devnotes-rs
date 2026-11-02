//! Spike S2: Inter and JetBrains Mono (the variable files of `@fontsource-variable`, converted
//! to TTF) drawn by epaint 0.37 through `egui_kittest` + `wgpu`, one PNG per sample and per scale,
//! to be compared with Chromium (`compare.mjs`). `s2-fonts <out dir>` from spikes/s2-fonts.

use std::sync::Arc;

use egui::epaint::text::VariationCoords;
use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, vec2};
use egui_kittest::Harness;

struct Sample {
    id: String,
    mono: bool,
    size: f32,
    weight: f32,
    spacing: f32,
    width: f32,
    rows: usize,
    text: String,
}

fn samples() -> Vec<Sample> {
    std::fs::read_to_string("samples.tsv")
        .expect("samples")
        .lines()
        .map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            Sample {
                id: f[0].into(),
                mono: f[1] == "mono",
                size: f[2].parse().expect("size"),
                weight: f[3].parse().expect("weight"),
                spacing: f[4].parse().expect("spacing"),
                width: f[5].parse().expect("width"),
                rows: f[6].parse().expect("rows"),
                text: f[7].into(),
            }
        })
        .collect()
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::empty();
    for (name, bytes) in [
        ("inter", &include_bytes!("../fonts/InterVariable-latin.ttf")[..]),
        ("inter-ext", &include_bytes!("../fonts/InterVariable-latin-ext.ttf")[..]),
        ("mono", &include_bytes!("../fonts/JetBrainsMonoVariable-latin.ttf")[..]),
    ] {
        fonts.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
    }
    fonts.families.insert(FontFamily::Proportional, vec!["inter".into(), "inter-ext".into()]);
    fonts.families.insert(FontFamily::Monospace, vec!["mono".into(), "inter".into()]);
    fonts
}

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "../out/s2/egui".into()));
    for scale in [1.0f32, 1.25, 1.5] {
        let dir = out.join(format!("{}", (scale * 100.0) as u32));
        std::fs::create_dir_all(&dir).expect("out");
        for sample in samples() {
            let height = 8.0 + sample.size * 1.5 * sample.rows as f32;
            let s = Arc::new(sample);
            let drawn = s.clone();
            // The first pass runs inside `build_ui`, before our fonts exist: it draws nothing.
            let ready = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let ready_in_ui = ready.clone();
            let mut harness = Harness::builder()
                .with_size(vec2(drawn.width + 8.0, height))
                .with_pixels_per_point(scale)
                .wgpu()
                .build_ui(move |ui| {
                    if !ready_in_ui.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    let family = if drawn.mono { FontFamily::Monospace } else { FontFamily::Proportional };
                    let format = TextFormat {
                        font_id: FontId::new(drawn.size, family),
                        extra_letter_spacing: drawn.spacing,
                        line_height: Some(drawn.size * 1.5),
                        color: Color32::from_gray(232),
                        coords: VariationCoords::new([(b"wght", drawn.weight)]),
                        ..Default::default()
                    };
                    let mut job = LayoutJob::single_section(drawn.text.clone(), format);
                    job.wrap = TextWrapping { max_width: drawn.width, max_rows: drawn.rows, break_anywhere: drawn.rows == 1, overflow_character: Some('…') };
                    let galley = ui.fonts_mut(|f| f.layout_job(job));
                    // From the window's corner, not the `Ui`'s margin: the CSS box starts at 0, 0.
                    let painter = ui.ctx().layer_painter(egui::LayerId::background());
                    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(drawn.width + 8.0, height));
                    painter.rect_filled(rect, 0.0, Color32::from_rgb(22, 22, 24));
                    painter.galley(rect.min + vec2(4.0, 4.0), galley, Color32::WHITE);
                });
            harness.ctx.set_fonts(fonts());
            if std::env::var("NO_PROVIDER").is_err() { harness.ctx.add_font_provider(Arc::new(egui_system_font_provider::SystemFontProvider::new())); }
            ready.store(true, std::sync::atomic::Ordering::Relaxed);
            harness.run_steps(4);
            let image = harness.render().expect("rendered");
            image.save(dir.join(format!("{}.png", s.id))).expect("png");
        }
    }
    println!("written to {}", out.display());
}
