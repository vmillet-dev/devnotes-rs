//! The one way to a picture of a drawing (D23): the SVG written from the scene, which is also
//! what is exported, and `resvg` for the card's and the Note's PNG.

use std::fmt::Write as _;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

use crate::geometry::{Box, MARGIN, Point, number, path_box, picture_scale, points_box, scene_box};
use crate::scene::{Drawing, Hue, Kind, Object, PathCommand};
use crate::text::{Fonts, first_baseline, line_step, wrap};

/// A card's picture and a Note's: the longest side, and the most a pixel is enlarged.
const THUMBNAIL: (f64, f64) = (480.0, 1.0);
const PICTURE: (f64, f64) = (1600.0, 2.0);

/// The colours each hue takes in the theme on screen, in the order of `Hue::ALL`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    pub stroke: [String; 6],
    pub fill: [String; 6],
    pub paper: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Png,
    Svg,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
        }
    }
}

/// A drawing's two pictures, as PNG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pictures {
    pub thumbnail: Vec<u8>,
    pub picture: Vec<u8>,
}

impl Palette {
    fn stroke(&self, hue: Hue) -> &str {
        &self.stroke[hue as usize]
    }

    fn fill(&self, hue: Hue) -> &str {
        &self.fill[hue as usize]
    }
}

/// `None` when nothing is drawn: a drawing with nothing on it any more has no picture.
pub fn pictures(drawing: &Drawing, palette: &Palette, fonts: &Fonts) -> Option<Pictures> {
    Some(Pictures {
        thumbnail: png(drawing, palette, fonts, THUMBNAIL)?,
        picture: png(drawing, palette, fonts, PICTURE)?,
    })
}

/// The file a drawing is exported as: the large picture, or the SVG it is drawn from. `None`
/// when nothing is drawn.
pub fn export(
    drawing: &Drawing,
    palette: &Palette,
    fonts: &Fonts,
    format: Format,
) -> Option<Vec<u8>> {
    match format {
        Format::Png => png(drawing, palette, fonts, PICTURE),
        Format::Svg => svg(drawing, palette, fonts).map(String::into_bytes),
    }
}

fn png(
    drawing: &Drawing,
    palette: &Palette,
    fonts: &Fonts,
    (side, most): (f64, f64),
) -> Option<Vec<u8>> {
    let frame = frame(drawing)?;
    let options = usvg::Options {
        font_family: fonts.family.clone(),
        fontdb: fonts.database.clone(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(&svg(drawing, palette, fonts)?, &options).ok()?;
    let scale = picture_scale(&frame, side, most);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "positive, and a picture's side at most"
    )]
    let (width, height) = (
        (frame.width * scale).ceil() as u32,
        (frame.height * scale).ceil() as u32,
    );
    let mut pixmap = Pixmap::new(width, height)?;
    let drawn = tree.size();
    #[expect(clippy::cast_precision_loss, reason = "a picture's side")]
    let transform =
        Transform::from_scale(width as f32 / drawn.width(), height as f32 / drawn.height());
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap.encode_png().ok()
}

/// What is drawn, with room around it; `None` when nothing is.
fn frame(drawing: &Drawing) -> Option<Box> {
    let boxes: Vec<Box> = drawing.objects.iter().map(Object::bounds).collect();
    scene_box(&boxes, MARGIN)
}

/// The drawing framed to what is drawn, its paper laid under it.
pub fn svg(drawing: &Drawing, palette: &Palette, fonts: &Fonts) -> Option<String> {
    let frame = frame(drawing)?;
    let (left, top) = (number(frame.left), number(frame.top));
    let (width, height) = (number(frame.width), number(frame.height));
    let mut out = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{left} {top} {width} {height}" width="{}" height="{}">"#,
        number(frame.width.round()),
        number(frame.height.round()),
    );
    // Past the frame: a size rounded leaves the frame a fraction short of the picture's edge.
    let _ = write!(
        out,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
        number(frame.left - 1.0),
        number(frame.top - 1.0),
        number(frame.width + 2.0),
        number(frame.height + 2.0),
        escaped(&palette.paper)
    );
    for object in &drawing.objects {
        write_object(&mut out, object, palette, fonts);
    }
    out.push_str("</svg>");
    Some(out)
}

fn write_object(out: &mut String, object: &Object, palette: &Palette, fonts: &Fonts) {
    // ⚠️ `resvg` knows no `non-scaling-stroke`: a stroke that keeps its width has its shape scaled
    // rather than the plane it is drawn on.
    let drawn_at = if object.stroke_uniform() {
        (object.scale_x().abs(), object.scale_y().abs())
    } else {
        (1.0, 1.0)
    };
    match object.kind() {
        Kind::Other => return,
        Kind::Group => {
            let _ = write!(out, r#"<g transform="{}">"#, transform(object, drawn_at));
            for child in &object.children {
                write_object(out, child, palette, fonts);
            }
            out.push_str("</g>");
            return;
        }
        Kind::Textbox => return write_text(out, object, palette, fonts),
        Kind::Rect | Kind::Ellipse | Kind::Polygon | Kind::Path => {}
    }
    let shape = shape(object, drawn_at);
    let _ = write!(
        out,
        r#"<{shape} transform="{}" {}/>"#,
        transform(object, drawn_at),
        paint(object, palette)
    );
}

/// The tag and geometry of a shape, around the plane's origin: what `left` and `top` place.
fn shape(object: &Object, (scale_x, scale_y): (f64, f64)) -> String {
    let (width, height) = (object.width() * scale_x, object.height() * scale_y);
    match object.kind() {
        Kind::Rect => {
            let radius = object.corner_radius();
            format!(
                r#"rect x="{}" y="{}" width="{}" height="{}" rx="{}" ry="{}""#,
                number(-width / 2.0),
                number(-height / 2.0),
                number(width),
                number(height),
                number(radius * scale_x),
                number(radius * scale_y),
            )
        }
        Kind::Ellipse => format!(
            r#"ellipse rx="{}" ry="{}""#,
            number(width / 2.0),
            number(height / 2.0)
        ),
        Kind::Polygon => {
            let points = object.points();
            let middle = points_box(&points).map(|bounds| bounds.centre());
            let middle = middle.unwrap_or(Point { x: 0.0, y: 0.0 });
            let points: Vec<String> = points
                .iter()
                .map(|point| {
                    let x = number((point.x - middle.x) * scale_x);
                    let y = number((point.y - middle.y) * scale_y);
                    format!("{x},{y}")
                })
                .collect();
            format!(r#"polygon points="{}""#, points.join(" "))
        }
        _ => format!(
            r#"path d="{}""#,
            path_data(&object.path(), (scale_x, scale_y))
        ),
    }
}

fn path_data(path: &[PathCommand], (scale_x, scale_y): (f64, f64)) -> String {
    let middle = path_box(path).map(|bounds| bounds.centre());
    let middle = middle.unwrap_or(Point { x: 0.0, y: 0.0 });
    let at = |point: Point| {
        let x = number((point.x - middle.x) * scale_x);
        let y = number((point.y - middle.y) * scale_y);
        format!("{x} {y}")
    };
    let commands: Vec<String> = path
        .iter()
        .map(|command| match *command {
            PathCommand::Move(to) => format!("M {}", at(to)),
            PathCommand::Line(to) => format!("L {}", at(to)),
            PathCommand::Quad(control, to) => format!("Q {} {}", at(control), at(to)),
            PathCommand::Close => "Z".to_owned(),
        })
        .collect();
    commands.join(" ")
}

/// Fabric's order: translate, rotate, scale, skew. What the shape took of the scale is left out.
fn transform(object: &Object, (drawn_x, drawn_y): (f64, f64)) -> String {
    let centre = object.centre();
    let (flip_x, flip_y) = object.flipped();
    let sign = |flipped: bool| if flipped { -1.0 } else { 1.0 };
    let (skew_x, skew_y) = object.skew();
    format!(
        "translate({} {}) rotate({}) scale({} {}) skewX({}) skewY({})",
        number(centre.x),
        number(centre.y),
        number(object.angle()),
        number(object.scale_x() / drawn_x * sign(flip_x)),
        number(object.scale_y() / drawn_y * sign(flip_y)),
        number(skew_x),
        number(skew_y),
    )
}

/// As the reference's `paint`: colours follow the hue, not what the scene stored.
fn paint(object: &Object, palette: &Palette) -> String {
    let hue = object.hue();
    let stroke = palette.stroke(hue);
    let fill = if object.ends().is_some() {
        stroke
    } else if object.kind() == Kind::Path {
        "none"
    } else {
        palette.fill(hue)
    };
    format!(
        r#"fill="{}" stroke="{}" stroke-width="{}" stroke-linecap="{}" stroke-linejoin="{}""#,
        escaped(fill),
        escaped(stroke),
        number(object.stroke_width()),
        escaped(object.line_cap()),
        escaped(object.line_join()),
    )
}

fn write_text(out: &mut String, object: &Object, palette: &Palette, fonts: &Fonts) {
    let font_size = object.font_size();
    let lines = wrap(
        object.text().unwrap_or_default(),
        object.width(),
        font_size,
        fonts,
    );
    let step = line_step(font_size, object.line_height());
    let left = -object.width() / 2.0;
    let top = -object.height() / 2.0 + first_baseline(font_size);
    let _ = write!(
        out,
        r#"<g transform="{}" fill="{}" font-family="{}" font-size="{}" xml:space="preserve">"#,
        transform(object, (1.0, 1.0)),
        escaped(palette.stroke(object.hue())),
        escaped(object.font_family()),
        number(font_size),
    );
    let mut baseline = top;
    for line in lines {
        let _ = write!(
            out,
            r#"<text x="{}" y="{}">{}</text>"#,
            number(left),
            number(baseline),
            escaped(&line)
        );
        baseline += step;
    }
    out.push_str("</g>");
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests;
