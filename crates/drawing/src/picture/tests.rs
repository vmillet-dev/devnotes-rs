use super::*;
use crate::text::tests::fonts;

fn palette() -> Palette {
    let each = |prefix: &str| Hue::ALL.map(|hue| format!("{prefix}-{}", hue.name()));
    Palette {
        stroke: each("stroke"),
        fill: each("fill"),
        paper: "#101010".to_owned(),
    }
}

fn colours() -> Palette {
    let each = |colour: &str| [colour; 6].map(str::to_owned);
    Palette {
        stroke: each("#e5e7eb"),
        fill: each("rgba(96, 165, 250, 0.14)"),
        paper: "#101010".to_owned(),
    }
}

fn scene(objects: &str) -> Drawing {
    Drawing::read(&format!(r#"{{"scene":{{"objects":[{objects}]}}}}"#)).unwrap()
}

fn golden() -> Drawing {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../../golden/fabric/cases.json")).unwrap();
    Drawing::read(cases[0]["output"].as_str().unwrap()).unwrap()
}

fn svg_of(objects: &str) -> String {
    svg(&scene(objects), &palette(), &fonts()).unwrap()
}

fn size_of(png: &[u8]) -> (u32, u32) {
    let pixmap = Pixmap::decode_png(png).unwrap();
    (pixmap.width(), pixmap.height())
}

#[test]
fn what_fabric_stored_is_drawn_whole() {
    let svg = svg(&golden(), &colours(), &fonts()).unwrap();

    for tag in ["<ellipse", "<polygon", "<path", "<text", "<g transform"] {
        assert!(svg.contains(tag), "{tag}");
    }
    assert!(svg.contains(">sits in front of «</text>"), "{svg}");
    assert!(usvg::Tree::from_str(&svg, &usvg::Options::default()).is_ok());
}

#[test]
fn nothing_drawn_has_no_picture() {
    let empty = scene("");

    assert_eq!(svg(&empty, &palette(), &fonts()), None);
    assert_eq!(pictures(&empty, &palette(), &fonts()), None);
}

#[test]
fn the_frame_is_what_is_drawn_with_room_on_its_paper() {
    let svg =
        svg_of(r#"{"type":"Rect","left":100,"top":50,"width":100,"height":50,"strokeWidth":0}"#);

    assert!(
        svg.contains(r#"viewBox="26 1 148 98" width="148" height="98""#),
        "{svg}"
    );
    assert!(svg.contains(r##"<rect x="25" y="0" width="150" height="100" fill="#101010"/>"##));
}

#[test]
fn each_shape_is_drawn_round_its_centre_in_its_hues_colours() {
    let svg = svg_of(
        r#"{"type":"Rect","hue":"blue","left":10,"top":20,"width":100,"height":50,"rx":6,"ry":6,"strokeWidth":2},
           {"type":"Ellipse","hue":"red","left":0,"top":0,"width":40,"height":20,"strokeWidth":2},
           {"type":"Polygon","left":0,"top":0,"width":40,"height":20,"points":[{"x":20,"y":0},{"x":40,"y":10},{"x":20,"y":20},{"x":0,"y":10}]}"#,
    );

    assert!(svg.contains(r#"<rect x="-50" y="-25" width="100" height="50" rx="6" ry="6" transform="translate(10 20)"#), "{svg}");
    assert!(svg.contains(r#"fill="fill-blue" stroke="stroke-blue" stroke-width="2""#));
    assert!(svg.contains(r#"<ellipse rx="20" ry="10""#));
    assert!(svg.contains(r#"fill="fill-red""#));
    assert!(
        svg.contains(r#"<polygon points="0,-10 20,0 0,10 -20,0""#),
        "{svg}"
    );
    assert!(svg.contains(r#"fill="fill-ink""#));
}

#[test]
fn an_arrow_is_filled_with_its_stroke_and_a_pen_line_is_not_filled() {
    let svg = svg_of(
        r#"{"type":"Path","hue":"red","ends":[0,0,10,0],"path":[["M",0,0],["L",10,0],["Z"]]},
           {"type":"Path","hue":"blue","strokeLineCap":"round","strokeLineJoin":"round","path":[["M",0,0],["Q",10,20,20,0]]}"#,
    );

    assert!(svg.contains(r#"<path d="M -5 0 L 5 0 Z""#), "{svg}");
    assert!(svg.contains(r#"fill="stroke-red" stroke="stroke-red""#));
    assert!(svg.contains(r#"<path d="M -10 -5 Q 0 15 10 -5""#), "{svg}");
    assert!(svg.contains(r#"fill="none" stroke="stroke-blue""#));
    assert!(svg.contains(r#"stroke-linecap="round" stroke-linejoin="round""#));
}

#[test]
fn a_stroke_that_keeps_its_width_has_its_shape_scaled_instead() {
    let svg = svg_of(
        r#"{"type":"Rect","width":100,"height":50,"scaleX":2,"scaleY":0.5,"strokeUniform":true,"strokeWidth":2},
           {"type":"Rect","width":100,"height":50,"scaleX":2,"scaleY":0.5,"strokeWidth":2}"#,
    );

    assert!(svg.contains(r#"width="200" height="25""#), "{svg}");
    assert!(svg.contains("scale(1 1)"));
    assert!(svg.contains(r#"width="100" height="50""#));
    assert!(svg.contains("scale(2 0.5)"));
}

#[test]
fn a_transform_reads_in_fabrics_order_flips_counted() {
    let svg = svg_of(
        r#"{"type":"Ellipse","left":1,"top":2,"width":10,"height":10,"angle":30,"scaleX":2,"flipY":true,"skewX":5,"skewY":6}"#,
    );

    assert!(
        svg.contains("translate(1 2) rotate(30) scale(2 -1) skewX(5) skewY(6)"),
        "{svg}"
    );
}

#[test]
fn a_text_is_written_as_the_lines_it_wraps_to_escaped() {
    let svg = svg_of(
        r#"{"type":"Textbox","hue":"amber","left":90,"top":0,"width":180,"height":43.9344,"fontSize":18,"text":"a < b & c\nnext"}"#,
    );

    assert!(svg.contains(r#"fill="stroke-amber""#));
    assert!(svg.contains(r#"font-size="18""#));
    assert!(svg.contains(">a &lt; b &amp; c</text>"), "{svg}");
    assert!(svg.contains(">next</text>"));
    assert!(svg.contains(r#"<text x="-90" y="-6.14268""#), "{svg}");
}

#[test]
fn a_group_holds_its_children_and_an_unknown_type_is_not_drawn() {
    let svg = svg_of(
        r#"{"type":"Group","left":50,"top":50,"width":40,"height":40,"angle":10,"objects":[{"type":"Ellipse","left":-5,"top":0,"width":10,"height":10}]},
           {"type":"Circle","radius":3}"#,
    );

    assert!(
        svg.contains(
            r#"<g transform="translate(50 50) rotate(10) scale(1 1) skewX(0) skewY(0)"><ellipse"#
        ),
        "{svg}"
    );
    assert!(!svg.to_lowercase().contains("circle"));
}

#[test]
fn a_rotated_object_covers_more_than_its_size_and_a_uniform_stroke_is_not_scaled() {
    let rotated = bounds(&scene(
        r#"{"type":"Rect","left":0,"top":0,"width":100,"height":100,"angle":45,"strokeWidth":0}"#,
    ).objects[0]);
    assert!((rotated.width - 141.421_356).abs() < 1e-3);

    let uniform = bounds(&scene(
        r#"{"type":"Rect","width":10,"height":10,"scaleX":3,"scaleY":3,"strokeWidth":2,"strokeUniform":true}"#,
    ).objects[0]);
    assert!((uniform.width - 32.0).abs() < 1e-9);
    let scaled = bounds(
        &scene(r#"{"type":"Rect","width":10,"height":10,"scaleX":3,"scaleY":3,"strokeWidth":2}"#)
            .objects[0],
    );
    assert!((scaled.width - 36.0).abs() < 1e-9);
}

#[test]
fn a_card_and_a_note_are_given_their_sizes() {
    let wide =
        scene(r#"{"type":"Rect","left":1500,"top":100,"width":3000,"height":100,"strokeWidth":0}"#);

    let pictures = pictures(&wide, &colours(), &fonts()).unwrap();

    assert_eq!(size_of(&pictures.thumbnail).0, 480);
    assert_eq!(size_of(&pictures.picture).0, 1600);
    assert_eq!(
        pictures.picture,
        export_png(&wide, &colours(), &fonts()).unwrap()
    );
}

#[test]
fn a_small_drawing_is_never_enlarged_past_a_cards_pixels() {
    let drawing =
        scene(r#"{"type":"Rect","left":10,"top":10,"width":20,"height":20,"strokeWidth":0}"#);

    let pictures = pictures(&drawing, &colours(), &fonts()).unwrap();

    assert_eq!(size_of(&pictures.thumbnail), (68, 68));
    assert_eq!(size_of(&pictures.picture), (136, 136));
}

#[test]
fn the_paper_reaches_every_edge_of_a_picture() {
    let drawing = scene(
        r#"{"type":"Rect","left":10.25,"top":10.25,"width":20.5,"height":20.5,"strokeWidth":0}"#,
    );

    let picture = pictures(&drawing, &colours(), &fonts()).unwrap().picture;

    let pixmap = Pixmap::decode_png(&picture).unwrap();
    let (right, bottom) = (pixmap.width() - 1, pixmap.height() - 1);
    for (x, y) in [(0, 0), (right, 0), (0, bottom), (right, bottom)] {
        assert_eq!(pixmap.pixel(x, y).unwrap().alpha(), 255, "{x}, {y}");
    }
}

#[test]
fn the_text_of_a_picture_is_drawn_with_the_embedded_font() {
    let text = r#"{"type":"Textbox","left":90,"top":0,"width":180,"height":20.34,"fontSize":18,"fontFamily":"Inter Variable, sans-serif","text":"Hello"}"#;
    let blank = r#"{"type":"Textbox","left":90,"top":0,"width":180,"height":20.34,"fontSize":18,"text":" "}"#;

    let written = pictures(&scene(text), &colours(), &fonts()).unwrap();
    let empty = pictures(&scene(blank), &colours(), &fonts()).unwrap();

    assert_ne!(written.picture, empty.picture);
}
