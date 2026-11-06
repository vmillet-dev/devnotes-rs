//! What drawing asks for, applied at the end of the frame (plan § 3.4, D13): drawing reads a
//! screen's state and pushes actions; the state changes only in `AppState::apply`. Keys, menus,
//! the palette, the tray and a second launch all speak this one language.

use crate::state::Area;
use crate::theme::Appearance;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    App(AppAction),
    Gate(crate::gate::Action),
    Notes(crate::notes::Action),
    Http(crate::http::Action),
    Tools(crate::tools::Action),
    Palette(crate::palette::Asked),
    Global(GlobalAction),
}

/// The frame's: the areas, the look, the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    ShowArea(Area),
    SetAppearance(Appearance),
    DismissError,
    DismissStatus,
    /// The library rail's width, once its edge rests.
    SetRailWidth(u16),
    /// The history list's width, once its edge rests.
    SetHistoryWidth(u16),
    /// The library rail shown or put away: the switchers take its place.
    ToggleRail,
    /// `Ctrl+Shift+T`: the tools' search, from any area.
    SearchTools,
}

/// Asked from outside the window: a global shortcut, the tray, a second launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalAction {
    Capture,
    NewNote,
    /// Asked from another application: a note copied from it puts the window away again.
    Palette,
    /// Asked with the window already in front: it only opens.
    PaletteHere,
    Quit,
}

impl GlobalAction {
    pub const ALL: [Self; 5] = [
        Self::Capture,
        Self::NewNote,
        Self::Palette,
        Self::PaletteHere,
        Self::Quit,
    ];

    /// The name a second launch of the `e2e` flavour hands over (`--action new-note`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Capture => "capture",
            Self::NewNote => "new-note",
            Self::Palette => "palette",
            Self::PaletteHere => "palette-here",
            Self::Quit => "quit",
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.name() == name)
    }
}

/// The frame's queue of actions, in the order they were pushed.
#[derive(Debug, Default)]
pub struct Actions(Vec<Action>);

impl Actions {
    pub fn push(&mut self, action: Action) {
        self.0.push(action);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = Action> + '_ {
        self.0.drain(..)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_global_action_is_found_by_its_name() {
        for action in GlobalAction::ALL {
            assert_eq!(GlobalAction::named(action.name()), Some(action));
        }
        assert_eq!(GlobalAction::named("format-disk"), None);
    }
}
