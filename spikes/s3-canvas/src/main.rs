//! Spike S3: 30,000 hand-painted cards, virtualised in rows, scrolled frame after frame.
//!
//! Drives an `egui::Context` without a window: each frame is `Context::run` + `tessellate`, the
//! CPU side of a real frame (the GPU upload is not counted). Reports p50/p95/p99 per scenario.

use std::time::Instant;

use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2,
};

const CARDS: usize = 30_000;
const CARD_H: f32 = 150.0;
const GAP: f32 = 16.0;
const MIN_W: f32 = 220.0;
const HEADER_H: f32 = 40.0;
const SECTIONS: &[(&str, usize)] = &[
    ("Épinglées", 40),
    ("Aujourd'hui", 160),
    ("Hier", 300),
    ("Cette semaine", 1_500),
    ("Ce mois-ci", 6_000),
    ("Plus tôt", CARDS - 8_000),
];

struct Card {
    title: String,
    preview: String,
    tags: Vec<String>,
    hue: Color32,
}

fn cards() -> Vec<Card> {
    let lines = [
        "let étape = migration::run(&mut connection)?;",
        "kubectl rollout restart deployment/api -n prod",
        "SELECT calls, mean_exec_time FROM pg_stat_statements",
        "const keysOf = <T extends object>(v: T) => Object.keys(v);",
        "curl -H 'Authorization: Bearer {{token}}' {{host}}",
        "## Résumé de la réunion — décisions prises",
    ];
    let hues = [
        Color32::from_rgb(232, 168, 56),
        Color32::from_rgb(96, 165, 250),
        Color32::from_rgb(52, 211, 153),
        Color32::from_rgb(244, 114, 182),
    ];
    (0..CARDS)
        .map(|n| Card {
            title: format!("Note {n} — étape de déploiement"),
            preview: (0..6).map(|l| lines[(n + l) % lines.len()]).collect::<Vec<_>>().join("\n"),
            tags: vec![format!("tag-{}", n % 600), format!("ops-{}", n % 7)],
            hue: hues[n % hues.len()],
        })
        .collect()
}

/// A virtual row: a section header or a row of cards, with its offset precomputed.
enum Row {
    Header(&'static str, usize),
    Cards(usize, usize),
}

fn rows(columns: usize) -> (Vec<(f32, Row)>, f32) {
    let mut rows = Vec::new();
    let mut y = 0.0;
    let mut first = 0;
    for (name, count) in SECTIONS {
        rows.push((y, Row::Header(name, *count)));
        y += HEADER_H;
        let mut at = 0;
        while at < *count {
            let n = columns.min(count - at);
            rows.push((y, Row::Cards(first + at, n)));
            y += CARD_H + GAP;
            at += n;
        }
        first += count;
    }
    (rows, y)
}

fn paint_card(ui: &mut egui::Ui, rect: Rect, card: &Card, id: usize) {
    let response = ui.interact(rect, egui::Id::new(("card", id)), Sense::click());
    let hovered = response.hovered();
    let painter = ui.painter();
    let bg = if hovered { Color32::from_gray(38) } else { Color32::from_gray(30) };
    painter.rect(
        rect,
        CornerRadius::same(10),
        bg,
        Stroke::new(1.0, if hovered { card.hue } else { Color32::from_gray(52) }),
        StrokeKind::Inside,
    );
    painter.rect_filled(
        Rect::from_min_size(rect.min + vec2(12.0, 14.0), vec2(6.0, 6.0)),
        CornerRadius::same(3),
        card.hue,
    );
    let title_w = rect.width() - 36.0;
    let title = ui.fonts_mut(|f| {
        let mut job = egui::text::LayoutJob::simple_singleline(
            card.title.clone(),
            FontId::proportional(14.0),
            Color32::from_gray(230),
        );
        job.wrap.max_width = title_w;
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        f.layout_job(job)
    });
    painter.galley(rect.min + vec2(26.0, 8.0), title, Color32::WHITE);
    let preview = ui.fonts_mut(|f| {
        let mut job = egui::text::LayoutJob::default();
        for (n, line) in card.preview.lines().enumerate() {
            let colour = [Color32::from_rgb(198, 120, 221), Color32::from_gray(170), Color32::from_rgb(152, 195, 121)][n % 3];
            job.append(line, 0.0, egui::TextFormat::simple(FontId::monospace(12.0), colour));
            job.append("\n", 0.0, egui::TextFormat::simple(FontId::monospace(12.0), colour));
        }
        job.wrap.max_width = rect.width() - 24.0;
        job.wrap.max_rows = 5;
        f.layout_job(job)
    });
    painter.galley(rect.min + vec2(12.0, 34.0), preview, Color32::WHITE);
    let mut x = rect.min.x + 12.0;
    for tag in &card.tags {
        let galley = ui.fonts_mut(|f| f.layout_no_wrap(tag.clone(), FontId::proportional(11.0), Color32::from_gray(180)));
        let chip = Rect::from_min_size(pos2(x, rect.max.y - 26.0), galley.size() + vec2(12.0, 6.0));
        painter.rect_filled(chip, CornerRadius::same(8), Color32::from_gray(44));
        painter.galley(chip.min + vec2(6.0, 3.0), galley, Color32::WHITE);
        x = chip.max.x + 6.0;
    }
}

fn canvas(ui: &mut egui::Ui, cards: &[Card], scroll_to: Option<f32>) {
    let width = ui.available_width();
    let columns = (((width + GAP) / (MIN_W + GAP)).floor() as usize).max(1);
    let card_w = (width - GAP * (columns as f32 - 1.0)) / columns as f32;
    let (rows, total) = rows(columns);
    let mut area = egui::ScrollArea::vertical().auto_shrink(false);
    if let Some(offset) = scroll_to {
        area = area.vertical_scroll_offset(offset);
    }
    let output = area.show_viewport(ui, |ui, viewport| {
        ui.set_height(total);
        let origin = ui.min_rect().min;
        // Binary search on the precomputed offsets: only the rows crossing the viewport.
        let first = rows.partition_point(|(y, _)| *y + CARD_H + GAP < viewport.min.y);
        for (y, row) in &rows[first..] {
            if *y > viewport.max.y {
                break;
            }
            let top = origin.y + y;
            match row {
                Row::Header(name, count) => {
                    ui.painter().text(
                        pos2(origin.x, top + 20.0),
                        Align2::LEFT_CENTER,
                        format!("{name} · {count}"),
                        FontId::proportional(13.0),
                        Color32::from_gray(160),
                    );
                }
                Row::Cards(start, n) => {
                    for c in 0..*n {
                        let min = pos2(origin.x + c as f32 * (card_w + GAP), top);
                        paint_card(ui, Rect::from_min_size(min, vec2(card_w, CARD_H)), &cards[start + c], start + c);
                    }
                }
            }
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("offset"), output.state.offset().y));
}

fn percentile(samples: &mut [f64], p: f64) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[((samples.len() as f64 - 1.0) * p).round() as usize]
}

fn main() {
    let cards = cards();
    let ctx = egui::Context::default();
    let size = vec2(1920.0 - 260.0, 1080.0 - 90.0);
    let mut time = 0.0;
    let mut frame = |events: Vec<egui::Event>, scroll_to: Option<f32>| -> (f64, usize) {
        time += 1.0 / 60.0;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            time: Some(time),
            events,
            ..Default::default()
        };
        let started = Instant::now();
        let output = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show_inside(ui, |ui| canvas(ui, &cards, scroll_to));
        });
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let vertices = primitives
            .iter()
            .map(|p| match &p.primitive {
                egui::epaint::Primitive::Mesh(mesh) => mesh.vertices.len(),
                egui::epaint::Primitive::Callback(_) => 0,
            })
            .sum();
        (started.elapsed().as_secs_f64() * 1e3, vertices)
    };

    let first = frame(vec![], None);
    println!("first frame (fonts, empty galley cache): {:.1} ms", first.0);
    for _ in 0..30 {
        frame(vec![], None);
    }

    let mut report = |name: &str, samples: &mut Vec<f64>, vertices: usize| {
        let (p50, p95, p99) = (percentile(samples, 0.5), percentile(samples, 0.95), percentile(samples, 0.99));
        let max = samples.iter().copied().fold(0.0, f64::max);
        println!("{name:<44} p50 {p50:5.2}  p95 {p95:5.2}  p99 {p99:5.2}  max {max:5.2} ms  (~{vertices} vertices)");
    };

    // Wheel scroll with the pointer moving over the cards: new rows come into view every frame.
    let mut samples = Vec::new();
    let mut vertices = 0;
    for n in 0..2_000 {
        let pointer = pos2(200.0 + (n % 900) as f32, 300.0 + (n % 400) as f32);
        let (ms, v) = frame(
            vec![
                egui::Event::PointerMoved(pointer),
                egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: vec2(0.0, -120.0), modifiers: Default::default(), phase: egui::TouchPhase::Move, source: egui::MouseWheelSource::Wheel },
            ],
            None,
        );
        samples.push(ms);
        vertices = v;
    }
    let offset: f32 = ctx.data(|d| d.get_temp(egui::Id::new("offset")).unwrap_or_default());
    println!("scrolled to {offset:.0} px of {:.0}", rows(7).1);
    report("wheel scroll + hover, 2000 frames", &mut samples, vertices);

    // Jumps across the whole list (scroll bar drag, End): every frame lays out unseen cards.
    let (_, total) = rows(7);
    let mut samples = Vec::new();
    for n in 0..500 {
        let offset = (n as f32 * 7919.0) % total;
        samples.push(frame(vec![egui::Event::PointerMoved(pos2(400.0, 400.0))], Some(offset)).0);
    }
    report("jump to a random offset, 500 frames", &mut samples, vertices);

    // Idle with hover moving inside one screen: the galley cache is warm.
    let mut samples = Vec::new();
    for n in 0..1_000 {
        samples.push(frame(vec![egui::Event::PointerMoved(pos2(100.0 + (n % 1400) as f32, 200.0 + (n % 600) as f32))], None).0);
    }
    report("hover only, 1000 frames", &mut samples, vertices);
}
