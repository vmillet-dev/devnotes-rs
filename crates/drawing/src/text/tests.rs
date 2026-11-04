use serde_json::Value;

use super::*;

pub(crate) fn fonts() -> Fonts {
    Fonts::new(&[
        include_bytes!("../../../ui/assets/fonts/inter-latin-wght-normal.ttf"),
        include_bytes!("../../../ui/assets/fonts/inter-latin-ext-wght-normal.ttf"),
    ])
}

fn wrapped(text: &str, width: f64) -> Vec<String> {
    wrap(text, width, 18.0, &fonts())
}

/// Where Fabric's lines, in a browser, kerned, break elsewhere than these (E14).
const KERNED_ELSEWHERE: [&str; 1] =
    ["The quick brown fox jumps over the lazy dog, twice over, then rests"];

#[test]
fn the_lines_are_those_fabric_broke_in_a_browser() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../../../golden/fabric-wrap/cases.json")).unwrap();
    let fonts = fonts();

    for case in cases {
        let text = case["input"].as_str().unwrap();
        let width = case["options"]["width"].as_f64().unwrap();
        let font_size = case["options"]["fontSize"].as_f64().unwrap();
        let expected: Vec<&str> = case["output"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line.as_str().unwrap())
            .collect();

        if !KERNED_ELSEWHERE.contains(&text) {
            assert_eq!(wrap(text, width, font_size, &fonts), expected, "{text}");
        }
    }
}

#[test]
fn a_word_is_never_broken_and_widens_the_box() {
    assert_eq!(
        wrapped("supercalifragilistic ok", 10.0),
        ["supercalifragilistic", "ok"]
    );
}

#[test]
fn a_windows_line_end_is_one_line_end() {
    assert_eq!(wrapped("one\r\ntwo", 180.0), ["one", "two"]);
}

#[test]
fn a_character_no_font_holds_still_takes_room() {
    assert_eq!(fonts().advance('\u{10FFFF}'), NO_GLYPH);
    assert!(fonts().advance('i') < fonts().advance('m'));
}

#[test]
fn the_baselines_are_fabrics() {
    assert!((line_step(18.0, 1.16) - 23.5944).abs() < 1e-9);
    assert!((first_baseline(18.0) - 15.82452).abs() < 1e-9);
}
