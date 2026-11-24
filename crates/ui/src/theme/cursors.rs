//! The open and the closed hand, drawn: on Windows winit shows both as the four arrows. A frame
//! asks for them by `CursorIcon`, as everywhere; what is drawn is put over it at its end.

use std::fmt::Write as _;
use std::sync::Arc;

use egui::{Context, CursorIcon, CustomCursorImage, Id};

/// In points, as the system's own arrow at 100 %.
const SIDE: f32 = 26.0;

/// Lucide's `hand` and `grab`, then what stands between the fingers and the palm: their lines are
/// open, and a cursor is full.
const OPEN: ([&str; 4], &str) = (
    [
        "M18 11V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2",
        "M14 10V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2",
        "M10 10.5V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2v8",
        "M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15",
    ],
    "M6 6h4V4h4v2h4v4h4v5a7 7 0 0 1-7 6h-3a7 7 0 0 1-5-2L4 15l2-1z",
);
const CLOSED: ([&str; 5], &str) = (
    [
        "M18 11.5V9a2 2 0 0 0-2-2a2 2 0 0 0-2 2v1.4",
        "M14 10V8a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2",
        "M10 9.9V9a2 2 0 0 0-2-2a2 2 0 0 0-2 2v5",
        "M6 14a2 2 0 0 0-2-2a2 2 0 0 0-2 2",
        "M18 11a2 2 0 1 1 4 0v3a8 8 0 0 1-8 8h-4a8 8 0 0 1-8-8 2 2 0 1 1 4 0",
    ],
    "M6 9h4V8h4v1h4v2h4v3a8 8 0 0 1-8 8h-4a8 8 0 0 1-8-8h4z",
);

#[derive(Clone)]
struct Drawn {
    scale: f32,
    open: CustomCursorImage,
    closed: CustomCursorImage,
}

impl Drawn {
    fn of(self, closed: bool) -> CustomCursorImage {
        if closed { self.closed } else { self.open }
    }
}

/// At the end of a frame: the hand drawn over the cursor it asked for.
pub fn show(ctx: &Context) {
    if !cfg!(windows) {
        return;
    }
    let icon = ctx.output(|output| output.cursor_icon);
    if matches!(icon, CursorIcon::Grab | CursorIcon::Grabbing) {
        ctx.set_cursor_image(hand(ctx, icon == CursorIcon::Grabbing));
    }
}

/// One image a hand and a scale: the integration knows a cursor by its bytes' address.
fn hand(ctx: &Context, closed: bool) -> Option<CustomCursorImage> {
    let scale = ctx.pixels_per_point();
    let id = Id::unique("theme.cursors");
    let held: Option<Drawn> = ctx.data(|data| data.get_temp(id));
    if let Some(held) = held.filter(|held| (held.scale - scale).abs() < f32::EPSILON) {
        return Some(held.of(closed));
    }
    let drawn = Drawn {
        scale,
        open: image(&OPEN.0, OPEN.1, scale)?,
        closed: image(&CLOSED.0, CLOSED.1, scale)?,
    };
    ctx.data_mut(|data| data.insert_temp(id, drawn.clone()));
    Some(drawn.of(closed))
}

/// Full in white and outlined in black, for it to show on either theme.
fn svg(lines: &[&str], inside: &str) -> String {
    let group = |style: &str, paths: &[&str]| {
        let paths = paths.iter().fold(String::new(), |mut paths, d| {
            let _ = write!(paths, "<path d=\"{d}\"/>");
            paths
        });
        format!("<g {style}>{paths}</g>")
    };
    let halo = group("fill=\"none\" stroke=\"white\" stroke-width=\"4\"", lines);
    let full = group("fill=\"white\"", &[inside]);
    let outline = group("fill=\"none\" stroke=\"black\" stroke-width=\"1.5\"", lines);
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"-1 -1 26 26\" \
         stroke-linecap=\"round\" stroke-linejoin=\"round\">{halo}{full}{outline}</svg>"
    )
}

fn image(lines: &[&str], inside: &str, scale: f32) -> Option<CustomCursorImage> {
    let svg = svg(lines, inside);
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).ok()?;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a cursor is a few dozen pixels wide"
    )]
    let side = (SIDE * scale).round().clamp(16.0, 128.0) as u16;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(side.into(), side.into())?;
    let zoom = f32::from(side) / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(zoom, zoom),
        &mut pixmap.as_mut(),
    );
    let straight = pixmap.pixels().iter().flat_map(|pixel| {
        let pixel = pixel.demultiply();
        [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]
    });
    Some(CustomCursorImage {
        rgba: Arc::from(straight.collect::<Vec<u8>>()),
        size: [side, side],
        hotspot: [side / 2, side / 2],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_hand_is_drawn_once_a_scale_and_shows_on_both_themes() {
        let mut harness = crate::testing::harness().build_ui(|ui| {
            let ctx = ui.ctx();
            let open = hand(ctx, false).unwrap();
            let closed = hand(ctx, true).unwrap();
            assert_eq!(open.size, [26, 26]);
            assert_eq!(open.rgba.len(), 26 * 26 * 4);
            assert_ne!(open.rgba, closed.rgba);
            assert!(Arc::ptr_eq(&open.rgba, &hand(ctx, false).unwrap().rgba));
            for image in [open, closed] {
                let of = |white: u8| {
                    let pixels = image.rgba.chunks(4);
                    pixels
                        .filter(|pixel| pixel[3] == 255 && pixel[0] == white)
                        .count()
                };
                assert!(of(255) > 60, "{} white", of(255));
                assert!(of(0) > 10, "{} black", of(0));
            }
        });
        harness.run();
    }

    #[test]
    #[cfg(windows)]
    fn a_frame_that_asks_for_a_hand_gets_the_one_drawn() {
        for (icon, drawn) in [
            (CursorIcon::Grab, true),
            (CursorIcon::Grabbing, true),
            (CursorIcon::PointingHand, false),
        ] {
            let mut harness = crate::testing::harness().build_ui(move |ui| {
                ui.ctx().set_cursor_icon(icon);
                show(ui.ctx());
            });
            harness.run();
            let image = &harness.output().platform_output.cursor_image;
            assert_eq!(image.is_some(), drawn, "{icon:?}");
        }
    }
}
