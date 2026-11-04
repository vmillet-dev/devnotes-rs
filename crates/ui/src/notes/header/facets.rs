//! The rails under the header: kinds, priorities, tags, formats, and the way out of them. Shown
//! while wanted, and forced open while a chip is pressed: a filter nobody can see is a filter
//! nobody can undo. The counts are the space's, from the core: pressing one chip zeroes no other.

use egui::{
    Color32, CornerRadius, FontFamily, Galley, Margin, Rect, Role, Sense, Stroke, StrokeKind, Ui,
    Vec2, WidgetInfo, pos2, vec2,
};
use engine::notes::kind::NoteKind;
use engine::notes::priority::Priority;
use engine::notes::view::{FacetCount, NotesView};
use i18n::{Key, keys};

use crate::action::{Action, Actions};
use crate::state::AppState;
use crate::state::notes_query::{Facet, Facets};
use crate::theme::{palette, spacing};
use crate::widgets::{self, hovered, interact, towards};

pub(super) fn show(ui: &mut Ui, state: &AppState, actions: &mut Actions) {
    let Some(view) = state.notes_query.view() else {
        return;
    };
    let facets = state.notes_query.facets();
    let language = state.language;
    kinds(ui, view, facets, language, actions);
    priorities(ui, view, facets, language, actions);
    tags(ui, view, facets, language, actions);
    languages(ui, view, facets, language, actions);
    if state.notes_query.has_user_filters() {
        rail(ui, |ui| {
            let label = i18n::translate(language, keys::notes::CLEAR_FILTERS, &[]);
            if widgets::Button::new(&label, "clear-filters")
                .icon(crate::theme::icons::Icon::X)
                .kind(widgets::Kind::Quiet)
                .show(ui)
                .clicked()
            {
                press(actions, Facet::Clear);
            }
        });
    }
}

fn press(actions: &mut Actions, facet: Facet) {
    actions.push(Action::Notes(crate::notes::Action::Facet(facet)));
}

/// One row, its label first, a hairline under it.
fn rail(ui: &mut Ui, draw: impl FnOnce(&mut Ui)) {
    #[expect(clippy::cast_possible_truncation, reason = "a few points")]
    let side = spacing(ui.ctx()).canvas.round() as i8;
    let p = palette(ui.ctx());
    let shown = egui::Frame::new()
        .inner_margin(Margin::symmetric(side, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 7.0;
                draw(ui);
            });
        });
    let rect = shown.response.rect;
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, p.line_soft));
}

fn label(ui: &mut Ui, words: &str) {
    let p = palette(ui.ctx());
    let galley = widgets::text(ui, words, 11.0, 400.0, FontFamily::Proportional, p.text_2);
    let (rect, _) = ui.allocate_exact_size(galley.size() + vec2(2.0, 0.0), Sense::hover());
    ui.painter().galley(
        pos2(rect.left(), rect.center().y - galley.size().y / 2.0),
        galley,
        p.text_2,
    );
}

/// What a chip carries before its words: a priority's dot, a format's badge.
enum Lead {
    None,
    Dot(Color32),
    Badge(String, Color32),
}

/// A rail's chip: the words, then the count. Empty and not pressed, it stays in its place but
/// cannot be pressed.
struct Chip<'a> {
    words: &'a str,
    count: Option<u32>,
    lead: Lead,
    on: bool,
    enabled: bool,
}

impl Chip<'_> {
    fn show(self, ui: &mut Ui, author_id: &str) -> bool {
        let p = palette(ui.ctx());
        let mono = FontFamily::Monospace;
        let words = widgets::text(ui, self.words, 11.0, 400.0, mono.clone(), p.text_1);
        let count = self
            .count
            .map(|count| widgets::text(ui, &count.to_string(), 11.0, 400.0, mono, p.text_2));
        let lead = lead_width(ui, &self.lead);
        let counted = count.as_ref().map_or(0.0, |galley| galley.size().x + 6.0);
        let size = vec2(lead + words.size().x + counted + 20.0, 24.0);
        let (on, enabled, label) = (self.on, self.enabled, self.words);
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let corner = CornerRadius::same(12);
        let response = interact(
            ui,
            size,
            sense,
            author_id,
            || WidgetInfo::selected(Role::Button, enabled, on, label),
            corner,
        );
        let lit = if on {
            1.0
        } else if enabled {
            hovered(ui, &response)
        } else {
            0.0
        };
        let fade = |colour: Color32| {
            if enabled {
                colour
            } else {
                colour.gamma_multiply(0.5)
            }
        };
        let rect = response.rect;
        let painter = ui.painter();
        painter.rect(
            rect,
            corner,
            p.bg_2,
            Stroke::new(1.0, fade(towards(p.line_soft, p.amber_dim, lit))),
            StrokeKind::Inside,
        );
        let ink = fade(towards(p.text_1, p.amber_text, lit));
        let mut left = rect.left() + 10.0;
        paint_lead(ui, &self.lead, left, rect.center().y);
        left += lead;
        let words = widgets::text(ui, self.words, 11.0, 400.0, FontFamily::Monospace, ink);
        paint(ui, left, rect, words, ink);
        if let Some(count) = count {
            paint(
                ui,
                left + count_at(ui, self.words),
                rect,
                count,
                fade(p.text_2),
            );
        }
        response.clicked()
    }
}

fn count_at(ui: &Ui, words: &str) -> f32 {
    widgets::text(
        ui,
        words,
        11.0,
        400.0,
        FontFamily::Monospace,
        Color32::WHITE,
    )
    .size()
    .x + 6.0
}

fn paint(ui: &Ui, left: f32, rect: Rect, galley: std::sync::Arc<Galley>, ink: Color32) {
    let at = pos2(left, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, ink);
}

fn lead_width(ui: &Ui, lead: &Lead) -> f32 {
    match lead {
        Lead::None => 0.0,
        Lead::Dot(_) => 8.0 + 6.0,
        Lead::Badge(words, _) => badge_size(ui, words).x + 6.0,
    }
}

fn badge_size(ui: &Ui, words: &str) -> Vec2 {
    let galley = widgets::text(ui, words, 9.5, 700.0, FontFamily::Monospace, Color32::WHITE);
    galley.size() + vec2(12.0, 4.0)
}

fn paint_lead(ui: &Ui, lead: &Lead, left: f32, middle: f32) {
    let p = palette(ui.ctx());
    match lead {
        Lead::None => {}
        Lead::Dot(colour) => {
            ui.painter()
                .circle_filled(pos2(left + 4.0, middle), 4.0, *colour);
        }
        Lead::Badge(words, hue) => {
            let size = badge_size(ui, words);
            let rect = Rect::from_min_size(pos2(left, middle - size.y / 2.0), size);
            ui.painter().rect(
                rect,
                CornerRadius::same(4),
                p.bg_3,
                Stroke::new(1.0, hue.gamma_multiply(0.4)),
                StrokeKind::Inside,
            );
            let galley = widgets::text(ui, words, 9.5, 700.0, FontFamily::Monospace, *hue);
            ui.painter().galley(
                pos2(left + 6.0, middle - galley.size().y / 2.0),
                galley,
                *hue,
            );
        }
    }
}

fn total<T>(counts: &[FacetCount<T>]) -> u32 {
    counts.iter().map(|counted| counted.count).sum()
}

/// « All », then each kind with what it holds in the space.
fn kinds(
    ui: &mut Ui,
    view: &NotesView,
    facets: &Facets,
    language: i18n::Language,
    actions: &mut Actions,
) {
    let every = facets.kinds.is_empty();
    let all = total(&view.kind_counts);
    if all == 0 && every {
        return;
    }
    let tr = |key| i18n::translate(language, key, &[]);
    rail(ui, |ui| {
        label(ui, &tr(keys::notes::KINDS));
        let words = tr(keys::notes::KIND_ALL);
        let chip = Chip {
            words: &words,
            count: Some(all),
            lead: Lead::None,
            on: every,
            enabled: true,
        };
        if chip.show(ui, "kind-chip-all") {
            press(actions, Facet::AllKinds);
        }
        for counted in &view.kind_counts {
            let on = facets.kinds.contains(&counted.value);
            let words = tr(kind_key(counted.value));
            let chip = Chip {
                words: &words,
                count: Some(counted.count),
                lead: Lead::None,
                on,
                enabled: on || counted.count > 0,
            };
            if chip.show(ui, &format!("kind-chip-{}", counted.value.as_str())) {
                press(actions, Facet::Kind(counted.value));
            }
        }
    });
}

fn kind_key(kind: NoteKind) -> Key {
    match kind {
        NoteKind::Snippet => keys::notes::kind_plural::SNIPPET,
        NoteKind::Note => keys::notes::kind_plural::NOTE,
        NoteKind::Checklist => keys::notes::kind_plural::CHECKLIST,
        NoteKind::Calc => keys::notes::kind_plural::CALC,
        NoteKind::Drawing => keys::notes::kind_plural::DRAWING,
    }
}

/// The levels from the most pressing down; « none » has no chip, and the rail stays away from
/// a space where nothing has a priority.
fn priorities(
    ui: &mut Ui,
    view: &NotesView,
    facets: &Facets,
    language: i18n::Language,
    actions: &mut Actions,
) {
    let every = facets.priorities.is_empty();
    let levels: Vec<&FacetCount<Priority>> = view
        .priority_counts
        .iter()
        .filter(|counted| counted.value != Priority::None)
        .rev()
        .collect();
    if every && levels.iter().all(|counted| counted.count == 0) {
        return;
    }
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    rail(ui, |ui| {
        label(ui, &tr(keys::notes::PRIORITIES));
        let words = tr(keys::notes::PRIORITY_ALL);
        let all = total(&view.priority_counts);
        let chip = Chip {
            words: &words,
            count: Some(all),
            lead: Lead::None,
            on: every,
            enabled: true,
        };
        if chip.show(ui, "priority-chip-all") {
            press(actions, Facet::AllPriorities);
        }
        for counted in levels {
            let on = facets.priorities.contains(&counted.value);
            let (key, dot) = match counted.value {
                Priority::Urgent => (keys::notes::priority::URGENT, p.red),
                Priority::High => (keys::notes::priority::HIGH, p.amber_text),
                Priority::Medium => (keys::notes::priority::MEDIUM, p.blue),
                Priority::Low | Priority::None => (keys::notes::priority::LOW, p.text_2),
            };
            let words = tr(key);
            let chip = Chip {
                words: &words,
                count: Some(counted.count),
                lead: Lead::Dot(dot),
                on,
                enabled: on || counted.count > 0,
            };
            if chip.show(ui, &format!("priority-chip-{}", counted.value.as_str())) {
                press(actions, Facet::Priority(counted.value));
            }
        }
    });
}

/// The space's tags with what each holds; only the pills scroll.
fn tags(
    ui: &mut Ui,
    view: &NotesView,
    facets: &Facets,
    language: i18n::Language,
    actions: &mut Actions,
) {
    if view.available_tags.is_empty() {
        return;
    }
    rail(ui, |ui| {
        label(ui, &i18n::translate(language, keys::notes::TAGS, &[]));
        egui::ScrollArea::horizontal()
            .id_salt("tag-rail")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 7.0;
                    for tag in &view.available_tags {
                        let count = view
                            .tag_counts
                            .iter()
                            .find(|counted| &counted.value == tag)
                            .map(|counted| counted.count);
                        let words = format!("#{tag}");
                        let chip = Chip {
                            words: &words,
                            count,
                            lead: Lead::None,
                            on: facets.tags.contains(tag),
                            enabled: true,
                        };
                        if chip.show(ui, "tag-pill") {
                            press(actions, Facet::Tag(tag.clone()));
                        }
                    }
                });
            });
    });
}

/// The badge carries the format's hue; tinted amber when pressed, it would lose the very clue
/// it is known by, so the chip's edge says it instead.
fn languages(
    ui: &mut Ui,
    view: &NotesView,
    facets: &Facets,
    language: i18n::Language,
    actions: &mut Actions,
) {
    if view.available_languages.is_empty() {
        return;
    }
    rail(ui, |ui| {
        label(ui, &i18n::translate(language, keys::notes::LANGUAGES, &[]));
        for format in &view.available_languages {
            let count = view
                .language_counts
                .iter()
                .find(|counted| counted.value == *format)
                .map(|counted| counted.count);
            let hue = crate::notes::ui::language_hue(ui, *format);
            let badge = crate::notes::language::label(*format).to_owned();
            let name = crate::notes::language::name(*format);
            if language_chip(
                ui,
                (badge, hue),
                name,
                count,
                facets.languages.contains(format),
            ) {
                press(actions, Facet::Language(*format));
            }
        }
    });
}

fn language_chip(
    ui: &mut Ui,
    (badge, hue): (String, Color32),
    name: &str,
    count: Option<u32>,
    on: bool,
) -> bool {
    let p = palette(ui.ctx());
    let size = badge_size(ui, &badge);
    let count = count.map(|count| {
        widgets::text(
            ui,
            &count.to_string(),
            11.0,
            400.0,
            FontFamily::Monospace,
            p.text_2,
        )
    });
    let counted = count.as_ref().map_or(0.0, |galley| galley.size().x + 9.0);
    let corner = CornerRadius::same(6);
    let response = interact(
        ui,
        vec2(size.x + 4.0 + counted, size.y + 4.0),
        Sense::click(),
        "language-chip",
        || WidgetInfo::selected(Role::Button, true, on, name),
        corner,
    );
    let rect = response.rect;
    let edge = if on {
        p.amber_edge
    } else if response.hovered() {
        p.line
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect(
        rect,
        corner,
        Color32::TRANSPARENT,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    paint_lead(
        ui,
        &Lead::Badge(badge, hue),
        rect.left() + 2.0,
        rect.center().y,
    );
    if let Some(count) = count {
        paint(ui, rect.left() + 2.0 + size.x + 5.0, rect, count, p.text_2);
    }
    response.clicked()
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};
    use engine::notes::kind::NoteKind;
    use engine::notes::model::NotePatch;

    use crate::notes::bench::{Bench, area, cards, click_id, settle};
    use crate::testing;

    /// A tagged snippet, a Note, a plain snippet; the rails open.
    fn three() -> Harness<'static, Bench> {
        let (backend, profile) = testing::unlocked();
        let tagged = testing::note(&backend, "tagged").id.clone();
        let prose = testing::note(&backend, "prose").id.clone();
        testing::note(&backend, "plain");
        let patch = |tags: Option<Vec<String>>, kind: Option<NoteKind>| NotePatch {
            tags,
            kind,
            ..NotePatch::default()
        };
        let (ops, note) = (
            patch(Some(vec!["ops".into()]), None),
            patch(None, Some(NoteKind::Note)),
        );
        engine::db::run_here(backend.engine(), move |engine| {
            engine::notes::update_note(engine, tagged, ops)?;
            engine::notes::update_note(engine, prose, note)
        })
        .unwrap();
        let mut harness = area(backend, profile);
        click_id(&mut harness, "facets-toggle");
        harness
    }

    fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
        harness
            .query_by(|node| node.author_id() == Some(author_id))
            .is_some()
    }

    #[test]
    fn a_kind_narrows_the_cards_and_all_gives_them_back() {
        let mut harness = three();
        click_id(&mut harness, "kind-chip-note");
        assert_eq!(cards(&harness), 1);
        let toggle = harness.get_by(|node| node.author_id() == Some("facets-toggle"));
        assert!(
            toggle.accesskit_node().is_disabled(),
            "it holds open while a chip is pressed"
        );
        assert!(has(&harness, "facets-count"));

        click_id(&mut harness, "kind-chip-all");

        assert_eq!(cards(&harness), 3);
        assert!(!has(&harness, "facets-count"));
    }

    #[test]
    fn a_tag_narrows_and_one_click_clears_it_with_the_search() {
        let mut harness = three();
        click_id(&mut harness, "tag-pill");
        assert_eq!(cards(&harness), 1);

        click_id(&mut harness, "clear-filters");

        assert_eq!(cards(&harness), 3);
        assert!(!has(&harness, "clear-filters"));
    }

    #[test]
    fn escape_lets_the_chips_go() {
        let mut harness = three();
        click_id(&mut harness, "kind-chip-snippet");
        assert_eq!(cards(&harness), 2);

        harness.key_press(egui::Key::Escape);
        settle(&mut harness);

        assert_eq!(cards(&harness), 3);
    }

    #[test]
    fn folded_the_rails_are_gone() {
        let mut harness = three();
        click_id(&mut harness, "facets-toggle");
        assert!(!has(&harness, "kind-chip-all"));
    }
}
