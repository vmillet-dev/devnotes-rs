//! What the interface's tests share: a harness that draws with the application's fonts.

use egui_kittest::HarnessBuilder;

use crate::theme::fonts;

pub(crate) fn harness<State>() -> HarnessBuilder<State> {
    HarnessBuilder::default().with_context_setup(|ctx| ctx.set_fonts(fonts::definitions()))
}

/// One frame of `ctx` at `time`, outside a harness.
pub(crate) fn frame(ctx: &egui::Context, time: f64, draw: impl FnMut(&mut egui::Ui)) {
    let input = egui::RawInput {
        time: Some(time),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, draw);
    // Nothing renders here: the textures are not uploaded anywhere.
    output.textures_delta.clear();
}
