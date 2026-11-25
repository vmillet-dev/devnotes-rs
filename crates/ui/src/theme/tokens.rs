//! The variables of the Angular reference's `styles.scss`, one for one: the dark
//! palette is the base, the light one redefines what a light surface needs, and the compact
//! density tightens four gaps and nothing else.
#![allow(
    clippy::unreadable_literal,
    reason = "written as the CSS writes them, so that a search finds both"
)]

use egui::Color32;
use egui::epaint::Shadow;

/// One theme's colours. The CSS `--*-rgb` triplets are not repeated: a tint is the colour
/// itself at an alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub bg_0: Color32,
    pub bg_1: Color32,
    pub bg_2: Color32,
    pub bg_3: Color32,
    pub line: Color32,
    pub line_soft: Color32,
    pub text_0: Color32,
    pub text_1: Color32,
    pub text_2: Color32,
    /// The accent as a surface: a solid button, a tick, a tint. The same in both themes.
    pub amber_fill: Color32,
    /// The accent as a hairline or a focus ring.
    pub amber_edge: Color32,
    /// The accent as a label.
    pub amber_text: Color32,
    /// The softer hairline, for a hover.
    pub amber_dim: Color32,
    /// Text drawn on solid amber.
    pub amber_ink: Color32,
    pub green: Color32,
    pub blue: Color32,
    pub red: Color32,
    pub purple: Color32,
    pub teal: Color32,
    pub pink: Color32,
    pub olive: Color32,
    /// Line numbers: dimmer than `text_2`, readable without drawing the eye.
    pub line_no: Color32,
    /// A modal's scrim, at an alpha. It stays dark in the light theme: that is what puts the
    /// panel in front.
    pub backdrop: Color32,
    pub shadow_dropdown: Shadow,
    pub shadow_overlay: Shadow,
}

const AMBER: Color32 = hex(0xe8a33d);

pub const DARK: Palette = Palette {
    bg_0: hex(0x15171c),
    bg_1: hex(0x1b1e25),
    bg_2: hex(0x22252e),
    bg_3: hex(0x2a2e39),
    line: hex(0x333947),
    line_soft: hex(0x282c37),
    text_0: hex(0xeef0f4),
    text_1: hex(0xaab0c0),
    text_2: hex(0x9298ac),
    // On a dark surface one amber does all three jobs.
    amber_fill: AMBER,
    amber_edge: AMBER,
    amber_text: AMBER,
    amber_dim: hex(0x8a662a),
    amber_ink: hex(0x1a1508),
    green: hex(0x6fbf8b),
    blue: hex(0x6f9bd1),
    red: hex(0xdc8281),
    purple: hex(0xa98fd1),
    teal: hex(0x5fbcbc),
    pink: hex(0xd68bbd),
    olive: hex(0xb4bd62),
    line_no: hex(0x3f4454),
    backdrop: hex(0x0a0b0e),
    shadow_dropdown: shadow(10, 30, 89),
    shadow_overlay: shadow(30, 60, 128),
};

/// ⚠️ `amber_fill` is the dark one: dark text on it reads at 7.8:1. On white it falls to 2.16:1,
/// under what a line (3) and a label (4.5) ask, so those two step down, and only those two.
/// Drawing the whole accent at the label's darkness is what read as mustard.
pub const LIGHT: Palette = Palette {
    // In another order than `DARK`, by what the light theme changes most: two tables in one
    // order read as a copied block.
    amber_edge: hex(0xcf6a08),
    amber_text: hex(0xa8500f),
    amber_dim: hex(0xdda882),
    amber_fill: AMBER,
    amber_ink: hex(0x1a1508),
    green: hex(0x297044),
    blue: hex(0x2a5d99),
    red: hex(0xb03a39),
    purple: hex(0x6a4aa6),
    teal: hex(0x1d6b6e),
    pink: hex(0x9c3a78),
    olive: hex(0x5a6114),
    text_0: hex(0x1a1d24),
    text_1: hex(0x4a5163),
    text_2: hex(0x636a7c),
    bg_0: hex(0xeceef2),
    bg_1: hex(0xf7f8fa),
    bg_2: hex(0xffffff),
    bg_3: hex(0xe9ecf2),
    line: hex(0xcdd2dc),
    line_soft: hex(0xe1e5ec),
    line_no: hex(0xb7bdca),
    backdrop: hex(0x404656),
    shadow_dropdown: shadow(10, 26, 36),
    shadow_overlay: shadow(24, 52, 51),
};

/// The four gaps density tightens; the type never shrinks, which would be a zoom out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing {
    pub card: f32,
    pub grid: f32,
    pub section: f32,
    pub canvas: f32,
}

pub const COMFORTABLE: Spacing = Spacing {
    card: 14.0,
    grid: 12.0,
    section: 28.0,
    canvas: 24.0,
};

pub const COMPACT: Spacing = Spacing {
    card: 10.0,
    grid: 8.0,
    section: 18.0,
    canvas: 16.0,
};

/// The type scale: each size named by what it is for, defined once. A screen picks a role, never
/// a number; changing a role's value changes every text that plays it.
pub mod font {
    /// Counts and monospace badges: a format's tag, a facet's total.
    pub const BADGE: f32 = 9.5;
    /// Keys, section titles in capitals, captions under a picture.
    pub const CAPTION: f32 = 10.5;
    /// What is said beside the content: dates, hints, secondary details.
    pub const META: f32 = 11.0;
    /// The words of a menu, a drop-down list or a chip.
    pub const CONTROL: f32 = 11.5;
    /// Buttons, field labels, table cells.
    pub const LABEL: f32 = 12.0;
    /// Running text: a card's title and preview, a paragraph.
    pub const BODY: f32 = 12.5;
    /// What is read or typed at length: a field's text, a list's content.
    pub const READING: f32 = 13.0;
    /// A subtitle under a title.
    pub const SUBTITLE: f32 = 14.0;
    /// A dialog's or a panel's title.
    pub const TITLE: f32 = 15.0;
    /// A page's title.
    pub const HEADING: f32 = 16.0;
    /// A value a tool shows large: a converted size, a time.
    pub const DISPLAY: f32 = 24.0;
    /// The largest values: a colour's code, a person's initials.
    pub const HERO: f32 = 32.0;
}

/// The gaps, named by what they separate, defined once.
pub mod gap {
    /// A rule's width.
    pub const HAIRLINE: f32 = 1.0;
    /// Between two lines of one block: a title and its subtitle.
    pub const LINE: f32 = 2.0;
    /// An icon and its words; the inside of a chip.
    pub const TIGHT: f32 = 4.0;
    /// The items of a compact list or row.
    pub const SNUG: f32 = 6.0;
    /// Between the controls of a row.
    pub const CONTROLS: f32 = 8.0;
    /// The inside of a control; between two fields.
    pub const INSET: f32 = 10.0;
    /// Between two groups of controls.
    pub const GROUPS: f32 = 12.0;
    /// Between two blocks of a panel.
    pub const BLOCKS: f32 = 16.0;
    /// A panel's or a dialog's margin.
    pub const PANEL: f32 = 20.0;
    /// Between two sections of a page.
    pub const PAGE: f32 = 28.0;

    /// A frame's margin, from two gaps (`0.0` for none).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the scale's steps are whole and small"
    )]
    pub const fn margin(x: f32, y: f32) -> egui::Margin {
        egui::Margin::symmetric(x as i8, y as i8)
    }
}

/// The corner radii, named by what they round, defined once.
pub mod radius {
    /// Bars and marks: a progress bar, a selection's edge.
    pub const MARK: u8 = 2;
    /// Tags and badges.
    pub const TAG: u8 = 4;
    /// Buttons and fields.
    pub const CONTROL: u8 = 6;
    /// Menus and tooltips.
    pub const MENU: u8 = 8;
    /// Cards.
    pub const CARD: u8 = 10;
    /// Panels.
    pub const PANEL: u8 = 12;
    /// Dialogs.
    pub const DIALOG: u8 = 14;
}

/// A full-screen editor starts under it, so the title bar stays in reach.
pub const TITLEBAR_HEIGHT: f32 = 38.0;

/// The bar at the top of every area and page — the notes header's first row, the tools', the
/// HTTP tabs and pages' —, and the sidebar's toggle at `BAR_INSET` from its left: the toggle
/// never moves from one to the next, whatever the density.
pub const BAR_HEIGHT: f32 = 56.0;
pub const BAR_INSET: f32 = gap::PANEL;
/// The toggle's side, centred in the bar.
pub const BAR_TOGGLE: f32 = 32.0;

const fn hex(rgb: u32) -> Color32 {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color32::from_rgb(r, g, b)
}

/// `0 <y>px <blur>px rgba(0, 0, 0, <alpha>)`, the only shape of shadow the reference draws;
/// the alpha in bytes (0.35 is 89).
const fn shadow(y: i8, blur: u8, alpha: u8) -> Shadow {
    Shadow {
        offset: [0, y],
        blur,
        spread: 0,
        color: Color32::from_black_alpha(alpha),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG 2.2 AA for text under 18.66 px, which is what every label here is drawn at.
    const AA: f64 = 4.5;
    /// WCAG 2.2 AA (1.4.11) for a control's boundary and a focus ring.
    const AA_UI: f64 = 3.0;
    /// The strongest tint a badge is drawn on (`tint-badge`).
    const MAX_TINT: f32 = 0.15;
    /// What a status (« 200 OK ») is drawn on (`status-badge`): twice a tint, to read as a signal.
    const STATUS_TINT: f32 = 0.3;

    fn luminance(colour: Color32) -> f64 {
        let channel = |raw: u8| {
            let part = f64::from(raw) / 255.0;
            if part <= 0.039_28 {
                part / 12.92
            } else {
                ((part + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(colour.r()) + 0.7152 * channel(colour.g()) + 0.0722 * channel(colour.b())
    }

    fn contrast(a: Color32, b: Color32) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        let (lighter, darker) = if a > b { (a, b) } else { (b, a) };
        (lighter + 0.05) / (darker + 0.05)
    }

    /// What a translucent fill composites to over an opaque surface, rounded as a browser does.
    fn mix(front: Color32, back: Color32, alpha: f32) -> Color32 {
        let channel = |f: u8, b: u8| {
            let blended = f32::from(f) * alpha + f32::from(b) * (1.0 - alpha);
            // Exact: two bytes blended stay within a byte.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let byte = blended.round() as u8;
            byte
        };
        Color32::from_rgb(
            channel(front.r(), back.r()),
            channel(front.g(), back.g()),
            channel(front.b(), back.b()),
        )
    }

    /// Every colour of a palette in one of four roles. Destructured without `..`: a colour
    /// added to the palette does not compile here until somebody says which it is.
    struct Roles {
        surfaces: [Color32; 4],
        text: [Color32; 11],
        edges: [Color32; 1],
        tinted: [Color32; 9],
        status: [Color32; 4],
    }

    fn roles(palette: &Palette) -> Roles {
        let Palette {
            bg_0,
            bg_1,
            bg_2,
            bg_3,
            text_0,
            text_1,
            text_2,
            amber_fill,
            amber_edge,
            amber_text,
            green,
            blue,
            red,
            purple,
            teal,
            pink,
            olive,
            // Never text, and never a line it matters to see.
            line: _,
            line_soft: _,
            line_no: _,
            amber_dim: _,
            amber_ink: _,
            backdrop: _,
            shadow_dropdown: _,
            shadow_overlay: _,
        } = *palette;
        Roles {
            surfaces: [bg_0, bg_1, bg_2, bg_3],
            text: [
                text_0, text_1, text_2, amber_text, green, blue, red, purple, teal, pink, olive,
            ],
            edges: [amber_edge],
            tinted: [
                amber_fill, green, blue, red, purple, teal, pink, olive, text_1,
            ],
            status: [green, blue, amber_fill, red],
        }
    }

    fn each_theme(check: impl Fn(&str, &Palette)) {
        check("dark", &DARK);
        check("light", &LIGHT);
    }

    #[test]
    fn every_text_colour_is_legible_on_every_surface() {
        each_theme(|theme, palette| {
            let roles = roles(palette);
            for text in roles.text {
                for surface in roles.surfaces {
                    let ratio = contrast(text, surface);
                    assert!(
                        ratio >= AA,
                        "{theme}: {text:?} on {surface:?} is {ratio:.2}:1"
                    );
                }
            }
        });
    }

    #[test]
    fn a_badge_label_is_legible_on_the_tint_it_sits_on() {
        each_theme(|theme, palette| {
            let roles = roles(palette);
            for hue in roles.tinted {
                for surface in roles.surfaces {
                    let tinted = mix(hue, surface, MAX_TINT);
                    let ratio = contrast(palette.text_1, tinted);
                    assert!(
                        ratio >= AA,
                        "{theme}: a badge on a {hue:?} tint is {ratio:.2}:1"
                    );
                }
            }
        });
    }

    #[test]
    fn a_status_is_legible_on_the_fill_of_its_family() {
        each_theme(|theme, palette| {
            let roles = roles(palette);
            for hue in roles.status {
                for surface in roles.surfaces {
                    let filled = mix(hue, surface, STATUS_TINT);
                    let ratio = contrast(palette.text_0, filled);
                    assert!(ratio >= AA, "{theme}: a status on {hue:?} is {ratio:.2}:1");
                }
            }
        });
    }

    #[test]
    fn every_edge_is_findable_on_every_surface() {
        each_theme(|theme, palette| {
            let roles = roles(palette);
            for edge in roles.edges {
                for surface in roles.surfaces {
                    let ratio = contrast(edge, surface);
                    assert!(
                        ratio >= AA_UI,
                        "{theme}: {edge:?} on {surface:?} is {ratio:.2}:1"
                    );
                }
            }
        });
    }

    #[test]
    fn the_ink_is_legible_on_solid_amber() {
        each_theme(|theme, palette| {
            let ratio = contrast(palette.amber_ink, palette.amber_fill);
            assert!(ratio >= AA, "{theme}: the ink on amber is {ratio:.2}:1");
        });
    }

    /// The promise the three-way split was made on.
    #[test]
    fn the_accent_surface_is_one_colour_and_the_dark_theme_does_one_amber() {
        assert_eq!(DARK.amber_fill, LIGHT.amber_fill);
        assert_eq!(DARK.amber_edge, DARK.amber_fill);
        assert_eq!(DARK.amber_text, DARK.amber_fill);
    }

    #[test]
    fn a_hex_reads_as_the_css_writes_it() {
        assert_eq!(hex(0x15171c), Color32::from_rgb(0x15, 0x17, 0x1c));
    }
}
