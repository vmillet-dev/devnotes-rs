//! A place picked in a tree: a filter that keeps the focus over rows of one height, drawn only
//! where they are in view. The popup keeps its size whatever it holds.

use crate::theme::tokens::{font, gap, radius};
use egui::{Color32, CornerRadius, FontFamily, Rect, Role, Sense, Ui, Vec2, WidgetInfo, pos2};
use engine::notes::view::fold;
use i18n::{Language, keys};

use super::menu::{FILTERED_HEIGHT, filter_field, walk, walked_ring};
use super::{hovered, interact, one_line, text, tooltip, towards};
use crate::theme::icons::Icon;
use crate::theme::palette;

const ROW: f32 = 26.0;
const WIDTH: f32 = 300.0;
const INDENT: f32 = 14.0;

#[derive(Clone, Copy)]
pub enum Lead {
    None,
    Icon(Icon),
    Dot(Color32),
}

pub struct Place {
    pub lead: Lead,
    pub name: String,
    /// What it is in, which a search finds it by too.
    pub within: String,
    pub depth: usize,
    pub strong: bool,
    /// Where what is moved is now.
    pub here: bool,
    /// Offered again at the top, at rest: the tree holds it too.
    pub again: bool,
}

impl Place {
    /// Its whole path: what a reader hears, and what the filter reads.
    fn path(&self) -> String {
        if self.within.is_empty() {
            self.name.clone()
        } else {
            format!("{} › {}", self.within, self.name)
        }
    }
}

/// The index of the place picked, the frame it is; the popup it is in closes.
pub fn place_picker(
    ui: &mut Ui,
    places: &[Place],
    (label, author_id): (&str, &str),
    language: Language,
) -> Option<usize> {
    ui.set_width(WIDTH);
    ui.spacing_mut().item_spacing.y = gap::TIGHT;
    let query_id = ui.unique_id().with((author_id, "query"));
    let placeholder = i18n::translate(language, keys::choice_menu::FILTER_PLACEHOLDER, &[]);
    let filter_id = format!("{author_id}-filter");
    let filter = filter_field(ui, query_id, (label, &filter_id, &placeholder));
    let wanted = fold(filter.query.trim());
    let shown: Vec<usize> = (0..places.len())
        .filter(|index| {
            let place = &places[*index];
            wanted.is_empty() || (!place.again && fold(&place.path()).contains(&wanted))
        })
        .collect();
    let here = places.iter().position(|place| place.here && !place.again);
    let walked = walk(
        ui,
        query_id.with("walked"),
        &shown,
        filter.step,
        here.filter(|_| wanted.is_empty()),
    );
    if shown.is_empty() {
        let none = i18n::translate(language, keys::choice_menu::NO_RESULTS, &[]);
        let ink = palette(ui.ctx()).text_2;
        ui.label(egui::RichText::new(none).size(font::LABEL).color(ink));
    }
    let searched = !wanted.is_empty();
    let picked = rows(
        ui,
        places,
        &shown,
        (walked, filter.step != 0, searched),
        author_id,
    );
    let picked = picked.or(walked.filter(|_| filter.entered));
    if picked.is_some() {
        ui.data_mut(|data| data.remove::<String>(query_id));
        ui.close();
    }
    picked
}

fn rows(
    ui: &mut Ui,
    places: &[Place],
    shown: &[usize],
    (walked, moved, searched): (Option<usize>, bool, bool),
    author_id: &str,
) -> Option<usize> {
    ui.spacing_mut().item_spacing.y = 0.0;
    let author_id = format!("{author_id}-place");
    egui::ScrollArea::vertical()
        .id_salt(&author_id)
        .max_height(FILTERED_HEIGHT - 40.0)
        .min_scrolled_height(FILTERED_HEIGHT - 40.0)
        .auto_shrink([false, true])
        .show_rows(ui, ROW, shown.len(), |ui, range| {
            let top = ui.cursor().top();
            let mut picked = None;
            if moved
                && let Some(at) =
                    walked.and_then(|walked| shown.iter().position(|one| *one == walked))
            {
                #[expect(clippy::cast_precision_loss, reason = "a row's rank")]
                let down = (at as f32 - range.start as f32) * ROW;
                let min = pos2(ui.cursor().left(), top + down);
                ui.scroll_to_rect(Rect::from_min_size(min, Vec2::splat(ROW)), None);
            }
            for index in range.filter_map(|at| shown.get(at)) {
                let on = Some(*index) == walked;
                if row(ui, &places[*index], &author_id, (on, searched)) {
                    picked = Some(*index);
                }
            }
            picked
        })
        .inner
}

/// Searched, or offered again, a place says what it is in; in the tree its indent does.
fn row(ui: &mut Ui, place: &Place, author_id: &str, (walked, searched): (bool, bool)) -> bool {
    let p = palette(ui.ctx());
    let corner = CornerRadius::same(radius::CONTROL);
    let path = place.path();
    let response = interact(
        ui,
        Vec2::new(ui.available_width(), ROW),
        Sense::click(),
        author_id,
        || WidgetInfo::selected(Role::MenuItem, true, place.here, &path),
        corner,
    );
    let lit = walked || place.here;
    let hover = hovered(ui, &response).max(if lit { 1.0 } else { 0.0 });
    if hover > 0.0 {
        ui.painter()
            .rect_filled(response.rect, corner, p.bg_3.gamma_multiply(hover));
    }
    if walked {
        walked_ring(ui, response.rect);
    }
    let ink = towards(p.text_1, p.text_0, hover);
    let flat = searched || place.again;
    #[expect(clippy::cast_precision_loss, reason = "a level of nesting")]
    let indent = if flat {
        0.0
    } else {
        place.depth as f32 * INDENT
    };
    let mut left = response.rect.left() + 8.0 + indent;
    let middle = response.rect.center().y;
    match place.lead {
        Lead::None => {}
        Lead::Icon(icon) => {
            let glyph = Rect::from_center_size(pos2(left + 6.0, middle), Vec2::splat(12.0));
            crate::theme::icons::paint(ui, glyph, icon, p.text_2);
            left += 18.0;
        }
        Lead::Dot(colour) => {
            let dot = Rect::from_center_size(pos2(left + 4.0, middle), Vec2::splat(8.0));
            ui.painter()
                .rect_filled(dot, CornerRadius::same(radius::TAG), colour);
            left += 16.0;
        }
    }
    let room = response.rect.right() - 8.0 - left;
    let weight = if place.strong { 600.0 } else { 400.0 };
    let name = one_line(ui, &place.name, (font::LABEL, weight), ink, room + 0.5);
    let mut cut = name.elided;
    let width = name.size().x;
    let at = pos2(left, middle - name.size().y / 2.0);
    ui.painter().galley(at, name, Color32::TRANSPARENT);
    if flat && !place.within.is_empty() {
        let rest = room - width - gap::CONTROLS;
        let within = text(
            ui,
            &place.within,
            font::META,
            400.0,
            FontFamily::Proportional,
            p.text_2,
        );
        if within.size().x <= rest {
            let at = pos2(
                response.rect.right() - 8.0 - within.size().x,
                middle - within.size().y / 2.0,
            );
            ui.painter().galley(at, within, Color32::TRANSPARENT);
        } else {
            cut = true;
        }
    }
    let clicked = response.clicked();
    if cut {
        tooltip(response, &path);
    }
    clicked
}

#[cfg(test)]
mod tests {
    use egui::{Key, Modifiers};
    use egui_kittest::kittest::{NodeT, Queryable};

    use super::*;

    fn place(name: String, within: &str, depth: usize) -> Place {
        Place {
            lead: Lead::None,
            name,
            within: within.to_owned(),
            depth,
            strong: false,
            here: false,
            again: false,
        }
    }

    /// A space of 300 folders, every tenth one nested in the one before it.
    fn library() -> Vec<Place> {
        let mut places = vec![place("Work".to_owned(), "", 0)];
        places.extend((0..300).map(|at| {
            let name = format!("Folder {at} of a name too long to hold on the width of a row");
            let depth = if at % 10 == 9 { 2 } else { 1 };
            place(name, "Work", depth)
        }));
        places
    }

    fn harness() -> egui_kittest::Harness<'static, Option<usize>> {
        let places = library();
        let mut harness = crate::testing::harness()
            .with_size(egui::vec2(600.0, 500.0))
            .build_ui_state(
                move |ui, picked: &mut Option<usize>| {
                    let language = i18n::Language::En;
                    if let Some(chosen) = place_picker(ui, &places, ("Move to", "move"), language) {
                        *picked = Some(chosen);
                    }
                },
                None,
            );
        harness.run();
        harness
    }

    fn rows(harness: &egui_kittest::Harness<'_, Option<usize>>) -> Vec<(String, Rect)> {
        harness
            .query_all_by(|node| node.author_id() == Some("move-place"))
            .map(|row| (row.accesskit_node().label().unwrap_or_default(), row.rect()))
            .collect()
    }

    #[test]
    fn three_hundred_folders_take_the_room_of_a_few_and_only_they_are_drawn() {
        let harness = harness();
        let rows = rows(&harness);
        assert!(rows.len() < 14, "{} rows drawn", rows.len());
        let bottom = rows
            .iter()
            .map(|(_, rect)| rect.bottom())
            .fold(0.0, f32::max);
        assert!(bottom < 300.0, "{bottom}");
        assert!(rows.iter().all(|(_, rect)| rect.width() <= WIDTH));
    }

    #[test]
    fn the_filter_finds_a_folder_by_its_name_and_enter_picks_it() {
        let mut harness = harness();
        harness
            .get_by(|node| node.author_id() == Some("move-filter"))
            .type_text("folder 299");
        harness.run();
        assert_eq!(rows(&harness).len(), 1);
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(*harness.state(), Some(300));
    }

    #[test]
    fn the_arrows_walk_past_the_rows_in_view() {
        let mut harness = harness();
        for _ in 0..30 {
            harness.key_press_modifiers(Modifiers::NONE, Key::ArrowDown);
            harness.run();
        }
        let shown: Vec<String> = rows(&harness).into_iter().map(|(label, _)| label).collect();
        assert!(
            shown
                .iter()
                .any(|label| label.starts_with("Work › Folder 29 ")),
            "{shown:?}"
        );
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(*harness.state(), Some(30));
    }

    #[test]
    fn a_click_picks_the_row_under_it() {
        let mut harness = harness();
        harness
            .query_all_by(|node| node.author_id() == Some("move-place"))
            .nth(2)
            .unwrap()
            .click();
        harness.run();
        assert_eq!(*harness.state(), Some(2));
    }
}
