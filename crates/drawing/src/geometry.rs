//! Where things go on the drawing (the reference's `drawing-geometry.ts`): an arrow's shaft and
//! head, where it leaves a shape it is bound to, a box dragged, a diamond's corners, a picture's
//! scale and frame.

/// A point of the scene, in its units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// An axis-aligned box, from its top-left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Box {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

/// An arrow's two ends, `[x1, y1, x2, y2]`, kept on it so it can be redrawn from them.
pub type Ends = [f64; 4];

/// How long the head of an arrow is, along it.
const HEAD: f64 = 12.0;

/// Room around what is drawn, in a picture taken of it.
pub const MARGIN: f64 = 24.0;

/// The shaft to the foot of the head, then the head as a closed triangle the fill paints; one
/// decimal kept, as the scene stores it.
pub fn arrow_path([x1, y1, x2, y2]: Ends) -> String {
    let length = (x2 - x1).hypot(y2 - y1);
    let length = if length == 0.0 { 1.0 } else { length };
    let (ux, uy) = ((x2 - x1) / length, (y2 - y1) / length);
    let reach = HEAD.min(length);
    let (foot_x, foot_y) = (x2 - ux * reach, y2 - uy * reach);
    let half = reach / 2.0;
    let (left_x, left_y) = (foot_x - uy * half, foot_y + ux * half);
    let (right_x, right_y) = (foot_x + uy * half, foot_y - ux * half);
    let at = |value: f64| number((value * 10.0).round() / 10.0);
    format!(
        "M {} {} L {} {} M {} {} L {} {} L {} {} Z",
        at(x1),
        at(y1),
        at(foot_x),
        at(foot_y),
        at(x2),
        at(y2),
        at(left_x),
        at(left_y),
        at(right_x),
        at(right_y)
    )
}

/// A number as JavaScript writes it: no `.0` on a whole one, no `-0`.
fn number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.fract() == 0.0 && value.abs() < 1e15 {
        #[expect(clippy::cast_possible_truncation, reason = "a whole number in range")]
        return format!("{}", value as i64);
    }
    format!("{value}")
}

impl Box {
    pub fn centre(&self) -> Point {
        Point {
            x: self.left + self.width / 2.0,
            y: self.top + self.height / 2.0,
        }
    }
}

/// Where a line from the box's centre toward a point leaves the box: an arrow stops there.
pub fn edge_point(bounds: &Box, toward: Point) -> Point {
    let centre = bounds.centre();
    let (dx, dy) = (toward.x - centre.x, toward.y - centre.y);
    if dx == 0.0 && dy == 0.0 {
        return centre;
    }
    let along = |half: f64, delta: f64| {
        if delta == 0.0 {
            f64::INFINITY
        } else {
            half / delta.abs()
        }
    };
    let scale = along(bounds.width / 2.0, dx).min(along(bounds.height / 2.0, dy));
    Point {
        x: centre.x + dx * scale,
        y: centre.y + dy * scale,
    }
}

/// An arrow's ends once the shapes it starts or ends on have moved: each bound end on the edge
/// of its shape, facing the other end — the other shape's centre when both are bound.
pub fn bound_ends(ends: Ends, from: Option<&Box>, to: Option<&Box>) -> Ends {
    let start = from.map_or(
        Point {
            x: ends[0],
            y: ends[1],
        },
        Box::centre,
    );
    let end = to.map_or(
        Point {
            x: ends[2],
            y: ends[3],
        },
        Box::centre,
    );
    let first = from.map_or(start, |bounds| edge_point(bounds, end));
    let last = to.map_or(end, |bounds| edge_point(bounds, start));
    [first.x, first.y, last.x, last.y]
}

/// The box a drag from one corner to the other covers, whichever way it went.
pub fn drag_box(from: Point, to: Point) -> Box {
    Box {
        left: from.x.min(to.x),
        top: from.y.min(to.y),
        width: (to.x - from.x).abs(),
        height: (to.y - from.y).abs(),
    }
}

pub fn diamond_points(width: f64, height: f64) -> [Point; 4] {
    [
        Point {
            x: width / 2.0,
            y: 0.0,
        },
        Point {
            x: width,
            y: height / 2.0,
        },
        Point {
            x: width / 2.0,
            y: height,
        },
        Point {
            x: 0.0,
            y: height / 2.0,
        },
    ]
}

/// The scale a picture is taken at: its longest side brought to `longest`, never enlarged past
/// `most`.
pub fn picture_scale(bounds: &Box, longest: f64, most: f64) -> f64 {
    let side = bounds.width.max(bounds.height).max(1.0);
    most.min(longest / side)
}

/// The box around every box, with room around it; `None` when nothing is drawn.
pub fn scene_box(boxes: &[Box], margin: f64) -> Option<Box> {
    let left = boxes.iter().map(|b| b.left).reduce(f64::min)? - margin;
    let top = boxes.iter().map(|b| b.top).reduce(f64::min)? - margin;
    let right = boxes.iter().map(|b| b.left + b.width).reduce(f64::max)? + margin;
    let bottom = boxes.iter().map(|b| b.top + b.height).reduce(f64::max)? + margin;
    Some(Box {
        left,
        top,
        width: right - left,
        height: bottom - top,
    })
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "whole numbers, exact in a double")]
mod tests {
    use super::*;

    const BOX: Box = Box {
        left: 0.0,
        top: 0.0,
        width: 100.0,
        height: 50.0,
    };

    fn at(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    #[test]
    fn an_arrow_is_its_shaft_to_the_head_then_the_head_closed() {
        assert_eq!(
            arrow_path([0.0, 0.0, 100.0, 0.0]),
            "M 0 0 L 88 0 M 100 0 L 88 6 L 88 -6 Z"
        );
        assert_eq!(
            arrow_path([0.0, 0.0, 0.0, 4.0]),
            "M 0 0 L 0 0 M 0 4 L -2 0 L 2 0 Z"
        );
        assert_eq!(
            arrow_path([5.0, 5.0, 5.0, 5.0]),
            "M 5 5 L 5 5 M 5 5 L 5 5 L 5 5 Z"
        );
    }

    #[test]
    fn a_line_from_the_centre_stops_where_it_leaves_the_box() {
        assert_eq!(edge_point(&BOX, at(200.0, 25.0)), at(100.0, 25.0));
        assert_eq!(edge_point(&BOX, at(50.0, -100.0)), at(50.0, 0.0));
        assert_eq!(edge_point(&BOX, at(100.0, 75.0)), at(75.0, 50.0));
        assert_eq!(edge_point(&BOX, at(150.0, 75.0)), at(100.0, 50.0));
        assert_eq!(edge_point(&BOX, at(50.0, 25.0)), at(50.0, 25.0));
    }

    #[test]
    fn each_bound_end_is_on_its_shape_facing_the_other() {
        let right = Box { left: 200.0, ..BOX };
        assert_eq!(
            bound_ends([0.0; 4], Some(&BOX), Some(&right)),
            [100.0, 25.0, 200.0, 25.0]
        );
        assert_eq!(
            bound_ends([0.0, 0.0, 50.0, 300.0], Some(&BOX), None),
            [50.0, 50.0, 50.0, 300.0]
        );
        assert_eq!(
            bound_ends([400.0, 25.0, 0.0, 0.0], None, Some(&right)),
            [400.0, 25.0, 300.0, 25.0]
        );
        assert_eq!(
            bound_ends([1.0, 2.0, 3.0, 4.0], None, None),
            [1.0, 2.0, 3.0, 4.0]
        );
    }

    #[test]
    fn a_drag_reads_whichever_way_it_went_and_a_diamond_has_its_corners_mid_side() {
        assert_eq!(
            drag_box(at(50.0, 40.0), at(10.0, 60.0)),
            Box {
                left: 10.0,
                top: 40.0,
                width: 40.0,
                height: 20.0
            }
        );
        assert_eq!(
            diamond_points(40.0, 20.0),
            [at(20.0, 0.0), at(40.0, 10.0), at(20.0, 20.0), at(0.0, 10.0)]
        );
    }

    #[test]
    fn a_picture_is_scaled_and_framed() {
        let wide = Box {
            width: 800.0,
            height: 200.0,
            ..BOX
        };
        assert!((picture_scale(&wide, 400.0, 2.0) - 0.5).abs() < f64::EPSILON);
        assert!((picture_scale(&BOX, 1600.0, 2.0) - 2.0).abs() < f64::EPSILON);
        let small = Box {
            left: 150.0,
            top: -20.0,
            width: 10.0,
            height: 10.0,
        };
        assert_eq!(
            scene_box(&[BOX, small], 10.0),
            Some(Box {
                left: -10.0,
                top: -30.0,
                width: 180.0,
                height: 90.0
            })
        );
        assert_eq!(scene_box(&[], MARGIN), None);
    }
}
