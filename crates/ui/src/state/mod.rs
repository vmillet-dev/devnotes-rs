//! What the interface remembers between frames, apart from the drawing (plan § 3.4).

mod query;

pub use query::QueryCell;

use crate::action::{Action, AppAction, GlobalAction};
use crate::theme::{self, Appearance, Density};

/// Bumped after every write the interface adopts: the queries that read notes carry it among
/// their parameters, so the list and the board ask again together. Writes are not optimistic:
/// persist, adopt the answer, bump.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Revision(u64);

impl Revision {
    pub fn bump(&mut self) {
        self.0 += 1;
    }
}

/// Every cache of a library's content — laid-out text, textures, memos, diagrams — and only
/// they: one field, emptied when another library opens (D17). Not wiped byte by byte; the
/// threat model says so.
#[derive(Default)]
pub struct LibraryCaches {}

/// The three areas, in the switch's order, which is also the order of their keys.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Area {
    #[default]
    Notes,
    Tools,
    Http,
}

/// The root of the interface's state. Each screen joins it as a pair — its state, which other
/// screens and the core see, and its view, which only its drawing sees (D13) — as it is ported.
pub struct AppState {
    /// The area on screen. The others are not drawn and cost nothing; their state stays here.
    pub area: Area,
    pub revision: Revision,
    pub caches: LibraryCaches,
    /// The application's look, which outlives a library switch.
    pub appearance: Appearance,
    pub density: Density,
    pub reduced_motion: bool,
}

impl AppState {
    pub fn new(reduced_motion: bool) -> Self {
        Self {
            area: Area::default(),
            revision: Revision::default(),
            caches: LibraryCaches::default(),
            appearance: Appearance::default(),
            density: Density::default(),
            reduced_motion,
        }
    }

    /// What reloading the page did in the reference: another library is open, so nothing of
    /// the previous one may stay — the caches go, and so does every widget's state egui keeps
    /// (a text field's undo history holds what was typed). The application's settings stay.
    pub fn reset_library_scope(&mut self, ctx: &egui::Context) {
        self.caches = LibraryCaches::default();
        self.revision.bump();
        ctx.memory_mut(|memory| memory.data.clear());
        self.apply_look(ctx);
    }

    /// At the end of the frame, in the order the actions were pushed.
    pub fn apply(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::App(AppAction::ShowArea(area)) => self.area = area,
            Action::App(AppAction::SetAppearance(appearance)) => {
                self.appearance = appearance;
                self.apply_look(ctx);
            }
            Action::Global(GlobalAction::Quit) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            // The capture, the new note and the palette arrive with the notes area.
            Action::Global(
                GlobalAction::Capture
                | GlobalAction::NewNote
                | GlobalAction::Palette
                | GlobalAction::PaletteHere,
            ) => {}
        }
    }

    /// The look into egui, which keeps it in the memory a library switch clears.
    pub fn apply_look(&self, ctx: &egui::Context) {
        theme::motion::set_reduced(ctx, self.reduced_motion);
        theme::apply(ctx, self.appearance, self.density);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_library_switch_forgets_the_widgets_and_keeps_the_look() {
        let ctx = egui::Context::default();
        let mut state = AppState::new(true);
        state.density = Density::Compact;
        state.apply_look(&ctx);
        let typed = egui::Id::unique("typed");
        ctx.data_mut(|data| data.insert_temp(typed, "psql -h prod".to_string()));
        let before = state.revision;

        state.reset_library_scope(&ctx);

        assert_eq!(ctx.data(|data| data.get_temp::<String>(typed)), None);
        assert_eq!(theme::spacing(&ctx), &theme::tokens::COMPACT);
        assert!(theme::motion::reduced(&ctx));
        assert_ne!(state.revision, before);
    }

    #[test]
    fn an_action_changes_the_state_at_the_end_of_the_frame() {
        let ctx = egui::Context::default();
        let mut state = AppState::new(false);

        state.apply(Action::App(AppAction::ShowArea(Area::Http)), &ctx);
        state.apply(
            Action::App(AppAction::SetAppearance(Appearance::Light)),
            &ctx,
        );

        assert_eq!(state.area, Area::Http);
        assert_eq!(state.appearance, Appearance::Light);
    }

    #[test]
    fn a_bumped_revision_is_another_parameter() {
        let mut revision = Revision::default();
        let before = revision;

        revision.bump();

        assert_ne!(revision, before);
    }
}
