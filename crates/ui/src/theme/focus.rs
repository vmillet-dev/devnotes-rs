//! The focus ring: 2 px of `amber_edge`, 2 px outside the control, and only after the keyboard
//! moved — the reference's `:focus-visible`. A pointer must not leave a ring behind it.

use egui::{Context, CornerRadius, Event, Id, Key, Response, Stroke, StrokeKind, Ui};

use super::palette;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Modality {
    #[default]
    Pointer,
    Keyboard,
}

/// Once a frame, before drawing: the last input that moves around decides. Typing does not:
/// a field focused by a click stays without a ring.
pub fn track(ctx: &Context) {
    let last = ctx.input(|input| {
        input.events.iter().rev().find_map(|event| match event {
            Event::Key {
                key, pressed: true, ..
            } if navigates(*key) => Some(Modality::Keyboard),
            Event::PointerButton { pressed: true, .. } => Some(Modality::Pointer),
            _ => None,
        })
    });
    if let Some(modality) = last {
        ctx.data_mut(|data| data.insert_temp(modality_id(), modality));
    }
}

fn navigates(key: Key) -> bool {
    matches!(
        key,
        Key::Tab
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::ArrowLeft
            | Key::ArrowRight
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::Enter
            | Key::Space
            | Key::Escape
    )
}

fn modality_id() -> Id {
    Id::unique("theme.focus.modality")
}

/// Whether `response` wears the ring this frame.
pub fn visible(ctx: &Context, response: &Response) -> bool {
    response.has_focus()
        && ctx.data(|data| data.get_temp::<Modality>(modality_id()).unwrap_or_default())
            == Modality::Keyboard
}

/// The ring around `response`, following its corners two pixels out.
pub fn ring(ui: &Ui, response: &Response, corner_radius: CornerRadius) {
    if !visible(ui.ctx(), response) {
        return;
    }
    let offset = 2.0;
    ui.painter().rect_stroke(
        response.rect.expand(offset),
        corner_radius + 2,
        Stroke::new(2.0, palette(ui.ctx()).amber_edge),
        StrokeKind::Outside,
    );
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    use super::*;

    #[derive(Default)]
    struct Seen {
        ringed: bool,
    }

    fn harness() -> Harness<'static, Seen> {
        crate::testing::harness().build_ui_state(
            |ui, seen: &mut Seen| {
                track(ui.ctx());
                let response = ui.button("Save");
                ring(ui, &response, CornerRadius::same(6));
                seen.ringed = visible(ui.ctx(), &response);
            },
            Seen::default(),
        )
    }

    #[test]
    fn the_keyboard_brings_the_ring() {
        let mut harness = harness();

        harness.key_press(Key::Tab);
        harness.run();

        assert!(harness.state().ringed);
    }

    #[test]
    fn a_click_leaves_no_ring_behind() {
        let mut harness = harness();
        harness.key_press(Key::Tab);
        harness.run();

        harness.get_by_label("Save").click();
        harness.run();

        assert!(!harness.state().ringed);
    }
}
