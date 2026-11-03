//! The look of the application (plan § 3.7): the reference's tokens, the two themes and the two
//! densities, and the egui `Style` drawn from them. A widget reads its colours from
//! [`palette`] and its gaps from [`spacing`], never a literal.

pub mod motion;
pub mod tokens;

use egui::{Color32, Context, Id, Stroke, Style, SystemTheme, Theme, ThemePreference, Visuals};

use tokens::{COMFORTABLE, COMPACT, DARK, LIGHT, Palette, Spacing};

/// The theme preference: « system » follows the operating system, live.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
}

impl Appearance {
    fn preference(self) -> ThemePreference {
        match self {
            Self::System => ThemePreference::System,
            Self::Dark => ThemePreference::Dark,
            Self::Light => ThemePreference::Light,
        }
    }

    /// The window's own title bar, kept in step.
    fn title_bar(self) -> SystemTheme {
        match self {
            Self::System => SystemTheme::SystemDefault,
            Self::Dark => SystemTheme::Dark,
            Self::Light => SystemTheme::Light,
        }
    }
}

/// Both themes' styles are set at once: egui switches between them by itself when the system
/// does, for a « system » preference.
pub fn apply(ctx: &Context, appearance: Appearance, density: Density) {
    ctx.set_style_of(Theme::Dark, style(&DARK, Theme::Dark));
    ctx.set_style_of(Theme::Light, style(&LIGHT, Theme::Light));
    ctx.set_theme(appearance.preference());
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(appearance.title_bar()));
    ctx.data_mut(|data| data.insert_temp(density_id(), density));
}

/// The colours of the theme on screen.
pub fn palette(ctx: &Context) -> &'static Palette {
    match ctx.theme() {
        Theme::Dark => &DARK,
        Theme::Light => &LIGHT,
    }
}

pub fn spacing(ctx: &Context) -> &'static Spacing {
    match ctx.data(|data| data.get_temp(density_id()).unwrap_or_default()) {
        Density::Comfortable => &COMFORTABLE,
        Density::Compact => &COMPACT,
    }
}

fn density_id() -> Id {
    Id::unique("theme.density")
}

/// egui's own widgets restyled after the reference; the visible components are painted by
/// hand from the palette, and only lean on this for what egui draws itself.
fn style(palette: &Palette, theme: Theme) -> Style {
    Style {
        visuals: visuals(palette, theme),
        ..Style::default()
    }
}

fn visuals(palette: &Palette, theme: Theme) -> Visuals {
    let mut visuals = match theme {
        Theme::Dark => Visuals::dark(),
        Theme::Light => Visuals::light(),
    };
    let line = |colour: Color32| Stroke::new(1.0, colour);

    visuals.panel_fill = palette.bg_0;
    visuals.window_fill = palette.bg_1;
    visuals.faint_bg_color = palette.bg_1;
    visuals.extreme_bg_color = palette.bg_1;
    visuals.code_bg_color = palette.bg_2;
    visuals.window_stroke = line(palette.line);
    visuals.window_shadow = palette.shadow_overlay;
    visuals.popup_shadow = palette.shadow_dropdown;
    visuals.hyperlink_color = palette.amber_text;
    visuals.warn_fg_color = palette.amber_text;
    visuals.error_fg_color = palette.red;
    visuals.selection.bg_fill = palette.amber_fill.gamma_multiply(0.35);
    visuals.selection.stroke = line(palette.text_0);
    visuals.text_cursor.stroke.color = palette.text_0;

    let widgets = &mut visuals.widgets;
    for (state, fill, edge, text) in [
        (
            &mut widgets.noninteractive,
            palette.bg_1,
            palette.line_soft,
            palette.text_1,
        ),
        (
            &mut widgets.inactive,
            palette.bg_2,
            palette.line,
            palette.text_0,
        ),
        (
            &mut widgets.hovered,
            palette.bg_3,
            palette.amber_dim,
            palette.text_0,
        ),
        (
            &mut widgets.active,
            palette.bg_3,
            palette.amber_edge,
            palette.text_0,
        ),
        (
            &mut widgets.open,
            palette.bg_2,
            palette.line,
            palette.text_0,
        ),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = line(edge);
        state.fg_stroke = line(text);
    }
    visuals
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;

    use super::*;

    fn harness(system: Theme) -> Harness<'static> {
        Harness::builder().with_theme(system).build_ui(|ui| {
            ui.label("·");
        })
    }

    #[test]
    fn a_chosen_theme_draws_its_palette() {
        let mut harness = harness(Theme::Light);

        apply(&harness.ctx, Appearance::Dark, Density::Comfortable);
        harness.step();
        assert_eq!(palette(&harness.ctx), &DARK);
        assert_eq!(harness.ctx.global_style().visuals.panel_fill, DARK.bg_0);

        apply(&harness.ctx, Appearance::Light, Density::Comfortable);
        harness.step();
        assert_eq!(palette(&harness.ctx), &LIGHT);
        assert_eq!(harness.ctx.global_style().visuals.panel_fill, LIGHT.bg_0);
    }

    #[test]
    fn the_system_theme_is_followed() {
        for (system, expected) in [(Theme::Dark, &DARK), (Theme::Light, &LIGHT)] {
            let mut harness = harness(Theme::Dark);
            harness.input_mut().system_theme = Some(system);

            apply(&harness.ctx, Appearance::System, Density::Comfortable);
            harness.step();

            assert_eq!(palette(&harness.ctx), expected);
        }
    }

    #[test]
    fn the_title_bar_is_told_the_theme() {
        let mut harness = harness(Theme::Dark);

        apply(&harness.ctx, Appearance::Light, Density::Comfortable);
        harness.step();

        let commands: Vec<&egui::ViewportCommand> = harness
            .output()
            .viewport_output
            .values()
            .flat_map(|viewport| &viewport.commands)
            .collect();
        assert!(commands.contains(&&egui::ViewportCommand::SetTheme(SystemTheme::Light)));
    }

    #[test]
    fn compact_tightens_the_four_gaps_and_nothing_else() {
        let harness = harness(Theme::Dark);
        assert_eq!(spacing(&harness.ctx), &COMFORTABLE);

        apply(&harness.ctx, Appearance::Dark, Density::Compact);

        assert_eq!(spacing(&harness.ctx), &COMPACT);
    }
}
