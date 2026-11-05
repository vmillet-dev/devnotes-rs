//! The Note's headings as a map (the reference's `mind-map`): around its title, each ring a
//! level deeper, each heading given a share of its parent's sector by the leaves it holds. A
//! Note under one heading has that heading in the middle. Placed in percent of the screen, so
//! it reads wide; a heading pressed opens the slide it is on.

use std::f64::consts::PI;

use egui::text::{LayoutJob, TextWrapping};
use egui::{CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, TextFormat, Ui, pos2, vec2};
use engine::notes::model::MapNode;

use crate::theme::palette;

/// How far from the edge the outermost headings sit, in percent.
const MARGIN: f64 = 9.0;

/// A heading placed on the map, in percent of its width and height.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Placed {
    pub key: String,
    pub text: String,
    /// The body's slide it is on; `None` for the Note's title, which is the cover.
    pub slide: Option<u32>,
    pub depth: usize,
    pub left: f64,
    pub top: f64,
}

/// The headings, and each link as the indices of its two ends.
pub(super) struct Layout {
    pub nodes: Vec<Placed>,
    pub links: Vec<(usize, usize)>,
}

/// A heading before it is stretched to the screen: on its ring, in units of a ring.
struct Point {
    key: String,
    text: String,
    slide: Option<u32>,
    depth: usize,
    x: f64,
    y: f64,
}

pub(super) fn layout(title: &str, map: &[MapNode]) -> Layout {
    let (root, children) = match map {
        [only] => (
            point("root", &only.text, Some(only.slide)),
            &only.children[..],
        ),
        _ => (point("root", title, None), map),
    };
    let mut points = vec![root];
    let mut links = Vec::new();
    place(
        children,
        (-PI / 2.0, 3.0 * PI / 2.0),
        1,
        0,
        &mut points,
        &mut links,
    );

    let span_x = points.iter().map(|point| point.x.abs()).fold(0.0, f64::max);
    let span_y = points.iter().map(|point| point.y.abs()).fold(0.0, f64::max);
    let scale = |value: f64, span: f64| {
        if span == 0.0 {
            50.0
        } else {
            50.0 + value / span * (50.0 - MARGIN)
        }
    };
    let nodes = points
        .into_iter()
        .map(|point| Placed {
            left: scale(point.x, span_x),
            top: scale(point.y, span_y),
            key: point.key,
            text: point.text,
            slide: point.slide,
            depth: point.depth,
        })
        .collect();
    Layout { nodes, links }
}

fn point(key: &str, text: &str, slide: Option<u32>) -> Point {
    Point {
        key: key.to_owned(),
        text: text.to_owned(),
        slide,
        depth: 0,
        x: 0.0,
        y: 0.0,
    }
}

/// `nodes` share the sector `(from, to)` of the point at `parent`, a ring further out.
fn place(
    nodes: &[MapNode],
    (from, to): (f64, f64),
    depth: usize,
    parent: usize,
    points: &mut Vec<Point>,
    links: &mut Vec<(usize, usize)>,
) {
    let total: f64 = nodes.iter().map(leaves).sum();
    let mut angle = from;
    #[expect(clippy::cast_precision_loss, reason = "a ring's number")]
    let ring = depth as f64;
    for (index, node) in nodes.iter().enumerate() {
        let span = (to - from) * leaves(node) / total;
        let middle = angle + span / 2.0;
        let at = points.len();
        points.push(Point {
            key: format!("{}.{index}", points[parent].key),
            text: node.text.clone(),
            slide: Some(node.slide),
            depth,
            x: middle.cos() * ring,
            y: middle.sin() * ring,
        });
        links.push((parent, at));
        place(
            &node.children,
            (angle, angle + span),
            depth + 1,
            at,
            points,
            links,
        );
        angle += span;
    }
}

fn leaves(node: &MapNode) -> f64 {
    if node.children.is_empty() {
        1.0
    } else {
        node.children.iter().map(leaves).sum()
    }
}

/// The map over `rect`: the links, then each heading a button over them. Where the heading
/// pressed is in the presentation: 0 for the title, the cover.
pub(super) fn show(ui: &mut Ui, rect: Rect, title: &str, map: &[MapNode]) -> Option<usize> {
    let p = palette(ui.ctx());
    let placed = layout(title, map);
    #[expect(clippy::cast_possible_truncation, reason = "a percentage")]
    let at = |node: &Placed| {
        pos2(
            rect.left() + rect.width() * (node.left / 100.0) as f32,
            rect.top() + rect.height() * (node.top / 100.0) as f32,
        )
    };
    for (from, to) in &placed.links {
        let ends = [at(&placed.nodes[*from]), at(&placed.nodes[*to])];
        ui.painter().line_segment(ends, Stroke::new(1.5, p.line));
    }
    let mut chosen = None;
    for node in &placed.nodes {
        if heading(ui, node, at(node)) {
            chosen = Some(node.slide.map_or(0, |slide| slide as usize + 1));
        }
    }
    chosen
}

/// A heading as its depth draws it; whether it was pressed.
fn heading(ui: &mut Ui, node: &Placed, centre: egui::Pos2) -> bool {
    let p = palette(ui.ctx());
    let (size, weight, colour, widest, padding) = match node.depth {
        0 => (22.0, 600.0, p.text_0, 320.0, vec2(20.0, 12.0)),
        1 => (17.0, 600.0, p.text_0, 220.0, vec2(14.0, 8.0)),
        2 => (15.0, 400.0, p.text_0, 220.0, vec2(14.0, 8.0)),
        _ => (13.0, 400.0, p.text_1, 220.0, vec2(14.0, 8.0)),
    };
    let mut format = TextFormat::simple(FontId::proportional(size), colour);
    format.coords.push(*b"wght", weight);
    let mut job = LayoutJob::single_section(node.text.clone(), format);
    job.wrap = TextWrapping::truncate_at_width(widest - 2.0 * padding.x);
    let galley = ui.painter().layout_job(job);
    let rect = Rect::from_center_size(centre, galley.size() + 2.0 * padding);
    let response = crate::widgets::region(ui, rect, ("map-node", &node.key), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &node.text));
    crate::widgets::address(ui, &response, "presentation-map-node");
    let hovered = response.hovered() || response.has_focus();
    let ink = if hovered { p.amber_text } else { colour };
    let edge = if node.depth == 0 || hovered {
        p.amber_edge
    } else {
        p.line
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        p.bg_2,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    ui.painter()
        .galley_with_override_text_color(rect.min + padding, galley, ink);
    response.on_hover_text(&node.text).clicked()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn map_of(value: &Value) -> Vec<MapNode> {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|node| MapNode {
                text: node["text"].as_str().unwrap().to_owned(),
                slide: u32::try_from(node["slide"].as_u64().unwrap()).unwrap(),
                children: map_of(&node["children"]),
            })
            .collect()
    }

    fn close(a: f64, b: &Value) -> bool {
        (a - b.as_f64().unwrap()).abs() < 1e-9
    }

    #[test]
    fn every_outline_of_the_golden_corpus_is_placed_as_the_reference_placed_it() {
        let cases: Value =
            serde_json::from_str(include_str!("../../../../../../golden/mind-map/cases.json"))
                .unwrap();
        for case in cases.as_array().unwrap() {
            let title = case["input"]["title"].as_str().unwrap();
            let placed = layout(title, &map_of(&case["input"]["map"]));
            let nodes = case["output"]["nodes"].as_array().unwrap();
            assert_eq!(placed.nodes.len(), nodes.len(), "{title}");
            for (node, expected) in placed.nodes.iter().zip(nodes) {
                assert_eq!(node.key, expected["key"].as_str().unwrap(), "{title}");
                assert_eq!(node.text, expected["text"].as_str().unwrap());
                assert_eq!(node.slide.map(u64::from), expected["slide"].as_u64());
                assert_eq!(node.depth as u64, expected["depth"].as_u64().unwrap());
                assert!(close(node.left, &expected["left"]), "{title} {}", node.key);
                assert!(close(node.top, &expected["top"]), "{title} {}", node.key);
            }
            let links = case["output"]["links"].as_array().unwrap();
            assert_eq!(placed.links.len(), links.len(), "{title}");
            for ((from, to), expected) in placed.links.iter().zip(links) {
                assert_eq!(
                    placed.nodes[*from].key,
                    expected["from"]["key"].as_str().unwrap()
                );
                assert_eq!(
                    placed.nodes[*to].key,
                    expected["to"]["key"].as_str().unwrap()
                );
            }
        }
    }
}
