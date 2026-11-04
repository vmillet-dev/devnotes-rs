//! Lucide's paths (<https://lucide.dev>, ISC licence), 24 × 24, drawn as 2 px strokes: the
//! reference's `shared/icon`, plus the three that replace the emoji the interface still drew
//! (📎 🖼 ⏳). Rasterised by resvg into an alpha mask at the size asked × `pixels_per_point`,
//! cached, then tinted by the colour it is painted in — `currentColor`, as in the CSS.
//! Decorative by construction: the control carrying an icon names itself.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

use egui::{Color32, ColorImage, Context, Id, Rect, TextureHandle, TextureOptions, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    ArrowLeftRight,
    Sidebar,
    Trash,
    Filter,
    ArrowUp,
    ArrowDown,
    /// The sort's ⇅.
    ArrowUpDown,
    ArrowRight,
    ArrowLeft,
    /// Enter's ↵.
    CornerDownLeft,
    /// Backspace's ⌫.
    Delete,
    ChevronDown,
    ChevronRight,
    X,
    Plus,
    Minus,
    /// A row's ⋯: what else it can be told.
    Ellipsis,
    Pin,
    Star,
    /// A star that is on: the same drawing, full.
    StarFilled,
    Sun,
    Moon,
    BookOpen,
    Bold,
    Italic,
    Strike,
    Code,
    ListBullet,
    ListOrdered,
    ListTasks,
    Quote,
    Link,
    Table,
    FileText,
    Type,
    Lock,
    Sparkle,
    Clock,
    Calculator,
    Search,
    Copy,
    Check,
    FolderPlus,
    Eye,
    EyeOff,
    Wrench,
    Fold,
    Unfold,
    Paperclip,
    Image,
    Hourglass,
    /// A snippet's `{{fields}}`, on its card.
    Zap,
    /// The error banner's « ⚠ ».
    TriangleAlert,
    Calendar,
    /// The editor's full screen, and the way out of it.
    Maximize,
    Minimize,
    MousePointer,
    Square,
    Circle,
    Diamond,
    Pencil,
    Undo,
    Redo,
}

impl Icon {
    #[allow(clippy::too_many_lines, reason = "a table, one arm an icon")]
    fn paths(self) -> &'static [&'static str] {
        match self {
            Self::ArrowLeftRight => &["m8 3-4 4 4 4", "M4 7h16", "m16 21 4-4-4-4", "M20 17H4"],
            Self::Sidebar => &[
                "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
                "M9 3v18",
            ],
            Self::Trash => &[
                "M3 6h18",
                "M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6",
                "M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2",
                "M10 11v6",
                "M14 11v6",
            ],
            Self::Filter => &["M22 3H2l8 9.46V19l4 2v-8.54L22 3z"],
            Self::ArrowUp => &["m5 12 7-7 7 7", "M12 19V5"],
            Self::ArrowDown => &["M12 5v14", "m19 12-7 7-7-7"],
            Self::ArrowUpDown => &["m21 16-4 4-4-4", "M17 20V4", "m3 8 4-4 4 4", "M7 4v16"],
            Self::ArrowRight => &["M5 12h14", "m12 5 7 7-7 7"],
            Self::ArrowLeft => &["M19 12H5", "m12 19-7-7 7-7"],
            Self::CornerDownLeft => &["M20 4v7a4 4 0 0 1-4 4H4", "m9 10-5 5 5 5"],
            Self::Delete => &[
                "M10 5a2 2 0 0 0-1.344.519l-6.328 5.74a1 1 0 0 0 0 1.481l6.328 5.741A2 2 0 0 0 10 19h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2z",
                "m12 9 6 6",
                "m18 9-6 6",
            ],
            Self::ChevronDown => &["m6 9 6 6 6-6"],
            Self::ChevronRight => &["m9 18 6-6-6-6"],
            Self::X => &["M18 6 6 18", "m6 6 12 12"],
            Self::Plus => &["M5 12h14", "M12 5v14"],
            Self::Minus => &["M5 12h14"],
            Self::Ellipsis => &["M12 12h.01", "M19 12h.01", "M5 12h.01"],
            Self::Pin => &[
                "M12 17v5",
                "M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z",
            ],
            Self::Star | Self::StarFilled => &[
                "M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z",
            ],
            Self::Sun => &[
                "M12 8a4 4 0 1 0 0 8a4 4 0 1 0 0-8z",
                "M12 2v2",
                "M12 20v2",
                "m4.93 4.93 1.41 1.41",
                "m17.66 17.66 1.41 1.41",
                "M2 12h2",
                "M20 12h2",
                "m6.34 17.66-1.41 1.41",
                "m19.07 4.93-1.41 1.41",
            ],
            Self::Moon => &["M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z"],
            Self::BookOpen => &[
                "M12 7v14",
                "M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z",
            ],
            Self::Bold => {
                &["M6 12h9a4 4 0 0 1 0 8H7a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h7a4 4 0 0 1 0 8"]
            }
            Self::Italic => &["M19 4h-9", "M14 20H5", "M15 4 9 20"],
            Self::Strike => &[
                "M16 4H9a3 3 0 0 0-2.83 4",
                "M14 12a4 4 0 0 1 0 8H6",
                "M4 12h16",
            ],
            Self::Code => &["m16 18 6-6-6-6", "m8 6-6 6 6 6"],
            Self::ListBullet => &[
                "M3 12h.01",
                "M3 18h.01",
                "M3 6h.01",
                "M8 12h13",
                "M8 18h13",
                "M8 6h13",
            ],
            Self::ListOrdered => &[
                "M10 12h11",
                "M10 18h11",
                "M10 6h11",
                "M4 10h2",
                "M4 6h1v4",
                "M6 18H4c0-1 2-2 2-3s-1-1.5-2-1",
            ],
            Self::ListTasks => &[
                "m3 17 2 2 4-4",
                "m3 7 2 2 4-4",
                "M13 6h8",
                "M13 12h8",
                "M13 18h8",
            ],
            Self::Quote => &["M17 6H3", "M21 12H8", "M21 18H8", "M3 12v6"],
            Self::Link => &[
                "M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71",
                "M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71",
            ],
            Self::Table => &[
                "M12 3v18",
                "M3 5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
                "M3 9h18",
                "M3 15h18",
            ],
            Self::FileText => &[
                "M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z",
                "M14 2v4a2 2 0 0 0 2 2h4",
                "M10 9H8",
                "M16 13H8",
                "M16 17H8",
            ],
            Self::Type => &["M4 7V5h16v2", "M12 5v14", "M9 19h6"],
            Self::Lock => &["M6 11h12v10H6z", "M8 11V7a4 4 0 0 1 8 0v4"],
            Self::Sparkle => &[
                "M12 3v4",
                "M12 17v4",
                "M3 12h4",
                "M17 12h4",
                "M6 6l2.5 2.5",
                "M15.5 15.5 18 18",
                "M6 18l2.5-2.5",
                "M15.5 8.5 18 6",
            ],
            Self::Clock => &["M12 2a10 10 0 1 0 0 20a10 10 0 1 0 0-20z", "M12 6v6l4 2"],
            Self::Calculator => &[
                "M6 2h12a2 2 0 0 1 2 2v16a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z",
                "M8 6h8",
                "M16 14v4",
                "M16 10h.01",
                "M12 10h.01",
                "M8 10h.01",
                "M12 14h.01",
                "M8 14h.01",
                "M12 18h.01",
                "M8 18h.01",
            ],
            Self::Search => &["M11 4a7 7 0 1 0 0 14a7 7 0 1 0 0-14z", "m20 20-3.5-3.5"],
            Self::Copy => &[
                "M10 8h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-8a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2z",
                "M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3",
            ],
            Self::Check => &["m5 12 5 5L20 7"],
            Self::FolderPlus => &[
                "M12 10v6",
                "M9 13h6",
                "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z",
            ],
            Self::Eye => &[
                "M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z",
                "M12 9a3 3 0 1 0 0 6a3 3 0 1 0 0-6z",
            ],
            Self::EyeOff => &[
                "M9.88 9.88a3 3 0 1 0 4.24 4.24",
                "M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68",
                "M6.61 6.61A13.526 13.526 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61",
                "m2 2 20 20",
            ],
            Self::Wrench => &[
                "M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z",
            ],
            Self::Fold => &["m7 20 5-5 5 5", "m7 4 5 5 5-5"],
            Self::Unfold => &["m7 15 5 5 5-5", "m7 9 5-5 5 5"],
            Self::Paperclip => &[
                "m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48",
            ],
            Self::Image => &[
                "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
                "M9 7a2 2 0 1 0 0 4a2 2 0 1 0 0-4z",
                "m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21",
            ],
            Self::TriangleAlert => &[
                "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3",
                "M12 9v4",
                "M12 17h.01",
            ],
            Self::Zap => &[
                "M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z",
            ],
            Self::Calendar => &[
                "M8 2v4",
                "M16 2v4",
                "M5 4h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z",
                "M3 10h18",
            ],
            Self::MousePointer => &[
                "M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z",
            ],
            Self::Square => {
                &["M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"]
            }
            Self::Circle => &["M12 2a10 10 0 1 0 0 20a10 10 0 1 0 0-20z"],
            Self::Diamond => &["M12 2.7 21.3 12 12 21.3 2.7 12z"],
            Self::Pencil => &[
                "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
                "m15 5 4 4",
            ],
            Self::Undo => &[
                "M9 14 4 9l5-5",
                "M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11",
            ],
            Self::Redo => &[
                "m15 14 5-5-5-5",
                "M20 9H9.5A5.5 5.5 0 0 0 4 14.5A5.5 5.5 0 0 0 9.5 20H13",
            ],
            Self::Maximize => &["M15 3h6v6", "M9 21H3v-6", "M21 3l-7 7", "M3 21l7-7"],
            Self::Minimize => &["M4 14h6v6", "M20 10h-6V4", "M14 10l7-7", "M3 21l7-7"],
            Self::Hourglass => &[
                "M5 22h14",
                "M5 2h14",
                "M17 22v-4.172a2 2 0 0 0-.586-1.414L12 12l-4.414 4.414A2 2 0 0 0 7 17.828V22",
                "M7 2v4.172a2 2 0 0 0 .586 1.414L12 12l4.414-4.414A2 2 0 0 0 17 6.172V2",
            ],
        }
    }

    fn svg(self) -> String {
        let fill = if self == Self::StarFilled {
            "white"
        } else {
            "none"
        };
        let paths = self.paths().iter().fold(String::new(), |mut paths, d| {
            let _ = write!(paths, "<path d=\"{d}\"/>");
            paths
        });
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"{fill}\" \
             stroke=\"white\" stroke-width=\"2\" stroke-linecap=\"round\" \
             stroke-linejoin=\"round\">{paths}</svg>"
        )
    }
}

/// Paints `icon` into `rect`, in `colour`.
pub fn paint(ui: &Ui, rect: Rect, icon: Icon, colour: Color32) {
    let side = (rect.width().min(rect.height()) * ui.ctx().pixels_per_point()).round();
    // Bounded: an icon is never drawn larger than a dialog.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pixels = side.clamp(1.0, 512.0) as u32;
    let Some(texture) = texture(ui.ctx(), icon, pixels) else {
        return;
    };
    let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    ui.painter().image(texture.id(), rect, uv, colour);
}

type Cache = Arc<HashMap<(Icon, u32), TextureHandle>>;

fn cache_id() -> Id {
    Id::unique("theme.icons")
}

/// ⚠️ The texture is made outside the memory's lock: `load_texture` takes the context too.
fn texture(ctx: &Context, icon: Icon, pixels: u32) -> Option<TextureHandle> {
    let cached = ctx.data(|data| {
        data.get_temp::<Cache>(cache_id())
            .and_then(|cache| cache.get(&(icon, pixels)).cloned())
    });
    if cached.is_some() {
        return cached;
    }
    let mask = rasterise(icon, pixels)?;
    let texture = ctx.load_texture(
        format!("icon-{icon:?}-{pixels}"),
        mask,
        TextureOptions::LINEAR,
    );
    ctx.data_mut(|data| {
        let cache = data.get_temp_mut_or_default::<Cache>(cache_id());
        Arc::make_mut(cache).insert((icon, pixels), texture.clone());
    });
    Some(texture)
}

/// White, with the drawing's coverage as alpha: egui multiplies it by the paint colour.
fn rasterise(icon: Icon, pixels: u32) -> Option<ColorImage> {
    let tree = resvg::usvg::Tree::from_str(&icon.svg(), &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixels, pixels)?;
    #[allow(clippy::cast_precision_loss)]
    let scale = pixels as f32 / 24.0;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let size = [pixmap.width() as usize, pixmap.height() as usize];
    Some(ColorImage::from_rgba_premultiplied(size, pixmap.data()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Icon; 63] = [
        Icon::ArrowLeftRight,
        Icon::Sidebar,
        Icon::Trash,
        Icon::Filter,
        Icon::ArrowUp,
        Icon::ArrowDown,
        Icon::ArrowUpDown,
        Icon::ArrowRight,
        Icon::ArrowLeft,
        Icon::CornerDownLeft,
        Icon::Delete,
        Icon::ChevronDown,
        Icon::ChevronRight,
        Icon::X,
        Icon::Plus,
        Icon::Minus,
        Icon::Ellipsis,
        Icon::Pin,
        Icon::Star,
        Icon::StarFilled,
        Icon::Sun,
        Icon::Moon,
        Icon::BookOpen,
        Icon::Bold,
        Icon::Italic,
        Icon::Strike,
        Icon::Code,
        Icon::ListBullet,
        Icon::ListOrdered,
        Icon::ListTasks,
        Icon::Quote,
        Icon::Link,
        Icon::Table,
        Icon::FileText,
        Icon::Type,
        Icon::Lock,
        Icon::Sparkle,
        Icon::Clock,
        Icon::Calculator,
        Icon::Search,
        Icon::Copy,
        Icon::Check,
        Icon::FolderPlus,
        Icon::Eye,
        Icon::EyeOff,
        Icon::Wrench,
        Icon::Fold,
        Icon::Unfold,
        Icon::Paperclip,
        Icon::Image,
        Icon::Hourglass,
        Icon::Zap,
        Icon::TriangleAlert,
        Icon::Calendar,
        Icon::Maximize,
        Icon::Minimize,
        Icon::MousePointer,
        Icon::Square,
        Icon::Circle,
        Icon::Diamond,
        Icon::Pencil,
        Icon::Undo,
        Icon::Redo,
    ];

    fn coverage(image: &ColorImage) -> usize {
        image.pixels.iter().filter(|pixel| pixel.a() > 0).count()
    }

    #[test]
    fn every_icon_draws_something_white() {
        for icon in ALL {
            let mask = rasterise(icon, 32).unwrap_or_else(|| panic!("{icon:?} did not parse"));

            assert!(coverage(&mask) > 0, "{icon:?} is empty");
            assert!(
                mask.pixels.iter().all(|pixel| pixel.r() == pixel.a()),
                "{icon:?} is not a white mask"
            );
        }
    }

    #[test]
    fn a_filled_star_covers_more_than_its_outline() {
        let outline = coverage(&rasterise(Icon::Star, 32).unwrap());
        let filled = coverage(&rasterise(Icon::StarFilled, 32).unwrap());

        assert!(filled > outline);
    }

    #[test]
    fn an_icon_is_rasterised_once_per_size() {
        let ctx = Context::default();

        let first = texture(&ctx, Icon::Pin, 16).unwrap();
        let again = texture(&ctx, Icon::Pin, 16).unwrap();
        let larger = texture(&ctx, Icon::Pin, 32).unwrap();

        assert_eq!(first.id(), again.id());
        assert_ne!(first.id(), larger.id());
        assert_eq!(larger.size(), [32, 32]);
    }
}
