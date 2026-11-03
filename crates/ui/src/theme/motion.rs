//! The reference's durations and its three entrances, the only motion the interface has. One
//! switch, [`set_reduced`], turns all of it off, like the reference's single
//! `prefers-reduced-motion` block: what is added later is cut without being named.

use egui::emath::easing;
use egui::{Context, Id, Pos2, Stroke, Ui};

/// What follows the pointer: a hover fill, a chip's state, a control appearing under it.
pub const FAST: f32 = 0.12;
/// What enters: a menu, a dialog, a transient bar. Anything slower reads as lag on a tool
/// used in bursts.
pub const ENTER: f32 = 0.16;
/// A spinner's turn, the one motion that lasts.
pub const TURN: f32 = 0.8;
/// Not a motion: how long a mark left by an action takes to fade, so the eye can find it.
pub const MARK: f32 = 2.0;

/// Where an element comes from when it appears: a short rise or lift, or a fade — never a size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entrance {
    /// From 4 px above: what opens downwards, a menu.
    Rise,
    /// From 4 px below: what opens upwards, a bar at the bottom.
    Lift,
    Fade,
}

/// How far into its entrance an element is: its opacity, and the vertical offset to draw it at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entering {
    pub opacity: f32,
    pub offset: f32,
}

const DISTANCE: f32 = 4.0;

/// Turns every animation off, egui's own included.
pub fn set_reduced(ctx: &Context, reduced: bool) {
    ctx.data_mut(|data| data.insert_temp(reduced_id(), reduced));
    ctx.all_styles_mut(|style| style.animation_time = if reduced { 0.0 } else { FAST });
}

pub fn reduced(ctx: &Context) -> bool {
    ctx.data(|data| data.get_temp(reduced_id()).unwrap_or(false))
}

fn reduced_id() -> Id {
    Id::unique("theme.motion.reduced")
}

/// An element drawn this frame under `id`: it enters from the first frame it is drawn after
/// a frame it was not, as a CSS animation runs on an element that did not exist a frame ago.
pub fn enter(ctx: &Context, id: Id, entrance: Entrance) -> Entering {
    let now = ctx.input(|input| input.time);
    let frame = ctx.cumulative_frame_nr();
    let started = ctx.data_mut(|data| {
        let seen = data.get_temp_mut_or(id.with("entered"), (now, frame));
        if seen.1 + 1 < frame {
            seen.0 = now;
        }
        seen.1 = frame;
        seen.0
    });
    if reduced(ctx) {
        return Entering {
            opacity: 1.0,
            offset: 0.0,
        };
    }
    #[allow(clippy::cast_possible_truncation)]
    let progress = (((now - started) as f32) / ENTER).clamp(0.0, 1.0);
    if progress < 1.0 {
        ctx.request_repaint();
    }
    let eased = easing::cubic_out(progress);
    let from = match entrance {
        Entrance::Rise => -DISTANCE,
        Entrance::Lift => DISTANCE,
        Entrance::Fade => 0.0,
    };
    Entering {
        opacity: eased,
        offset: from * (1.0 - eased),
    }
}

/// What is left of a mark set at `since` (in `input.time`): 1 when set, 0 once faded.
pub fn fading(ctx: &Context, since: f64) -> f32 {
    if reduced(ctx) {
        return 0.0;
    }
    let now = ctx.input(|input| input.time);
    #[allow(clippy::cast_possible_truncation)]
    let left = 1.0 - ((now - since) as f32 / MARK).clamp(0.0, 1.0);
    if left > 0.0 {
        ctx.request_repaint();
    }
    left
}

/// A spinner of `radius` around `centre`: three quarters of a circle turning once a `TURN`,
/// still under reduced motion.
pub fn spinner(ui: &Ui, centre: Pos2, radius: f32, stroke: Stroke) {
    let ctx = ui.ctx();
    let start = if reduced(ctx) {
        0.0
    } else {
        ctx.request_repaint();
        #[allow(clippy::cast_possible_truncation)]
        let turn = (ctx.input(|input| input.time) / f64::from(TURN)).fract() as f32;
        turn * std::f32::consts::TAU
    };
    let points: Vec<Pos2> = (0..=24)
        .map(|step| {
            #[allow(clippy::cast_precision_loss)]
            let angle = start + step as f32 / 24.0 * 1.5 * std::f32::consts::PI;
            centre + radius * egui::vec2(angle.cos(), angle.sin())
        })
        .collect();
    ui.painter().add(egui::Shape::line(points, stroke));
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn frame(ctx: &Context, time: f64, mut draw: impl FnMut(&Context)) {
        crate::testing::frame(ctx, time, |ui| draw(ui.ctx()));
    }

    #[test]
    fn an_element_enters_once_and_again_after_it_was_gone() {
        let ctx = Context::default();
        let id = Id::unique("menu");
        let mut seen = Vec::new();

        for (time, drawn) in [
            (0.0, true),
            (0.08, true),
            (1.0, true),
            (1.1, false),
            (1.2, true),
        ] {
            frame(&ctx, time, |ctx| {
                if drawn {
                    seen.push(enter(ctx, id, Entrance::Rise));
                }
            });
        }

        assert!(seen[0].opacity < f32::EPSILON && (seen[0].offset + DISTANCE).abs() < 0.01);
        assert!(seen[1].opacity > 0.0 && seen[1].opacity < 1.0);
        assert!(
            (seen[2].opacity - 1.0).abs() < f32::EPSILON && seen[2].offset.abs() < f32::EPSILON
        );
        assert!(seen[3].opacity < f32::EPSILON, "it entered again");
    }

    #[test]
    fn reduced_motion_cuts_every_entrance_and_mark() {
        let ctx = Context::default();
        set_reduced(&ctx, true);

        frame(&ctx, 0.0, |ctx| {
            assert!(
                (enter(ctx, Id::unique("dialog"), Entrance::Lift).opacity - 1.0).abs()
                    < f32::EPSILON
            );
            assert!(fading(ctx, 0.0) < f32::EPSILON);
        });
        assert!(ctx.global_style().animation_time < f32::EPSILON);
    }

    #[test]
    fn a_mark_fades_over_its_two_seconds() {
        let ctx = Context::default();

        frame(&ctx, 1.0, |ctx| {
            assert!((fading(ctx, 0.0) - 0.5).abs() < 0.01);
            assert!(fading(ctx, -5.0) < f32::EPSILON);
        });
    }

    /// The test `scripts/motion.test.mjs` was: a duration typed anywhere else would escape
    /// the reduced-motion switch.
    #[test]
    fn no_duration_is_written_outside_this_module() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        for file in rust_files(&source) {
            if file.ends_with(Path::new("theme").join("motion.rs")) {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap();
            for (number, line) in text.lines().enumerate() {
                if writes_a_duration(line) {
                    offenders.push(format!("{}:{}", file.display(), number + 1));
                }
            }
        }

        assert!(
            offenders.is_empty(),
            "durations outside theme::motion: {offenders:?}"
        );
    }

    fn writes_a_duration(line: &str) -> bool {
        const CALLS: [&str; 5] = [
            "_with_time(",
            "request_repaint_after(",
            "Duration::from_millis(",
            "Duration::from_secs_f32(",
            "animation_time =",
        ];
        CALLS.iter().any(|call| {
            line.split(call).skip(1).any(|rest| {
                let arguments = rest.split(')').next().unwrap_or_default();
                arguments.split(',').any(|argument| {
                    let argument = argument.trim().trim_end_matches(';');
                    argument.trim_end_matches("_f32").parse::<f64>().is_ok()
                })
            })
        })
    }

    fn rust_files(directory: &Path) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files.extend(rust_files(&path));
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
        files
    }

    #[test]
    fn the_sweep_sees_a_typed_duration() {
        assert!(writes_a_duration("ctx.animate_bool_with_time(id, on, 0.2)"));
        assert!(writes_a_duration(
            "ctx.request_repaint_after(Duration::from_millis(300))"
        ));
        assert!(!writes_a_duration(
            "ctx.animate_bool_with_time(id, on, motion::FAST)"
        ));
    }
}
