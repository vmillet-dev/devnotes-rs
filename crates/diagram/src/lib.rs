//! A Note's `mermaid` block drawn (D2, ADR S7): `merman` writes the SVG in the theme's colours,
//! measuring its labels with the app's fonts, and `resvg` paints it. One drawing at a time, off
//! the frame; what is drawn is pixels, what failed says its line.

use std::sync::Arc;

use merman::svg::SvgPipeline;
use merman::svg::export::{NativeFontContext, NativeFontLimits};
use merman::{
    Engine, MermaidConfig, OperationControl, ParseOptions, RenderError, RenderOutput,
    RenderRequest, Renderer, SvgEnvironment, SvgRequest,
};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, fontdb};

/// The fence language that makes a Note's code block a diagram.
pub const MERMAID: &str = "mermaid";

/// The drawing's pixels per point: sharp on a screen at twice the density.
pub const SCALE: f32 = 2.0;

/// A side past which a drawing is not painted, in pixels: a diagram that large is a mistake.
const LARGEST: f32 = 8_192.0;

/// The theme a diagram is drawn in, as `#rrggbb` colours: Mermaid derives its shades from them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Theme {
    pub dark: bool,
    pub background: String,
    pub primary: String,
    pub primary_text: String,
    pub primary_border: String,
    pub secondary: String,
    pub line: String,
    pub text: String,
}

/// The embedded fonts a diagram's labels are measured and painted with.
pub struct Fonts {
    context: NativeFontContext,
    database: Arc<fontdb::Database>,
    family: String,
}

impl Fonts {
    /// `None` when no file is a font.
    pub fn new(files: &[&'static [u8]]) -> Option<Self> {
        let control = OperationControl::new();
        let owned = files.iter().map(|file| file.to_vec());
        let context =
            NativeFontContext::from_font_bytes(owned, NativeFontLimits::default(), &control)
                .ok()?;
        let mut database = fontdb::Database::new();
        for file in files {
            database.load_font_data(file.to_vec());
        }
        let family = database.faces().next()?.families.first()?.0.clone();
        database.set_sans_serif_family(family.clone());
        Some(Self {
            context,
            database: Arc::new(database),
            family,
        })
    }
}

/// A diagram drawn, or why not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drawn {
    /// Premultiplied RGBA, `SCALE` pixels a point.
    Picture {
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
    /// The source is not Mermaid: the line it stopped at (from 1), when it says, and what it says.
    Failed { line: Option<usize>, detail: String },
    /// A kind of diagram this engine does not draw.
    Unsupported,
}

pub fn draw(source: &str, theme: &Theme, fonts: &Fonts) -> Drawn {
    let renderer = Renderer::new()
        .with_engine(Engine::new().with_site_config(config(theme, &fonts.family)))
        .with_parse_options(ParseOptions::strict());
    let Ok(environment) =
        SvgEnvironment::deterministic().with_native_font_context(fonts.context.clone())
    else {
        return Drawn::Unsupported;
    };
    let request = SvgRequest {
        environment,
        pipeline: Some(SvgPipeline::resvg_safe()),
        ..SvgRequest::default()
    };
    match renderer.render(RenderRequest::svg(source, OperationControl::new(), request)) {
        Ok(RenderOutput::Svg(Some(svg))) => paint(svg.svg(), fonts).unwrap_or(Drawn::Unsupported),
        Err(RenderError::Parse(diagnostic)) => failed(source, &diagnostic),
        Ok(_) | Err(_) => Drawn::Unsupported,
    }
}

fn failed(source: &str, diagnostic: &merman::TerminalDiagnostic) -> Drawn {
    let line = diagnostic
        .terminal_diagnostic_details()
        .span
        .map(|span| line_of(source, span.start));
    Drawn::Failed {
        line,
        detail: diagnostic.terminal_safe_message(),
    }
}

/// The line, from 1, that byte `at` stands on.
fn line_of(source: &str, at: usize) -> usize {
    source.get(..at).unwrap_or(source).matches('\n').count() + 1
}

/// Mermaid's `base` theme, its variables from the tokens, as `MermaidRenderer` gave them.
fn config(theme: &Theme, family: &str) -> MermaidConfig {
    MermaidConfig::from_value(serde_json::json!({
        "theme": "base",
        "fontFamily": family,
        "themeVariables": {
            "darkMode": theme.dark,
            "fontFamily": family,
            "background": theme.background,
            "primaryColor": theme.primary,
            "primaryTextColor": theme.primary_text,
            "primaryBorderColor": theme.primary_border,
            "secondaryColor": theme.secondary,
            "tertiaryColor": theme.secondary,
            "lineColor": theme.line,
            "textColor": theme.text,
            "edgeLabelBackground": theme.background,
        },
    }))
}

fn paint(svg: &str, fonts: &Fonts) -> Option<Drawn> {
    let options = usvg::Options {
        font_family: fonts.family.clone(),
        fontdb: Arc::clone(&fonts.database),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(svg, &options).ok()?;
    let size = tree.size();
    let scale = SCALE.min(LARGEST / size.width().max(size.height()));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "positive, and LARGEST at most"
    )]
    let (width, height) = (
        (size.width() * scale).ceil() as u32,
        (size.height() * scale).ceil() as u32,
    );
    let mut pixmap = Pixmap::new(width, height)?;
    resvg::render(
        &tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Some(Drawn::Picture {
        width,
        height,
        pixels: pixmap.take(),
    })
}

#[cfg(test)]
mod tests;
