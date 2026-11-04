//! The offer to put back the last write, at the foot of the window while its timer runs. Above
//! every modal: shown while the editor is open, under its backdrop its first click would close
//! the editor.

use egui::{
    Align2, CornerRadius, FontFamily, Margin, Order, Role, Sense, Stroke, StrokeKind, Ui,
    WidgetInfo, vec2,
};
use i18n::{Key, keys};

use crate::action::{Action, Actions};
use crate::api::undo::Reversible;
use crate::state::AppState;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, IconButton};

pub(in crate::notes) fn show(ui: &Ui, state: &AppState, actions: &mut Actions) {
    let now = ui.input(|input| input.time);
    let Some(action) = state.undo.banner(now) else {
        return;
    };
    if let Some(left) = state.undo.banner_left(now) {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(left));
    }
    let p = palette(ui.ctx());
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(10))
        .shadow(p.shadow_overlay)
        .inner_margin(Margin::symmetric(12, 6));
    egui::Area::new(egui::Id::unique("undo-bar"))
        .order(Order::Tooltip)
        .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -20.0))
        .show(ui.ctx(), |ui| {
            let bar = frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    message(ui, action, state.language);
                    restore(ui, state.language, actions);
                    let dismiss = i18n::translate(state.language, keys::undo::DISMISS, &[]);
                    if IconButton::new(Icon::X, &dismiss, "undo-dismiss")
                        .show(ui)
                        .clicked()
                    {
                        actions.push(Action::Notes(crate::notes::Action::DismissUndo));
                    }
                });
            });
            // `status`, not `alert`: a screen reader must not interrupt for this.
            let response = bar.response;
            response.widget_info(|| WidgetInfo::labeled(Role::Status, true, ""));
            widgets::address(ui, &response, "undo-bar");
        });
}

fn message(ui: &mut Ui, action: &Reversible, language: i18n::Language) {
    let p = palette(ui.ctx());
    let count = u32::try_from(action.count()).unwrap_or(u32::MAX);
    let words = i18n::translate(language, said(action), &[("count", count.into())]);
    let galley = widgets::text(ui, &words, 12.5, 400.0, FontFamily::Proportional, p.text_1);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &words));
    widgets::address(ui, &response, "undo-text");
    ui.painter().galley(rect.min, galley, p.text_1);
}

/// One key per kind: « 3 notes deleted » and « 3 notes moved » are not the same sentence.
fn said(action: &Reversible) -> Key {
    match action {
        Reversible::Deletion { .. } => keys::undo::DELETED,
        Reversible::Move(_) => keys::undo::MOVED,
        Reversible::Tag(_) => keys::undo::TAGGED,
        Reversible::Untag(_) => keys::undo::UNTAGGED,
        Reversible::Pin { pinned: true, .. } => keys::undo::PINNED,
        Reversible::Pin { pinned: false, .. } => keys::undo::UNPINNED,
        Reversible::Priority(_) => keys::undo::PRIORITISED,
        Reversible::File(_) => keys::undo::FILED,
    }
}

/// The key is on the control, not in the sentence: the card that armed the note named Escape,
/// and the bar names it too.
fn restore(ui: &mut Ui, language: i18n::Language, actions: &mut Actions) {
    let p = palette(ui.ctx());
    let label = i18n::translate(language, keys::undo::RESTORE, &[]);
    let cap = i18n::translate(language, keys::undo::RESTORE_KEY, &[]);
    let words = widgets::text(
        ui,
        &label,
        12.0,
        400.0,
        FontFamily::Proportional,
        p.amber_text,
    );
    let key = widgets::text(ui, &cap, 10.0, 400.0, FontFamily::Monospace, p.text_2);
    let cap_size = key.size() + vec2(10.0, 2.0);
    let wanted = vec2(words.size().x + 6.0 + cap_size.x, words.size().y) + vec2(20.0, 8.0);
    let corner = CornerRadius::same(6);
    let response = widgets::interact(
        ui,
        wanted,
        Sense::click(),
        "undo-restore",
        || WidgetInfo::labeled(Role::Button, true, &label),
        corner,
    );
    let rect = response.rect;
    let (fill, edge) = if response.hovered() {
        (p.bg_3, p.amber_edge)
    } else {
        (p.bg_2, p.line_soft)
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        corner,
        fill,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    let words_at = egui::pos2(rect.left() + 10.0, rect.center().y - words.size().y / 2.0);
    let cap_rect = egui::Rect::from_min_size(
        egui::pos2(
            words_at.x + words.size().x + 6.0,
            rect.center().y - cap_size.y / 2.0,
        ),
        cap_size,
    );
    painter.galley(words_at, words, p.amber_text);
    painter.rect_stroke(
        cap_rect,
        CornerRadius::same(4),
        Stroke::new(1.0, p.line_soft),
        StrokeKind::Inside,
    );
    painter.galley(cap_rect.min + vec2(5.0, 1.0), key, p.text_2);
    if response.clicked() {
        actions.push(Action::Notes(crate::notes::Action::Undo));
    }
}

#[cfg(test)]
mod tests {
    use egui::{Key, Modifiers};
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};

    use crate::notes::bench::{Bench, area, cards, click_id, settle};
    use crate::testing;

    fn press(harness: &mut Harness<'_, Bench>, modifiers: Modifiers, key: Key) {
        harness.key_press_modifiers(modifiers, key);
        settle(harness);
    }

    /// The focused card, sent to the trash in two presses.
    fn trashed() -> Harness<'static, Bench> {
        let (backend, profile) = testing::unlocked();
        testing::note(&backend, "going");
        let mut harness = area(backend, profile);
        press(&mut harness, Modifiers::NONE, Key::ArrowDown);
        press(&mut harness, Modifiers::NONE, Key::Delete);
        press(&mut harness, Modifiers::NONE, Key::Delete);
        assert_eq!(cards(&harness), 0);
        harness
    }

    fn offered(harness: &Harness<'_, Bench>) -> bool {
        harness
            .query_by(|node| node.author_id() == Some("undo-bar"))
            .is_some()
    }

    #[test]
    fn a_deletion_is_offered_back_and_taken_back() {
        let mut harness = trashed();
        let text = harness.get_by(|node| node.author_id() == Some("undo-text"));
        let node = text.accesskit_node();
        let words = node.label().or_else(|| node.value()).unwrap_or_default();
        assert_eq!(words, "1 note moved to the trash");

        click_id(&mut harness, "undo-restore");

        assert_eq!(cards(&harness), 1);
        assert!(!offered(&harness));
    }

    #[test]
    fn escape_takes_it_back_while_the_bar_offers() {
        let mut harness = trashed();
        press(&mut harness, Modifiers::NONE, Key::Escape);

        assert_eq!(cards(&harness), 1);
    }

    #[test]
    fn a_dismissed_offer_leaves_nothing_for_ctrl_z() {
        let mut harness = trashed();
        click_id(&mut harness, "undo-dismiss");
        assert!(!offered(&harness));
        press(&mut harness, Modifiers::COMMAND, Key::Z);

        assert_eq!(cards(&harness), 0, "dismissed is given up for good");
    }

    #[test]
    fn ctrl_z_takes_it_back() {
        let mut harness = trashed();
        press(&mut harness, Modifiers::COMMAND, Key::Z);

        assert_eq!(cards(&harness), 1);
    }

    /// Typed into, the field keeps its own history: `Ctrl+Z` is never both.
    #[test]
    fn ctrl_z_in_a_field_is_the_field_s() {
        let mut harness = trashed();
        click_id(&mut harness, "search-input");
        harness
            .get_by(|node| node.author_id() == Some("search-input"))
            .type_text("going");
        settle(&mut harness);
        press(&mut harness, Modifiers::COMMAND, Key::Z);

        assert!(harness.state().state.undo.banner(f64::MAX).is_none());
        assert_eq!(cards(&harness), 0);
    }

    fn said(harness: &Harness<'_, Bench>) -> String {
        let node = harness.get_by(|node| node.author_id() == Some("undo-text"));
        let node = node.accesskit_node();
        node.label().or_else(|| node.value()).unwrap_or_default()
    }

    #[test]
    fn a_batch_says_what_it_did_and_is_put_back_whole() {
        let (backend, profile) = testing::unlocked();
        testing::note(&backend, "one");
        testing::note(&backend, "two");
        let mut harness = area(backend, profile);
        for key in [Key::ArrowDown, Key::X, Key::ArrowRight, Key::X] {
            press(&mut harness, Modifiers::NONE, key);
        }
        click_id(&mut harness, "selection-pin");
        assert_eq!(said(&harness), "2 notes pinned");

        click_id(&mut harness, "undo-restore");
        let view = harness.state().state.notes_query.view().unwrap();
        assert!(view.cards().all(|card| !card.pinned));
    }
}
