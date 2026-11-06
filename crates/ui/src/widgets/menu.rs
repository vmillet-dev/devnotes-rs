//! Menus: a list that opens from its control, sub-menus to the side, arrows to move, Escape to
//! close the deepest — and the choice menu, which names its value and filters from nine entries.

use egui::{
    Color32, CornerRadius, FontFamily, Id, Key, Margin, Modifiers, Popup, PopupCloseBehavior,
    RectAlign, Response, Role, Sense, Stroke, Ui, Vec2, WidgetInfo,
};

use engine::notes::view::fold;
use i18n::{Language, keys};

use super::{TextField, hovered, interact, text, towards};
use crate::theme::icons::Icon;
use crate::theme::palette;

/// One entry; one with `children` opens them to its side.
pub struct MenuItem<'a> {
    pub label: &'a str,
    pub author_id: &'a str,
    pub children: &'a [MenuItem<'a>],
    /// Red at rest, like every destructive control.
    pub destructive: bool,
}

impl<'a> MenuItem<'a> {
    pub const fn new(label: &'a str, author_id: &'a str) -> Self {
        Self {
            label,
            author_id,
            children: &[],
            destructive: false,
        }
    }

    #[must_use]
    pub const fn children(mut self, children: &'a [MenuItem<'a>]) -> Self {
        self.children = children;
        self
    }

    #[must_use]
    pub const fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
}

/// The menu `anchor` opens when clicked: the `author_id` of the entry picked, the frame it is.
/// ⚠️ Closed by a click outside, or by the entry picked: a click on an entry that opens a
/// sub-menu would otherwise close the menu it opens from.
pub fn menu<'a>(anchor: &Response, items: &'a [MenuItem<'a>]) -> Option<&'a str> {
    Popup::menu(anchor)
        .frame(frame(&anchor.ctx))
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| entries(ui, items))
        .and_then(|shown| shown.inner)
}

/// The surface a menu is drawn on: `bg-2`, 8 px, 4 px inside, the dropdown's shadow.
pub fn frame(ctx: &egui::Context) -> egui::Frame {
    let p = palette(ctx);
    egui::Frame::new()
        .fill(p.bg_2)
        .stroke(Stroke::new(1.0, p.line_soft))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(4))
        .shadow(p.shadow_dropdown)
}

fn entries<'a>(ui: &mut Ui, items: &'a [MenuItem<'a>]) -> Option<&'a str> {
    ui.set_width(180.0);
    ui.spacing_mut().item_spacing.y = 0.0;
    let mut picked = None;
    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        let response = entry(
            ui,
            item.label,
            item.author_id,
            item.destructive,
            false,
            !item.children.is_empty(),
        );
        ids.push(response.id);
        if item.children.is_empty() {
            if response.clicked() {
                picked = Some(item.author_id);
                ui.close();
            }
            continue;
        }
        let opens = response.clicked()
            || response.hovered()
            || (super::takes_keys(ui, &response)
                && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowRight)));
        // Its own open state: egui keeps one popup in its memory, and the parent is that one.
        let open_id = response.id.with("children");
        let mut open = opens || ui.data(|data| data.get_temp(open_id)).unwrap_or(false);
        let sub = Popup::from_response(&response)
            .id(open_id)
            .align(RectAlign::RIGHT_START)
            .frame(frame(ui.ctx()))
            .open_bool(&mut open)
            .show(|ui| {
                let inner = entries(ui, item.children);
                let back = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowLeft));
                (inner, back)
            });
        let sub = sub.map(|shown| {
            let (inner, back) = shown.inner;
            if back {
                open = false;
                ui.memory_mut(|memory| memory.request_focus(response.id));
            }
            egui::InnerResponse::new(inner, shown.response)
        });
        ui.data_mut(|data| data.insert_temp(open_id, open));
        if let Some(chosen) = sub.and_then(|shown| shown.inner) {
            picked = Some(chosen);
        }
    }
    arrows(ui, &ids);
    picked
}

/// The first entry takes the focus when the menu opens; egui's own arrow navigation moves it
/// from there, entry to entry.
pub fn arrows(ui: &Ui, ids: &[Id]) {
    let Some(first) = ids.first().copied() else {
        return;
    };
    let inside = ui
        .memory(egui::Memory::focused)
        .is_some_and(|focused| ids.contains(&focused));
    if !inside && ui.memory(egui::Memory::focused).is_none() {
        ui.memory_mut(|memory| memory.request_focus(first));
    }
}

/// An entry: 5 × 8, 6 px, 12 px; `bg-3` under the pointer and when it is the value.
pub fn entry(
    ui: &mut Ui,
    label: &str,
    author_id: &str,
    destructive: bool,
    on: bool,
    opens: bool,
) -> Response {
    drawn(ui, None, label, author_id, (destructive, on, opens))
}

/// An entry after its icon, drawn in `colour`, or in the entry's own ink without one.
pub fn icon_entry(
    ui: &mut Ui,
    (icon, colour): (Icon, Option<Color32>),
    label: &str,
    author_id: &str,
    (destructive, opens): (bool, bool),
) -> Response {
    let lead = Some((icon, colour));
    drawn(ui, lead, label, author_id, (destructive, false, opens))
}

fn drawn(
    ui: &mut Ui,
    lead: Option<(Icon, Option<Color32>)>,
    label: &str,
    author_id: &str,
    (destructive, on, opens): (bool, bool, bool),
) -> Response {
    let p = palette(ui.ctx());
    let width = ui.available_width().max(172.0);
    let probe = text(ui, label, 12.0, 400.0, FontFamily::Proportional, p.text_1);
    let corner = CornerRadius::same(6);
    let response = interact(
        ui,
        Vec2::new(width, probe.size().y + 10.0),
        Sense::click(),
        author_id,
        || WidgetInfo::labeled(Role::MenuItem, true, label),
        corner,
    );
    let hover = hovered(ui, &response)
        .max(if response.has_focus() { 1.0 } else { 0.0 })
        .max(if on { 1.0 } else { 0.0 });
    let rest = if destructive { p.red } else { p.text_1 };
    let ink = if destructive {
        rest
    } else {
        towards(rest, p.text_0, hover)
    };
    if hover > 0.0 {
        ui.painter()
            .rect_filled(response.rect, corner, p.bg_3.gamma_multiply(hover));
    }
    let mut left = response.rect.left() + 8.0;
    if let Some((icon, colour)) = lead {
        let glyph = egui::Rect::from_center_size(
            egui::pos2(left + 7.0, response.rect.center().y),
            Vec2::splat(14.0),
        );
        crate::theme::icons::paint(ui, glyph, icon, colour.unwrap_or(ink));
        left += 22.0;
    }
    let galley = text(ui, label, 12.0, 400.0, FontFamily::Proportional, ink);
    let at = egui::pos2(left, response.rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, Color32::TRANSPARENT);
    if opens {
        let glyph = egui::Rect::from_center_size(
            egui::pos2(response.rect.right() - 12.0, response.rect.center().y),
            Vec2::splat(12.0),
        );
        crate::theme::icons::paint(ui, glyph, Icon::ChevronRight, p.text_2);
    }
    response
}

/// The line between two groups of entries.
pub fn line(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 9.0), Sense::hover());
    let colour = palette(ui.ctx()).line_soft;
    ui.painter()
        .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, colour));
}

/// The menus that name their value: `Value` shows the current choice on the control, `Label`
/// keeps the control's own label (an action menu that happens to remember).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Naming {
    Value,
    Label,
}

/// From this many entries, a filter sits at the top and takes the focus.
pub const FILTER_FROM: usize = 9;
/// A filtered menu keeps its size whatever it holds: the entries scroll under the field.
const FILTERED_WIDTH: f32 = 240.0;
const FILTERED_HEIGHT: f32 = 240.0;

pub struct ChoiceMenu<'a, T> {
    label: &'a str,
    author_id: &'a str,
    options: &'a [(T, &'a str)],
    naming: Naming,
    hints: &'a [&'a str],
    language: Option<Language>,
}

impl<'a, T: PartialEq + Copy> ChoiceMenu<'a, T> {
    pub fn new(label: &'a str, author_id: &'a str, options: &'a [(T, &'a str)]) -> Self {
        Self {
            label,
            author_id,
            options,
            naming: Naming::Value,
            hints: &[],
            language: None,
        }
    }

    #[must_use]
    pub fn naming(mut self, naming: Naming) -> Self {
        self.naming = naming;
        self
    }

    /// What each entry is also called, in step with the options: shown at its right, and found
    /// by the filter. An empty one shows nothing.
    #[must_use]
    pub fn hints(mut self, hints: &'a [&'a str]) -> Self {
        self.hints = hints;
        self
    }

    /// The language of the filter's own words, for a menu that may reach `FILTER_FROM`.
    #[must_use]
    pub fn language(mut self, language: Language) -> Self {
        self.language = Some(language);
        self
    }

    fn hint(&self, index: usize) -> &'a str {
        self.hints.get(index).copied().unwrap_or_default()
    }

    /// The control and its menu; the response is changed when another value was picked.
    pub fn show(self, ui: &mut Ui, selected: &mut T) -> Response {
        let p = palette(ui.ctx());
        let current = self
            .options
            .iter()
            .find(|(value, _)| value == selected)
            .map(|(_, name)| *name);
        let shown = match self.naming {
            Naming::Value => current.unwrap_or(self.label),
            Naming::Label => self.label,
        };
        let probe = text(ui, shown, 11.5, 400.0, FontFamily::Proportional, p.text_1);
        let corner = CornerRadius::same(7);
        let label = self.label;
        let mut response = interact(
            ui,
            Vec2::new((probe.size().x + 30.0).min(190.0), probe.size().y + 8.0),
            Sense::click(),
            self.author_id,
            || {
                let mut info = WidgetInfo::labeled(Role::ComboBox, true, label);
                info.current_text_value = Some(shown.to_owned());
                info
            },
            corner,
        );
        let hover = hovered(ui, &response);
        ui.painter().rect(
            response.rect,
            corner,
            p.bg_2,
            Stroke::new(1.0, towards(p.line_soft, p.amber_dim, hover)),
            egui::StrokeKind::Inside,
        );
        let ink = towards(p.text_1, p.amber_text, hover);
        let galley = text(ui, shown, 11.5, 400.0, FontFamily::Proportional, ink);
        let at = egui::pos2(
            response.rect.left() + 8.0,
            response.rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(at, galley, Color32::TRANSPARENT);
        let glyph = egui::Rect::from_center_size(
            egui::pos2(response.rect.right() - 12.0, response.rect.center().y),
            Vec2::splat(11.0),
        );
        crate::theme::icons::paint(ui, glyph, Icon::ChevronDown, ink);

        let query_id = response.id.with("query");
        let filtering = self.options.len() >= FILTER_FROM;
        let picked = Popup::menu(&response)
            .frame(frame(ui.ctx()))
            .close_behavior(PopupCloseBehavior::CloseOnClick)
            .show(|ui| {
                if filtering {
                    self.filtered(ui, query_id, current)
                } else {
                    self.plain(ui, current)
                }
            })
            .and_then(|shown| shown.inner);
        if let Some(value) = picked {
            ui.data_mut(|data| data.remove::<String>(query_id));
            if value != *selected {
                *selected = value;
                response.mark_changed();
            }
        }
        response
    }

    fn plain(&self, ui: &mut Ui, current: Option<&str>) -> Option<T> {
        ui.set_min_width(180.0);
        let mut picked = None;
        let mut ids = Vec::new();
        for (index, (value, name)) in self.options.iter().enumerate() {
            let id = format!("{}-{index}", self.author_id);
            let entry = entry(ui, name, &id, false, Some(*name) == current, false);
            ids.push(entry.id);
            if entry.clicked() {
                picked = Some(*value);
            }
        }
        arrows(ui, &ids);
        picked
    }

    /// The field stays put and keeps the focus: the arrows walk the entries from it, Enter picks
    /// the one walked to, Escape empties the field before it closes the menu.
    fn filtered(&self, ui: &mut Ui, query_id: Id, current: Option<&str>) -> Option<T> {
        ui.set_width(FILTERED_WIDTH);
        ui.spacing_mut().item_spacing.y = 4.0;
        let walked_id = query_id.with("walked");
        let mut query: String = ui.data(|data| data.get_temp(query_id)).unwrap_or_default();
        let emptied = !query.is_empty()
            && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
        if emptied {
            query.clear();
        }
        // Read before the field: it would lose the focus to Enter, and walk its caret.
        let (step, entered) = ui.input_mut(|input| {
            let down = input.consume_key(Modifiers::NONE, Key::ArrowDown);
            let up = input.consume_key(Modifiers::NONE, Key::ArrowUp);
            let enter = input.consume_key(Modifiers::NONE, Key::Enter);
            (isize::from(down) - isize::from(up), enter)
        });
        let filter_id = format!("{}-filter", self.author_id);
        let placeholder = self.words(keys::choice_menu::FILTER_PLACEHOLDER);
        let field = TextField::new(&mut query, self.label, &filter_id)
            .hint(&placeholder)
            .compact()
            .show(ui);
        if !field.has_focus() && ui.memory(|memory| memory.focused().is_none()) {
            field.request_focus();
        }
        let shown = self.matching(&query);
        let walked = self.walk(ui, walked_id, &shown, step, current);
        ui.data_mut(|data| data.insert_temp(query_id, query));
        if shown.is_empty() {
            let none = self.words(keys::choice_menu::NO_RESULTS);
            let ink = palette(ui.ctx()).text_2;
            ui.label(egui::RichText::new(none).size(12.0).color(ink));
        }
        let picked = egui::ScrollArea::vertical()
            .max_height(FILTERED_HEIGHT - field.rect.height() - 4.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                self.rows(ui, &shown, (walked, step != 0), current)
            })
            .inner;
        if picked.is_some() {
            return picked;
        }
        if entered {
            ui.close();
            return walked.map(|index| self.options[index].0);
        }
        None
    }

    /// The indices of the entries `query` finds, in the name or in the hint.
    fn matching(&self, query: &str) -> Vec<usize> {
        let wanted = fold(query.trim());
        (0..self.options.len())
            .filter(|index| {
                wanted.is_empty()
                    || fold(self.options[*index].1).contains(&wanted)
                    || fold(self.hint(*index)).contains(&wanted)
            })
            .collect()
    }

    /// The entry the arrows stand on: the one they left, or the current value, or the first.
    fn walk(
        &self,
        ui: &mut Ui,
        walked_id: Id,
        shown: &[usize],
        step: isize,
        current: Option<&str>,
    ) -> Option<usize> {
        let held: Option<usize> = ui.data(|data| data.get_temp(walked_id));
        let start = held
            .and_then(|walked| shown.iter().position(|index| *index == walked))
            .or_else(|| {
                shown
                    .iter()
                    .position(|index| Some(self.options[*index].1) == current)
            })
            .unwrap_or(0);
        let last = shown.len().saturating_sub(1);
        let walked = shown
            .get(start.saturating_add_signed(step).min(last))
            .copied();
        ui.data_mut(|data| {
            if let Some(walked) = walked {
                data.insert_temp(walked_id, walked);
            }
        });
        walked
    }

    fn rows(
        &self,
        ui: &mut Ui,
        shown: &[usize],
        (walked, moved): (Option<usize>, bool),
        current: Option<&str>,
    ) -> Option<T> {
        let mut picked = None;
        for index in shown {
            let (value, name) = self.options[*index];
            let id = format!("{}-{index}", self.author_id);
            let here = Some(*index) == walked;
            let row = entry(ui, name, &id, false, here || Some(name) == current, false);
            self.paint_hint(ui, row.rect, *index);
            if here {
                walked_ring(ui, row.rect);
                if moved {
                    row.scroll_to_me(None);
                }
            }
            if row.clicked() {
                picked = Some(value);
            }
        }
        picked
    }

    fn words(&self, key: i18n::Key) -> String {
        self.language
            .map_or_else(String::new, |language| i18n::translate(language, key, &[]))
    }

    fn paint_hint(&self, ui: &Ui, row: egui::Rect, index: usize) {
        let hint = self.hint(index);
        if hint.is_empty() {
            return;
        }
        let ink = palette(ui.ctx()).text_2;
        let galley = text(ui, hint, 11.0, 400.0, FontFamily::Proportional, ink);
        let at = egui::pos2(
            row.right() - 8.0 - galley.size().x,
            row.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(at, galley, Color32::TRANSPARENT);
    }
}

/// The entry walked to: the focus stays in the field, so its ring is drawn by hand.
fn walked_ring(ui: &Ui, row: egui::Rect) {
    ui.painter().rect_stroke(
        row,
        CornerRadius::same(6),
        Stroke::new(1.0, palette(ui.ctx()).amber_edge),
        egui::StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use egui_kittest::kittest::Queryable;

    use super::*;
    use crate::widgets::Button;

    const EXPORT: [MenuItem<'static>; 2] = [
        MenuItem::new("Markdown", "export-markdown"),
        MenuItem::new("JSON", "export-json"),
    ];
    const ITEMS: [MenuItem<'static>; 3] = [
        MenuItem::new("Renommer", "rename"),
        MenuItem::new("Exporter", "export").children(&EXPORT),
        MenuItem::new("Supprimer", "delete").destructive(),
    ];

    fn harness() -> egui_kittest::Harness<'static, Option<String>> {
        crate::testing::harness()
            .with_size(egui::vec2(600.0, 400.0))
            .build_ui_state(
                |ui, picked: &mut Option<String>| {
                    let anchor = Button::new("Actions", "actions").show(ui);
                    if let Some(chosen) = menu(&anchor, &ITEMS) {
                        *picked = Some(chosen.to_string());
                    }
                },
                None,
            )
    }

    fn open(harness: &mut egui_kittest::Harness<'static, Option<String>>) {
        harness.run();
        harness
            .get_by(|node| node.author_id() == Some("actions"))
            .click();
        harness.run();
    }

    #[test]
    fn the_arrows_walk_the_entries_and_enter_picks_one() {
        let mut harness = harness();
        open(&mut harness);
        harness.key_press(Key::ArrowDown);
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();

        assert_eq!(harness.state().as_deref(), Some("delete"));
    }

    #[test]
    fn the_right_arrow_opens_a_sub_menu() {
        let mut harness = harness();
        open(&mut harness);

        harness.key_press(Key::ArrowDown);
        harness.run();
        harness.key_press(Key::ArrowRight);
        harness.run();

        assert!(
            harness
                .query_by(|node| node.author_id() == Some("export-json"))
                .is_some()
        );
    }

    #[test]
    fn escape_closes_the_menu() {
        let mut harness = harness();
        open(&mut harness);
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("rename"))
                .is_some()
        );

        harness.key_press(Key::Escape);
        harness.run();

        assert!(
            harness
                .query_by(|node| node.author_id() == Some("rename"))
                .is_none()
        );
    }

    #[test]
    fn nine_choices_bring_a_filter() {
        const NAMES: [&str; 9] = [
            "UTC", "Paris", "Londres", "Tokyo", "Lima", "Oslo", "Rome", "Dakar", "Sydney",
        ];
        let options: Vec<(usize, &str)> = NAMES.iter().copied().enumerate().collect();
        let mut harness = crate::testing::harness()
            .with_size(egui::vec2(600.0, 600.0))
            .build_ui_state(
                move |ui, selected: &mut usize| {
                    ChoiceMenu::new("Fuseau", "zone", &options).show(ui, selected);
                },
                0,
            );
        harness.run();
        harness
            .get_by(|node| node.author_id() == Some("zone"))
            .click();
        harness.run();

        harness
            .get_by(|node| node.author_id() == Some("zone-filter"))
            .type_text("lo");
        harness.run();
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("zone-2"))
                .is_some()
        );
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("zone-3"))
                .is_none()
        );

        harness
            .get_by(|node| node.author_id() == Some("zone-5"))
            .click();
        harness.run();
        assert_eq!(*harness.state(), 5);
    }

    const LANGUAGES: [(usize, &str); 12] = [
        (0, "JSON"),
        (1, "JS"),
        (2, "TS"),
        (3, "PY"),
        (4, "RS"),
        (5, "GO"),
        (6, "SQL"),
        (7, "XML"),
        (8, "CSS"),
        (9, "SH"),
        (10, "MD"),
        (11, "TXT"),
    ];
    const NAMES: [&str; 12] = [
        "",
        "JavaScript",
        "TypeScript",
        "Python",
        "Rust",
        "Go",
        "",
        "",
        "",
        "Shell",
        "Markdown",
        "",
    ];

    fn opened() -> egui_kittest::Harness<'static, usize> {
        let mut harness = crate::testing::harness()
            .with_size(egui::vec2(900.0, 900.0))
            .build_ui_state(
                |ui, selected: &mut usize| {
                    ChoiceMenu::new("Format", "format", &LANGUAGES)
                        .hints(&NAMES)
                        .show(ui, selected);
                },
                0,
            );
        harness.run();
        harness
            .get_by(|node| node.author_id() == Some("format"))
            .click();
        harness.run();
        harness.run();
        harness
    }

    #[test]
    fn a_filter_finds_an_entry_by_its_hint() {
        let mut harness = opened();
        harness
            .get_by(|node| node.author_id() == Some("format-filter"))
            .type_text("rus");
        harness.run();
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("format-4"))
                .is_some()
        );
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("format-3"))
                .is_none()
        );
    }

    #[test]
    fn a_filtered_menu_keeps_its_size_and_scrolls() {
        let harness = opened();
        let filter = harness.get_by(|node| node.author_id() == Some("format-filter"));
        assert!(
            filter.rect().width() <= FILTERED_WIDTH,
            "{:?}",
            filter.rect()
        );
        let last = harness.get_by(|node| node.author_id() == Some("format-11"));
        let bottom = filter.rect().top() + FILTERED_HEIGHT;
        assert!(
            last.rect().bottom() > bottom,
            "the last entry lies under the fold"
        );
    }

    #[test]
    fn the_arrows_walk_from_the_filter_and_enter_picks() {
        let mut harness = opened();
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(*harness.state(), 2);
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("format-filter"))
                .is_none(),
            "the menu closed"
        );
    }

    #[test]
    fn escape_empties_the_filter_before_it_closes_the_menu() {
        let mut harness = opened();
        harness
            .get_by(|node| node.author_id() == Some("format-filter"))
            .type_text("rus");
        harness.run();
        harness.key_press(Key::Escape);
        harness.run();
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("format-3"))
                .is_some(),
            "every entry is back"
        );
        harness.key_press(Key::Escape);
        harness.run();
        assert!(
            harness
                .query_by(|node| node.author_id() == Some("format-filter"))
                .is_none()
        );
    }
}
