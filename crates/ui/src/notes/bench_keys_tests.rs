//! The bench reads the keys as the window does: what a test presses there is what a person's
//! press would do.

use egui::{Key, Modifiers};

use crate::notes::bench::{area, settle};
use crate::state::Area;
use crate::testing;
use crate::widgets::Layer;

#[test]
fn under_a_stacked_modal_no_key_reaches_the_area() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);
    harness
        .state_mut()
        .state
        .dialogs
        .open(Layer::Settings, egui::Id::unique("a-dialog"), None);
    settle(&mut harness);

    harness.key_press_modifiers(Modifiers::COMMAND, Key::Num2);
    settle(&mut harness);

    assert_eq!(harness.state().state.settings.area, Area::Notes);
}
