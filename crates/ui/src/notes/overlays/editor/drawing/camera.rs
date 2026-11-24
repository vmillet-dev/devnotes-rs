//! Where the stage looks on the drawing: how large, and from where. A matter of the view: the scene
//! keeps its own units, and neither the note nor its pictures know of it.

use drawing::{Box, Point};
use egui::{Pos2, Rect, Vec2};

use crate::theme::tokens::gap;

pub(super) const LEAST: f32 = 0.1;
pub(super) const MOST: f32 = 8.0;
/// The keys and the buttons, a step each.
pub(super) const STEP: f32 = 1.25;
/// Left around a drawing fitted on the stage.
const MARGIN: f32 = gap::PAGE;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Camera {
    pub(super) zoom: f32,
    /// Where the scene's origin stands, from the stage's corner.
    pan: Vec2,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
        }
    }
}

impl Camera {
    pub(super) fn scene_at(&self, stage: Rect, at: Pos2) -> Point {
        let from = (at - stage.min - self.pan) / self.zoom;
        Point {
            x: f64::from(from.x),
            y: f64::from(from.y),
        }
    }

    pub(super) fn screen_at(&self, stage: Rect, at: Point) -> Pos2 {
        #[expect(clippy::cast_possible_truncation, reason = "a point on the screen")]
        let at = Vec2::new(at.x as f32, at.y as f32);
        stage.min + self.pan + at * self.zoom
    }

    /// A length of the scene, on the screen.
    pub(super) fn length(&self, length: f64) -> f32 {
        #[expect(clippy::cast_possible_truncation, reason = "a length on the screen")]
        let length = length as f32;
        length * self.zoom
    }

    pub(super) fn rect(&self, stage: Rect, of: &Box) -> Rect {
        let min = Point {
            x: of.left,
            y: of.top,
        };
        let size = Vec2::new(self.length(of.width), self.length(of.height));
        Rect::from_min_size(self.screen_at(stage, min), size)
    }

    pub(super) fn pan_by(&mut self, moved: Vec2) {
        self.pan += moved;
    }

    /// Closer or further by `factor`, what is under `at` staying under it.
    pub(super) fn zoom_at(&mut self, stage: Rect, at: Pos2, factor: f32) {
        let zoom = (self.zoom * factor).clamp(LEAST, MOST);
        let under = at - stage.min;
        self.pan = under - (under - self.pan) * (zoom / self.zoom);
        self.zoom = zoom;
    }

    /// Whole and centred on the stage, never larger than it is drawn.
    pub(super) fn fit(stage: Vec2, drawn: &Box) -> Self {
        #[expect(clippy::cast_possible_truncation, reason = "a size on the screen")]
        let (size, centre) = {
            let centre = drawn.centre();
            (
                Vec2::new(drawn.width as f32, drawn.height as f32).max(Vec2::splat(1.0)),
                Vec2::new(centre.x as f32, centre.y as f32),
            )
        };
        let room = (stage - Vec2::splat(2.0 * MARGIN)).max(Vec2::splat(1.0));
        let zoom = (room / size).min_elem().clamp(LEAST, 1.0);
        Self {
            zoom,
            pan: stage / 2.0 - centre * zoom,
        }
    }

    /// As a drawing opens: as it was drawn where the stage holds it, fitted where it does not.
    pub(super) fn opening(stage: Vec2, drawn: Option<&Box>) -> Self {
        let Some(drawn) = drawn else {
            return Self::default();
        };
        let room = Rect::from_min_size(Pos2::ZERO, stage);
        if room.contains_rect(Self::default().rect(room, drawn)) {
            Self::default()
        } else {
            Self::fit(stage, drawn)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAGE: Rect = Rect {
        min: Pos2::new(100.0, 50.0),
        max: Pos2::new(700.0, 450.0),
    };

    fn near(a: Pos2, b: Pos2) -> bool {
        (a - b).length() < 0.01
    }

    fn drawn(left: f64, top: f64, width: f64, height: f64) -> Box {
        Box {
            left,
            top,
            width,
            height,
        }
    }

    #[test]
    fn a_point_of_the_screen_comes_back_from_the_scene() {
        let mut camera = Camera::default();
        camera.zoom_at(STAGE, Pos2::new(300.0, 200.0), 2.5);
        camera.pan_by(Vec2::new(-40.0, 12.0));
        for at in [STAGE.min, STAGE.center(), Pos2::new(-900.0, 3.0)] {
            let back = camera.screen_at(STAGE, camera.scene_at(STAGE, at));
            assert!(near(back, at), "{back:?} for {at:?}");
        }
    }

    #[test]
    fn zooming_keeps_what_is_under_the_pointer_and_its_bounds() {
        let mut camera = Camera::default();
        camera.pan_by(Vec2::new(31.0, -17.0));
        let at = Pos2::new(420.0, 180.0);
        let under = camera.scene_at(STAGE, at);
        for factor in [1.25, 1.25, 0.3, 100.0, 0.0001] {
            camera.zoom_at(STAGE, at, factor);
            assert!(near(camera.screen_at(STAGE, under), at));
            assert!((LEAST..=MOST).contains(&camera.zoom));
        }
    }

    #[test]
    fn a_fitted_drawing_is_whole_and_centred_and_never_enlarged() {
        for of in [
            drawn(0.0, 0.0, 3_000.0, 200.0),
            drawn(-500.0, 900.0, 100.0, 2_000.0),
            drawn(10.0, 10.0, 20.0, 20.0),
        ] {
            let camera = Camera::fit(STAGE.size(), &of);
            let shown = camera.rect(STAGE, &of);
            assert!(STAGE.contains_rect(shown), "{shown:?}");
            assert!(near(shown.center(), STAGE.center()));
            assert!(camera.zoom <= 1.0);
        }
    }

    #[test]
    fn a_drawing_that_holds_opens_as_drawn_and_one_that_does_not_is_fitted() {
        let size = STAGE.size();
        assert_eq!(Camera::opening(size, None), Camera::default());
        let small = drawn(20.0, 20.0, 200.0, 100.0);
        assert_eq!(Camera::opening(size, Some(&small)), Camera::default());
        for beyond in [
            drawn(20.0, 20.0, 900.0, 100.0),
            drawn(-50.0, 0.0, 60.0, 60.0),
        ] {
            let camera = Camera::opening(size, Some(&beyond));
            assert!(STAGE.contains_rect(camera.rect(STAGE, &beyond)));
        }
    }
}
