//! The modal frame and the stack that orders the open ones (the reference's `dialog.model.ts`):
//! one rung per layer, the higher drawn over the lower and served first by Escape. A modal
//! blocks what is under it, keeps Tab inside, and hands the focus back to what opened it.

use crate::theme::tokens::{font, gap, radius};
use egui::{Context, CornerRadius, Id, Margin, ModalResponse, Stroke};

use crate::theme::{palette, spacing};

/// The rungs, lowest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    Editor,
    Titlebar,
    App,
    Settings,
    Update,
    Palette,
    Fields,
    Zoom,
    Presentation,
    Passphrase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Opened {
    layer: Layer,
    id: Id,
    opener: Option<Id>,
}

/// Which modals are open, in which order. State, not view: Escape and the keys read it.
#[derive(Debug, Default)]
pub struct DialogStack {
    open: Vec<Opened>,
    /// The opener of the modal just closed, given the focus once nothing blocks it any more.
    refocus: Option<Id>,
}

impl DialogStack {
    /// `opener` gets the focus back when it closes.
    pub fn open(&mut self, layer: Layer, id: Id, opener: Option<Id>) {
        if self.is_open(id) {
            return;
        }
        self.open.push(Opened { layer, id, opener });
        // Stable: on one rung, the one opened last is on top.
        self.open.sort_by_key(|opened| opened.layer);
    }

    pub fn close(&mut self, ctx: &Context, id: Id) {
        let Some(at) = self.open.iter().position(|opened| opened.id == id) else {
            return;
        };
        let closed = self.open.remove(at);
        self.refocus = closed.opener;
        ctx.request_repaint();
    }

    /// Once a frame, before drawing: hands the focus back to what opened the modal last closed.
    pub fn begin_frame(&mut self, ctx: &Context) {
        // The closed modal still blocks the frame after its last: wait until none does.
        if self.open.is_empty() && ctx.memory(egui::Memory::top_modal_layer).is_some() {
            return;
        }
        if let Some(opener) = self.refocus.take() {
            ctx.memory_mut(|memory| memory.request_focus(opener));
        }
    }

    /// Open as long as `open` says, a modal whose state lives elsewhere: what had the focus
    /// when it opened gets it back when it closes.
    pub fn follow(&mut self, ctx: &Context, layer: Layer, id: Id, open: bool) {
        if open {
            let opener = ctx.memory(egui::Memory::focused);
            self.open(layer, id, opener);
        } else {
            self.close(ctx, id);
        }
    }

    pub fn is_open(&self, id: Id) -> bool {
        self.open.iter().any(|opened| opened.id == id)
    }

    /// The modal Escape goes to first.
    pub fn top(&self) -> Option<Id> {
        self.open.last().map(|opened| opened.id)
    }

    /// The open ones, lowest first: the order to draw them in.
    pub fn in_order(&self) -> impl Iterator<Item = Id> + '_ {
        self.open.iter().map(|opened| opened.id)
    }
}

/// What a modal drew, and whether it asks to close: Escape, a click outside, or its cross.
pub struct Shown<R> {
    pub inner: R,
    crossed: bool,
    modal: ModalResponse<()>,
}

impl<R> Shown<R> {
    /// ⚠️ Consumes Escape, as egui's own does: asked only by the screen that closes it.
    pub fn should_close(&self) -> bool {
        self.crossed || self.modal.should_close()
    }
}

/// The cross every modal closes by, top right, drawn over its content: `(label, author_id)`.
pub type Cross<'a> = (&'a str, &'a str);

/// `margin`: the frame's, around the content. The cross sits in that margin's corner, so it covers
/// none of what the content puts top right.
pub(crate) fn cross(ui: &mut egui::Ui, (label, author_id): Cross<'_>, margin: f32) -> bool {
    let content = ui.max_rect();
    let size = egui::Vec2::splat(super::MIN_TARGET);
    let corner = margin - gap::TIGHT;
    let at = egui::Rect::from_min_size(
        egui::pos2(content.right() + corner - size.x, content.top() - corner),
        size,
    );
    let mut corner = ui.new_child(egui::UiBuilder::new().max_rect(at));
    super::IconButton::new(crate::theme::icons::Icon::X, label, author_id)
        .show(&mut corner)
        .clicked()
}

pub(crate) fn shown<R>(response: ModalResponse<(R, bool)>) -> Shown<R> {
    let ModalResponse {
        response,
        backdrop_response,
        inner: (inner, crossed),
        is_top_modal,
        any_popup_open,
    } = response;
    Shown {
        inner,
        crossed,
        modal: ModalResponse {
            response,
            backdrop_response,
            inner: (),
            is_top_modal,
            any_popup_open,
        },
    }
}

/// A fitted modal: `bg-1`, 14 px corners, a `line` border, the overlay's shadow, a cross top
/// right. Its scrim is solid, where the reference blurred. `None` when `id` is not open.
pub fn dialog<R>(
    ctx: &Context,
    stack: &DialogStack,
    (id, close): (Id, Cross<'_>),
    width: f32,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<Shown<R>> {
    if !stack.is_open(id) {
        return None;
    }
    let p = palette(ctx);
    let gaps = spacing(ctx);
    #[allow(clippy::cast_possible_truncation)]
    let padding = Margin::symmetric(gaps.canvas as i8, (gaps.canvas - 2.0) as i8);
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(radius::DIALOG))
        .shadow(p.shadow_overlay)
        .inner_margin(padding);
    let shown = egui::Modal::new(id)
        .backdrop_color(p.backdrop.gamma_multiply(0.7))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(width.min(ctx.content_rect().width() * 0.9));
            let inner = content(ui);
            (inner, cross(ui, close, gaps.canvas))
        });
    Some(self::shown(shown))
}

/// A framed modal: the same surface, its content edge to edge, for a panel with parts of its own;
/// the same cross.
pub fn framed_dialog<R>(
    ctx: &Context,
    stack: &DialogStack,
    (id, close): (Id, Cross<'_>),
    size: egui::Vec2,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<Shown<R>> {
    if !stack.is_open(id) {
        return None;
    }
    let p = palette(ctx);
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(radius::DIALOG))
        .shadow(p.shadow_overlay);
    let room = ctx.content_rect().size() * 0.9;
    let shown = egui::Modal::new(id)
        .backdrop_color(p.backdrop.gamma_multiply(0.7))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(size.x.min(room.x));
            ui.set_height(size.y.min(room.y));
            let inner = content(ui);
            // No margin: the cross stands inside the panel's own header.
            (inner, cross(ui, close, -gap::CONTROLS))
        });
    Some(self::shown(shown))
}

/// A Notes panel's modal (a folder's name, a snippet's fields, the tags, the trash): the
/// dialog's frame, padded evenly, over the dimmed backdrop. Its keys and its closing stay the
/// caller's.
pub fn modal_panel<R>(
    ctx: &Context,
    id: Id,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> ModalResponse<R> {
    let p = palette(ctx);
    #[expect(clippy::cast_possible_truncation, reason = "a few points")]
    let padding = spacing(ctx).canvas.round() as i8;
    let frame = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CornerRadius::same(radius::DIALOG))
        .shadow(p.shadow_overlay)
        .inner_margin(Margin::same(padding));
    egui::Modal::new(id)
        .backdrop_color(p.backdrop.gamma_multiply(0.7))
        .frame(frame)
        .show(ctx, content)
}

/// A dialog's title: its words at the title's size, read as its heading, found by `author_id`.
pub fn dialog_title(ui: &mut egui::Ui, words: &str, author_id: &str) -> egui::Response {
    let ink = palette(ui.ctx()).text_0;
    let galley = super::text(
        ui,
        words,
        font::TITLE,
        600.0,
        egui::FontFamily::Proportional,
        ink,
    );
    let (rect, heading) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
    ui.painter().galley(rect.min, galley, ink);
    heading.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Heading, true, words));
    super::address(ui, &heading, author_id);
    heading
}

#[cfg(test)]
mod tests {
    use egui::{Key, Modifiers};
    use egui_kittest::kittest::{NodeT, Queryable};

    use super::*;

    use crate::keys::{self, Binding, Read};
    use crate::widgets::Button;

    #[test]
    fn escape_closes_the_highest_rung_first_whatever_the_order_opened() {
        let ctx = Context::default();
        let mut stack = DialogStack::default();
        let (passphrase, settings) = (Id::unique("passphrase"), Id::unique("settings"));

        stack.open(Layer::Passphrase, passphrase, None);
        stack.open(Layer::Settings, settings, None);

        assert_eq!(stack.top(), Some(passphrase));
        stack.close(&ctx, passphrase);
        assert_eq!(stack.top(), Some(settings));
        assert_eq!(stack.in_order().collect::<Vec<_>>(), [settings]);
    }

    #[derive(Default)]
    struct Screen {
        stack: DialogStack,
        under_clicked: bool,
    }

    fn screen() -> egui_kittest::Harness<'static, Screen> {
        crate::testing::harness()
            .with_size(egui::vec2(800.0, 600.0))
            .build_ui_state(
                |ui, screen: &mut Screen| {
                    screen.stack.begin_frame(ui.ctx());
                    let opener = Button::new("Préférences", "open-settings").show(ui);
                    if opener.clicked() {
                        screen
                            .stack
                            .open(Layer::Settings, Id::unique("settings"), Some(opener.id));
                    }
                    if Button::new("Dessous", "under").show(ui).clicked() {
                        screen.under_clicked = true;
                    }
                    dialog(
                        ui.ctx(),
                        &screen.stack,
                        (Id::unique("settings"), ("Fermer", "settings-cross")),
                        480.0,
                        |ui| {
                            Button::new("Un", "first").show(ui);
                            Button::new("Deux", "second").show(ui);
                        },
                    );
                    let close = [Binding {
                        key: Key::Escape,
                        modifiers: Modifiers::NONE,
                        read: Read::Character,
                        while_typing: true,
                        action: crate::action::Action::Global(crate::action::GlobalAction::Quit),
                    }];
                    if screen.stack.top().is_some() && keys::dispatch(ui.ctx(), &[&close]).is_some()
                    {
                        screen.stack.close(ui.ctx(), Id::unique("settings"));
                    }
                },
                Screen::default(),
            )
    }

    fn focused(harness: &egui_kittest::Harness<'_, Screen>) -> Option<String> {
        harness
            .query_by(|node| node.is_focused() && node.author_id().is_some())
            .and_then(|node| node.accesskit_node().author_id().map(str::to_string))
    }

    #[test]
    fn a_modal_keeps_tab_inside_blocks_what_is_under_and_gives_the_focus_back() {
        let mut harness = screen();
        harness.run();
        harness
            .get_by(|node| node.author_id() == Some("open-settings"))
            .click();
        harness.run();

        for _ in 0..4 {
            harness.key_press(Key::Tab);
            harness.run();
            let now = focused(&harness);
            assert!(
                matches!(now.as_deref(), Some("first" | "second" | "settings-cross")),
                "{now:?}"
            );
        }
        harness
            .get_by(|node| node.author_id() == Some("under"))
            .click();
        harness.run();
        assert!(!harness.state().under_clicked);

        harness.key_press(Key::Escape);
        harness.run();
        assert!(harness.state().stack.top().is_none());
        assert_eq!(focused(&harness).as_deref(), Some("open-settings"));
    }
}
