//! The scene a Dessin stores: Fabric's JSON. An object is its fields as they were read and is
//! read through them, so that what no code here names, or names and leaves alone, is written
//! back as it came.

use serde_json::{Map, Value};

use crate::geometry::{Ends, Point};

const OBJECTS: &str = "objects";

/// A drawing as stored: `{ version, scene }`, the scene's objects taken out to be worked on.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    body: Map<String, Value>,
    scene: Map<String, Value>,
    pub objects: Vec<Object>,
}

/// One thing on the drawing; a group holds others.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    fields: Map<String, Value>,
    pub children: Vec<Object>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Rect,
    Ellipse,
    Polygon,
    Path,
    Textbox,
    Group,
    /// Fabric's, but nothing the app draws: kept, not shown.
    Other,
}

/// What a colour is called, so a theme changed afterwards draws it in its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Hue {
    #[default]
    Ink,
    Blue,
    Green,
    Amber,
    Red,
    Purple,
}

/// The commands the app's paths are made of: `M/L/Z` for an arrow, `M/Q/L` for the pen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    Move(Point),
    Line(Point),
    Quad(Point, Point),
    Close,
}

impl Drawing {
    /// `None` for a body that holds no scene, or one whose objects are not objects: what
    /// opens as a new drawing.
    pub fn read(content: &str) -> Option<Self> {
        let Value::Object(mut body) = serde_json::from_str(content).ok()? else {
            return None;
        };
        let Some(Value::Object(mut scene)) = body.get_mut("scene").map(Value::take) else {
            return None;
        };
        let objects = take_objects(&mut scene)?;
        Some(Self {
            body,
            scene,
            objects,
        })
    }

    /// The body to store.
    pub fn write(&self) -> String {
        let mut scene = self.scene.clone();
        put_objects(&mut scene, &self.objects);
        let mut body = self.body.clone();
        body.insert("scene".to_owned(), Value::Object(scene));
        Value::Object(body).to_string()
    }
}

/// Leaves a `null` in the objects' place, for `put_objects` to fill: the fields keep their order.
fn take_objects(fields: &mut Map<String, Value>) -> Option<Vec<Object>> {
    let Some(objects) = fields.get_mut(OBJECTS).map(Value::take) else {
        return Some(Vec::new());
    };
    let Value::Array(objects) = objects else {
        return None;
    };
    objects.into_iter().map(Object::read).collect()
}

fn put_objects(fields: &mut Map<String, Value>, objects: &[Object]) {
    if objects.is_empty() && !fields.contains_key(OBJECTS) {
        return;
    }
    let objects = objects
        .iter()
        .map(|object| Value::Object(object.fields()))
        .collect();
    fields.insert(OBJECTS.to_owned(), Value::Array(objects));
}

impl Object {
    fn read(value: Value) -> Option<Self> {
        let Value::Object(mut fields) = value else {
            return None;
        };
        let children = take_objects(&mut fields)?;
        Some(Self { fields, children })
    }

    fn fields(&self) -> Map<String, Value> {
        let mut fields = self.fields.clone();
        put_objects(&mut fields, &self.children);
        fields
    }

    pub fn kind(&self) -> Kind {
        match self.words("type") {
            Some("Rect") => Kind::Rect,
            Some("Ellipse") => Kind::Ellipse,
            Some("Polygon") => Kind::Polygon,
            Some("Path") => Kind::Path,
            Some("Textbox") => Kind::Textbox,
            Some("Group") => Kind::Group,
            _ => Kind::Other,
        }
    }

    pub fn id(&self) -> Option<&str> {
        self.words("id")
    }

    pub fn hue(&self) -> Hue {
        self.words("hue").map(Hue::named).unwrap_or_default()
    }

    /// The shape an arrow starts on.
    pub fn from(&self) -> Option<&str> {
        self.words("from")
    }

    /// The shape an arrow ends on.
    pub fn to(&self) -> Option<&str> {
        self.words("to")
    }

    /// Only an arrow has them.
    pub fn ends(&self) -> Option<Ends> {
        let ends = self.fields.get("ends")?.as_array()?;
        let ends: Vec<f64> = ends.iter().filter_map(Value::as_f64).collect();
        ends.try_into().ok()
    }

    /// ⚠️ Fabric 7 places an object by its centre: `left` and `top` are the box's middle.
    pub fn centre(&self) -> Point {
        Point {
            x: self.number("left", 0.0),
            y: self.number("top", 0.0),
        }
    }

    pub fn width(&self) -> f64 {
        self.number("width", 0.0)
    }

    pub fn height(&self) -> f64 {
        self.number("height", 0.0)
    }

    pub fn scale_x(&self) -> f64 {
        self.number("scaleX", 1.0)
    }

    pub fn scale_y(&self) -> f64 {
        self.number("scaleY", 1.0)
    }

    /// In degrees, clockwise.
    pub fn angle(&self) -> f64 {
        self.number("angle", 0.0)
    }

    pub fn stroke_width(&self) -> f64 {
        self.number("strokeWidth", 1.0)
    }

    /// A rectangle's corners.
    pub fn corner_radius(&self) -> f64 {
        self.number("rx", 0.0)
    }

    /// A pen's line ends and bends round; everything else keeps Fabric's default.
    pub fn line_cap(&self) -> &str {
        self.words("strokeLineCap").unwrap_or("butt")
    }

    pub fn line_join(&self) -> &str {
        self.words("strokeLineJoin").unwrap_or("miter")
    }

    /// A stroke that keeps its width when the object is scaled.
    pub fn stroke_uniform(&self) -> bool {
        self.fields.get("strokeUniform").and_then(Value::as_bool) == Some(true)
    }

    /// Mirrored, as a handle dragged past the opposite side leaves it.
    pub fn flipped(&self) -> (bool, bool) {
        let flip = |name| self.fields.get(name).and_then(Value::as_bool) == Some(true);
        (flip("flipX"), flip("flipY"))
    }

    /// In degrees: Fabric's side handles lean an object with Shift held.
    pub fn skew(&self) -> (f64, f64) {
        (self.number("skewX", 0.0), self.number("skewY", 0.0))
    }

    pub fn text(&self) -> Option<&str> {
        self.words("text")
    }

    pub fn font_size(&self) -> f64 {
        self.number("fontSize", 40.0)
    }

    pub fn font_family(&self) -> &str {
        self.words("fontFamily").unwrap_or("sans-serif")
    }

    pub fn line_height(&self) -> f64 {
        self.number("lineHeight", 1.16)
    }

    /// A polygon's corners, in its own plane.
    pub fn points(&self) -> Vec<Point> {
        self.list("points", point_of)
    }

    /// A command this app never wrote is left out of what is drawn, and stays stored.
    pub fn path(&self) -> Vec<PathCommand> {
        self.list("path", PathCommand::read)
    }

    fn words(&self, name: &str) -> Option<&str> {
        self.fields.get(name)?.as_str()
    }

    fn number(&self, name: &str, otherwise: f64) -> f64 {
        self.fields
            .get(name)
            .and_then(Value::as_f64)
            .unwrap_or(otherwise)
    }

    fn list<T>(&self, name: &str, read: impl Fn(&Value) -> Option<T>) -> Vec<T> {
        let list = self.fields.get(name).and_then(Value::as_array);
        list.into_iter().flatten().filter_map(read).collect()
    }
}

fn point_of(value: &Value) -> Option<Point> {
    Some(Point {
        x: value.get("x")?.as_f64()?,
        y: value.get("y")?.as_f64()?,
    })
}

impl Hue {
    pub const ALL: [Self; 6] = [
        Self::Ink,
        Self::Blue,
        Self::Green,
        Self::Amber,
        Self::Red,
        Self::Purple,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Ink => "ink",
            Self::Blue => "blue",
            Self::Green => "green",
            Self::Amber => "amber",
            Self::Red => "red",
            Self::Purple => "purple",
        }
    }

    /// A name no theme paints is ink, as an object with none.
    fn named(name: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|hue| hue.name() == name)
            .unwrap_or_default()
    }
}

impl PathCommand {
    fn read(value: &Value) -> Option<Self> {
        let parts = value.as_array()?;
        let at = |index: usize| {
            Some(Point {
                x: parts.get(index)?.as_f64()?,
                y: parts.get(index + 1)?.as_f64()?,
            })
        };
        match parts.first()?.as_str()? {
            "M" => Some(Self::Move(at(1)?)),
            "L" => Some(Self::Line(at(1)?)),
            "Q" => Some(Self::Quad(at(1)?, at(3)?)),
            "Z" => Some(Self::Close),
            _ => None,
        }
    }
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "the numbers stored, read back untouched")]
mod tests;
