use super::*;

/// What Fabric 7.4.0 wrote for the objects the reference makes (`golden/fabric`): `(what, body)`.
fn cases() -> Vec<(String, String)> {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../../../golden/fabric/cases.json")).unwrap();
    cases
        .iter()
        .map(|case| {
            let read = |name: &str| case[name].as_str().unwrap().to_owned();
            (read("input"), read("output"))
        })
        .collect()
}

fn drawn() -> Drawing {
    Drawing::read(&cases()[0].1).unwrap()
}

fn at(x: f64, y: f64) -> Point {
    Point { x, y }
}

#[test]
fn what_fabric_wrote_is_written_back_to_the_byte() {
    for (what, body) in cases() {
        assert_eq!(Drawing::read(&body).unwrap().write(), body, "{what}");
    }
}

#[test]
fn the_scenes_the_reference_tests_hold_come_back_unchanged() {
    for body in [
        r#"{"version":1,"scene":{}}"#,
        r#"{"version":1,"scene":{"objects":[]}}"#,
        r#"{"version":1,"scene":{"objects":[{"type":"Rect","left":60,"top":45,"width":100,"height":50}]}}"#,
        r##"{"scene":{"version":"7.4.0","background":"#000","objects":[{"type":"Circle","radius":3,"extra":{"a":[1,null]}}],"later":true},"later":1}"##,
    ] {
        assert_eq!(Drawing::read(body).unwrap().write(), body);
    }
}

#[test]
fn a_body_with_no_scene_opens_as_a_new_drawing() {
    for body in [
        "",
        "not json",
        "[1, 2]",
        r#"{"version":1}"#,
        r#"{"scene":3}"#,
        r#"{"scene":{"objects":3}}"#,
        r#"{"scene":{"objects":[3]}}"#,
        r#"{"scene":{"objects":[{"objects":[3]}]}}"#,
    ] {
        assert_eq!(Drawing::read(body), None, "{body}");
    }
}

#[test]
fn each_object_reads_as_its_kind_and_hue() {
    let drawing = drawn();
    let read: Vec<_> = drawing
        .objects
        .iter()
        .map(|object| (object.kind(), object.hue()))
        .collect();

    assert_eq!(
        read,
        [
            (Kind::Rect, Hue::Ink),
            (Kind::Ellipse, Hue::Blue),
            (Kind::Polygon, Hue::Green),
            (Kind::Path, Hue::Red),
            (Kind::Path, Hue::Purple),
            (Kind::Textbox, Hue::Amber),
            (Kind::Group, Hue::Ink),
        ]
    );
}

#[test]
fn a_type_the_app_never_draws_is_other() {
    let drawing = Drawing::read(r#"{"scene":{"objects":[{"type":"Circle"},{}]}}"#).unwrap();

    assert_eq!(drawing.objects[0].kind(), Kind::Other);
    assert_eq!(drawing.objects[1].kind(), Kind::Other);
}

#[test]
fn an_object_is_placed_by_its_centre_with_fabrics_transform() {
    let drawing = drawn();
    let ellipse = &drawing.objects[1];

    assert_eq!(ellipse.id(), Some("ellipse-1"));
    assert_eq!(ellipse.centre(), at(420.5, 90.25));
    assert_eq!(
        [
            ellipse.width(),
            ellipse.height(),
            ellipse.scale_x(),
            ellipse.scale_y(),
            ellipse.angle(),
            ellipse.stroke_width(),
            drawing.objects[0].corner_radius(),
        ],
        [140.0, 80.0, 1.5, 0.75, 30.0, 2.0, 6.0]
    );
}

#[test]
fn a_field_left_out_reads_as_fabrics_default() {
    let drawing =
        Drawing::read(r#"{"scene":{"objects":[{"type":"Textbox","hue":"teal","ends":[1,2]}]}}"#)
            .unwrap();
    let object = &drawing.objects[0];

    assert_eq!(object.centre(), at(0.0, 0.0));
    assert_eq!(
        [
            object.scale_x(),
            object.scale_y(),
            object.angle(),
            object.stroke_width(),
            object.font_size(),
        ],
        [1.0, 1.0, 0.0, 1.0, 40.0]
    );
    assert_eq!(object.hue(), Hue::Ink);
    assert_eq!(object.ends(), None);
    assert_eq!(object.text(), None);
    assert!(object.points().is_empty() && object.path().is_empty());
}

#[test]
fn an_arrow_holds_its_ends_its_shapes_and_its_path() {
    let drawing = drawn();
    let arrow = &drawing.objects[3];

    assert_eq!(arrow.from(), Some("rect-1"));
    assert_eq!(arrow.to(), Some("ellipse-1"));
    assert_eq!(
        arrow.ends().map(|ends| ends[0]),
        Some(240.000_000_000_000_03)
    );
    assert_eq!(
        arrow.path().first(),
        Some(&PathCommand::Move(at(240.0, 90.1)))
    );
    assert_eq!(arrow.path().last(), Some(&PathCommand::Close));
}

#[test]
fn a_freehand_line_is_read_with_its_curves() {
    let drawing = drawn();

    assert_eq!(
        drawing.objects[4].path(),
        [
            PathCommand::Move(at(300.0, 300.0)),
            PathCommand::Quad(at(301.0, 301.0), at(310.5, 305.25)),
            PathCommand::Quad(at(320.0, 310.0), at(330.0, 300.0)),
            PathCommand::Line(at(340.0, 302.0)),
        ]
    );
}

#[test]
fn a_path_command_the_app_never_wrote_is_not_drawn_and_stays_stored() {
    let body =
        r#"{"scene":{"objects":[{"type":"Path","path":[["M",1,2],["C",1,2,3,4,5,6],["L",3]]}]}}"#;
    let drawing = Drawing::read(body).unwrap();

    assert_eq!(drawing.objects[0].path(), [PathCommand::Move(at(1.0, 2.0))]);
    assert_eq!(drawing.write(), body);
}

#[test]
fn a_diamond_keeps_its_corners_a_text_its_words_a_group_its_children() {
    let drawing = drawn();

    assert_eq!(
        drawing.objects[2].points(),
        [
            at(60.0, 0.0),
            at(120.0, 40.0),
            at(60.0, 80.0),
            at(0.0, 40.0)
        ]
    );
    assert_eq!(
        drawing.objects[5].text(),
        Some("API gateway\nsits in front of « everything »")
    );
    assert_eq!(drawing.objects[5].font_size(), 18.0);
    let inside: Vec<_> = drawing.objects[6]
        .children
        .iter()
        .map(Object::kind)
        .collect();
    assert_eq!(inside, [Kind::Rect, Kind::Textbox]);
}

#[test]
fn every_hue_is_read_back_by_its_name() {
    for hue in Hue::ALL {
        assert_eq!(Hue::named(hue.name()), hue);
    }
}

#[test]
fn an_object_taken_away_leaves_the_rest_as_stored() {
    let body = r#"{"version":1,"scene":{"objects":[{"id":"a"},{"id":"b"}],"background":"x"}}"#;
    let mut drawing = Drawing::read(body).unwrap();

    drawing.objects.remove(0);

    assert_eq!(
        drawing.write(),
        r#"{"version":1,"scene":{"objects":[{"id":"b"}],"background":"x"}}"#
    );
}

#[test]
fn the_first_object_put_on_an_empty_scene_is_stored() {
    let mut drawing = Drawing::read(r#"{"scene":{}}"#).unwrap();
    drawing.objects = Drawing::read(r#"{"scene":{"objects":[{"id":"a"}]}}"#)
        .unwrap()
        .objects;

    assert_eq!(drawing.write(), r#"{"scene":{"objects":[{"id":"a"}]}}"#);
}
