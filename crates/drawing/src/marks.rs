//! What a drawing puts on screen, flattened on the scene's plane: lines to stroke or fill and
//! words to write. The editor paints these as they are; curves are already points.

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::geometry::{Point, path_box, points_box};
use crate::scene::{Drawing, Hue, Kind, Object, PathCommand};
use crate::text::{Fonts, first_baseline, line_step, wrap};

const ELLIPSE_STEPS: u32 = 48;
const CORNER_STEPS: u32 = 6;
const CURVE_STEPS: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    None,
    /// The hue's soft tint, as a shape.
    Tint,
    /// The stroke's own colour, as an arrow's head.
    Stroke,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mark {
    Line {
        points: Vec<Point>,
        closed: bool,
        fill: Fill,
        hue: Hue,
        width: f64,
    },
    /// One wrapped line, from where its baseline starts.
    Words {
        text: String,
        at: Point,
        /// In degrees, clockwise.
        angle: f64,
        size: f64,
        hue: Hue,
    },
}

/// Every object in stacking order; `skip` is a text typed into, which its field shows.
pub fn marks(drawing: &Drawing, fonts: &Fonts, skip: Option<usize>) -> Vec<Mark> {
    let mut marks = Vec::new();
    for (index, object) in drawing.objects.iter().enumerate() {
        if Some(index) != skip {
            add(&mut marks, object, Plane::IDENTITY, fonts);
        }
    }
    marks
}

/// From an object's own plane to the one it sits on.
#[derive(Debug, Clone, Copy)]
struct Plane {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Plane {
    const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// Fabric's order: translate, rotate, scale, skew.
    fn of(object: &Object) -> Self {
        let centre = object.centre();
        let (sin, cos) = object.angle().to_radians().sin_cos();
        let (flip_x, flip_y) = object.flipped();
        let mirror = |flipped: bool| if flipped { -1.0 } else { 1.0 };
        let (scale_x, scale_y) = (
            object.scale_x() * mirror(flip_x),
            object.scale_y() * mirror(flip_y),
        );
        let (skew_x, skew_y) = object.skew();
        let turned = Self {
            a: cos * scale_x,
            b: sin * scale_x,
            c: -sin * scale_y,
            d: cos * scale_y,
            e: centre.x,
            f: centre.y,
        };
        turned
            .then(Self {
                c: skew_x.to_radians().tan(),
                ..Self::IDENTITY
            })
            .then(Self {
                b: skew_y.to_radians().tan(),
                ..Self::IDENTITY
            })
    }

    /// This plane, with `inner` inside it.
    fn then(self, inner: Self) -> Self {
        Self {
            a: self.a * inner.a + self.c * inner.b,
            b: self.b * inner.a + self.d * inner.b,
            c: self.a * inner.c + self.c * inner.d,
            d: self.b * inner.c + self.d * inner.d,
            e: self.a * inner.e + self.c * inner.f + self.e,
            f: self.b * inner.e + self.d * inner.f + self.f,
        }
    }

    fn apply(self, at: Point) -> Point {
        Point {
            x: self.a * at.x + self.c * at.y + self.e,
            y: self.b * at.x + self.d * at.y + self.f,
        }
    }

    fn scale(self) -> f64 {
        self.a.hypot(self.b)
    }

    fn angle(self) -> f64 {
        self.b.atan2(self.a).to_degrees()
    }
}

fn add(marks: &mut Vec<Mark>, object: &Object, outer: Plane, fonts: &Fonts) {
    let plane = outer.then(Plane::of(object));
    match object.kind() {
        Kind::Other => {}
        Kind::Group => {
            for child in &object.children {
                add(marks, child, plane, fonts);
            }
        }
        Kind::Textbox => add_text(marks, object, plane, fonts),
        Kind::Path => add_path(marks, object, plane),
        Kind::Rect | Kind::Ellipse | Kind::Polygon => {
            let outline = match object.kind() {
                Kind::Rect => rectangle(object),
                Kind::Ellipse => ellipse(object),
                _ => polygon(object),
            };
            marks.push(Mark::Line {
                points: outline.into_iter().map(|at| plane.apply(at)).collect(),
                closed: true,
                fill: Fill::Tint,
                hue: object.hue(),
                width: stroke_width(object, plane),
            });
        }
    }
}

fn stroke_width(object: &Object, plane: Plane) -> f64 {
    if object.stroke_uniform() {
        object.stroke_width()
    } else {
        object.stroke_width() * plane.scale()
    }
}

/// Round its centre, each corner an arc when it has a radius.
fn rectangle(object: &Object) -> Vec<Point> {
    let (half_width, half_height) = (object.width() / 2.0, object.height() / 2.0);
    let radius = object.corner_radius().min(half_width).min(half_height);
    let corners = [
        (half_width - radius, -half_height + radius, -FRAC_PI_2),
        (half_width - radius, half_height - radius, 0.0),
        (-half_width + radius, half_height - radius, FRAC_PI_2),
        (-half_width + radius, -half_height + radius, 2.0 * FRAC_PI_2),
    ];
    let mut outline = Vec::new();
    for (x, y, from) in corners {
        for step in 0..=CORNER_STEPS {
            let turned = from + FRAC_PI_2 * f64::from(step) / f64::from(CORNER_STEPS);
            outline.push(Point {
                x: x + radius * turned.cos(),
                y: y + radius * turned.sin(),
            });
        }
    }
    outline
}

fn ellipse(object: &Object) -> Vec<Point> {
    let (rx, ry) = (object.width() / 2.0, object.height() / 2.0);
    (0..ELLIPSE_STEPS)
        .map(|step| {
            let turned = TAU * f64::from(step) / f64::from(ELLIPSE_STEPS);
            Point {
                x: rx * turned.cos(),
                y: ry * turned.sin(),
            }
        })
        .collect()
}

fn polygon(object: &Object) -> Vec<Point> {
    let points = object.points();
    let middle = points_box(&points).map_or(Point { x: 0.0, y: 0.0 }, |bounds| bounds.centre());
    points
        .iter()
        .map(|point| Point {
            x: point.x - middle.x,
            y: point.y - middle.y,
        })
        .collect()
}

/// Each piece of it a line of its own: an arrow is a shaft, then a head that closes.
fn add_path(marks: &mut Vec<Mark>, object: &Object, plane: Plane) {
    let commands = object.path();
    let middle = path_box(&commands).map_or(Point { x: 0.0, y: 0.0 }, |bounds| bounds.centre());
    let on_plane = |at: Point| {
        plane.apply(Point {
            x: at.x - middle.x,
            y: at.y - middle.y,
        })
    };
    let arrow = object.ends().is_some();
    let mut flush = |points: &mut Vec<Point>, closed: bool| {
        if points.len() > 1 {
            marks.push(Mark::Line {
                points: std::mem::take(points),
                closed,
                fill: if closed && arrow {
                    Fill::Stroke
                } else {
                    Fill::None
                },
                hue: object.hue(),
                width: stroke_width(object, plane),
            });
        }
        points.clear();
    };
    let mut piece: Vec<Point> = Vec::new();
    let mut pen = Point { x: 0.0, y: 0.0 };
    for command in commands.iter().copied() {
        match command {
            PathCommand::Move(to) => {
                flush(&mut piece, false);
                piece.push(on_plane(to));
                pen = to;
            }
            PathCommand::Line(to) => {
                piece.push(on_plane(to));
                pen = to;
            }
            PathCommand::Quad(control, to) => {
                for step in 1..=CURVE_STEPS {
                    let t = f64::from(step) / f64::from(CURVE_STEPS);
                    piece.push(on_plane(along_curve(pen, control, to, t)));
                }
                pen = to;
            }
            PathCommand::Close => flush(&mut piece, true),
        }
    }
    flush(&mut piece, false);
}

fn along_curve(from: Point, control: Point, to: Point, t: f64) -> Point {
    let rest = 1.0 - t;
    let at = |from: f64, control: f64, to: f64| {
        rest * rest * from + 2.0 * rest * t * control + t * t * to
    };
    Point {
        x: at(from.x, control.x, to.x),
        y: at(from.y, control.y, to.y),
    }
}

fn add_text(marks: &mut Vec<Mark>, object: &Object, plane: Plane, fonts: &Fonts) {
    let font_size = object.font_size();
    let text = object.text().unwrap_or_default();
    let step = line_step(font_size, object.line_height());
    let left = -object.width() / 2.0;
    let mut baseline = -object.height() / 2.0 + first_baseline(font_size);
    for line in wrap(text, object.width(), font_size, fonts) {
        marks.push(Mark::Words {
            text: line,
            at: plane.apply(Point {
                x: left,
                y: baseline,
            }),
            angle: plane.angle(),
            size: font_size * plane.scale(),
            hue: object.hue(),
        });
        baseline += step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::tests::fonts;

    fn marks_of(objects: &str) -> Vec<Mark> {
        let body = format!(r#"{{"scene":{{"objects":[{objects}]}}}}"#);
        marks(&Drawing::read(&body).unwrap(), &fonts(), None)
    }

    fn near(a: Point, x: f64, y: f64) -> bool {
        (a.x - x).abs() < 1e-6 && (a.y - y).abs() < 1e-6
    }

    fn line(mark: &Mark) -> (&[Point], bool, Fill) {
        match mark {
            Mark::Line {
                points,
                closed,
                fill,
                ..
            } => (points, *closed, *fill),
            Mark::Words { .. } => panic!("words"),
        }
    }

    #[test]
    fn a_rectangle_is_closed_tinted_and_turned_round_its_centre() {
        let marks = marks_of(
            r#"{"type":"Rect","left":100,"top":50,"width":20,"height":10,"angle":90,"strokeWidth":2}"#,
        );
        let (points, closed, fill) = line(&marks[0]);

        assert!(closed);
        assert_eq!(fill, Fill::Tint);
        assert!(points.iter().any(|at| near(*at, 105.0, 60.0)), "{points:?}");
    }

    #[test]
    fn a_rounded_corner_stays_inside_the_square_one() {
        let marks = marks_of(r#"{"type":"Rect","width":20,"height":20,"rx":5}"#);
        let (points, ..) = line(&marks[0]);

        assert!(
            points
                .iter()
                .all(|at| at.x.hypot(at.y) < 10.0 * 2f64.sqrt() - 1.0)
        );
    }

    #[test]
    fn an_ellipse_and_a_diamond_reach_their_sides() {
        let marks = marks_of(
            r#"{"type":"Ellipse","width":40,"height":20},
               {"type":"Polygon","width":40,"height":20,"points":[{"x":20,"y":0},{"x":40,"y":10},{"x":20,"y":20},{"x":0,"y":10}]}"#,
        );

        assert!(line(&marks[0]).0.iter().any(|at| near(*at, 20.0, 0.0)));
        assert!(near(line(&marks[1]).0[0], 0.0, -10.0));
    }

    #[test]
    fn an_arrow_is_its_shaft_then_its_head_filled_with_the_stroke() {
        let marks = marks_of(
            r#"{"type":"Path","ends":[0,0,20,0],"path":[["M",0,0],["L",10,0],["M",20,0],["L",10,5],["L",10,-5],["Z"]]}"#,
        );

        assert_eq!(marks.len(), 2);
        assert_eq!(line(&marks[0]).2, Fill::None);
        assert!(!line(&marks[0]).1);
        assert_eq!(line(&marks[1]).2, Fill::Stroke);
    }

    #[test]
    fn a_pen_line_is_open_and_its_curves_are_points() {
        let marks = marks_of(r#"{"type":"Path","path":[["M",0,0],["Q",10,20,20,0],["L",30,0]]}"#);
        let (points, closed, fill) = line(&marks[0]);

        assert!(!closed);
        assert_eq!(fill, Fill::None);
        assert_eq!(points.len(), usize::try_from(CURVE_STEPS).unwrap() + 2);
    }

    #[test]
    fn a_stroke_grows_with_its_object_unless_it_keeps_its_width() {
        let width = |object: &str| match &marks_of(object)[0] {
            Mark::Line { width, .. } => *width,
            Mark::Words { .. } => panic!("words"),
        };

        assert!(
            (width(r#"{"type":"Rect","strokeWidth":2,"scaleX":3,"scaleY":3}"#) - 6.0).abs() < 1e-9
        );
        assert!(
            (width(r#"{"type":"Rect","strokeWidth":2,"scaleX":3,"strokeUniform":true}"#) - 2.0)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn a_text_is_its_lines_each_from_its_baseline() {
        let marks = marks_of(
            r#"{"type":"Textbox","left":90,"top":0,"width":180,"height":43.9344,"fontSize":18,"text":"one\ntwo"}"#,
        );

        let Mark::Words { text, at, size, .. } = &marks[1] else {
            panic!("a line");
        };
        assert_eq!(text, "two");
        assert!((at.x).abs() < 1e-9);
        assert!((at.y - 17.45).abs() < 0.01, "{at:?}");
        assert!((size - 18.0).abs() < 1e-9);
    }

    #[test]
    fn a_group_carries_its_children_and_a_text_typed_into_is_left_to_its_field() {
        let group = marks_of(
            r#"{"type":"Group","left":100,"top":100,"objects":[{"type":"Ellipse","left":10,"top":0,"width":2,"height":2}]}"#,
        );
        assert!(line(&group[0]).0.iter().any(|at| near(*at, 111.0, 100.0)));

        let body = r#"{"scene":{"objects":[{"type":"Textbox","text":"a"},{"type":"Circle"}]}}"#;
        let drawing = Drawing::read(body).unwrap();
        assert!(marks(&drawing, &fonts(), Some(0)).is_empty());
    }

    #[test]
    fn a_flip_and_a_skew_reach_the_plane() {
        let flipped = marks_of(
            r#"{"type":"Path","flipX":true,"left":0,"top":0,"path":[["M",0,0],["L",10,0]]}"#,
        );
        assert!(near(line(&flipped[0]).0[0], 5.0, 0.0));

        let skewed =
            marks_of(r#"{"type":"Polygon","skewX":45,"points":[{"x":0,"y":0},{"x":0,"y":10}]}"#);
        assert!(near(line(&skewed[0]).0[1], 5.0, 5.0));
    }
}
