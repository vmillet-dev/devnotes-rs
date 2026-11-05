use super::*;

const INTER: &[u8] = include_bytes!("../../ui/assets/fonts/inter-latin-wght-normal.ttf");

fn fonts() -> Fonts {
    Fonts::new(&[INTER]).unwrap()
}

fn theme() -> Theme {
    Theme {
        dark: true,
        background: "#1b1e25".into(),
        primary: "#2a2e37".into(),
        primary_text: "#e8e6e1".into(),
        primary_border: "#3a3f4a".into(),
        secondary: "#22262e".into(),
        line: "#8a8f98".into(),
        text: "#e8e6e1".into(),
    }
}

#[test]
fn a_flowchart_is_drawn() {
    let Drawn::Picture {
        width,
        height,
        pixels,
    } = draw("graph TD; A-->B", &theme(), &fonts())
    else {
        panic!("not drawn");
    };

    assert!(width > 100 && height > 100, "{width} × {height}");
    assert_eq!(pixels.len(), (width * height * 4) as usize);
    assert!(pixels.iter().any(|byte| *byte != 0), "nothing painted");
}

#[test]
fn a_syntax_error_names_its_line() {
    let drawn = draw("graph TD\n  A -->", &theme(), &fonts());

    let Drawn::Failed { line, detail } = drawn else {
        panic!("{drawn:?}");
    };
    assert_eq!(line, Some(2));
    assert!(!detail.is_empty());
}

#[test]
fn a_text_naming_no_diagram_fails_without_a_line() {
    let drawn = draw("nonsense here", &theme(), &fonts());

    assert!(
        matches!(drawn, Drawn::Failed { line: None, .. }),
        "{drawn:?}"
    );
}

#[test]
fn a_line_is_counted_from_one_and_a_span_past_the_end_stays_on_the_last() {
    assert_eq!(line_of("a\nb\nc", 0), 1);
    assert_eq!(line_of("a\nb\nc", 2), 2);
    assert_eq!(line_of("a\nb\nc", 99), 3);
}

#[test]
fn no_file_that_is_a_font_gives_no_fonts() {
    assert!(Fonts::new(&[b"not a font"]).is_none());
}
