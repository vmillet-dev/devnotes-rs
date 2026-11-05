//! The `e2e` flavour paints a frame only when it is captured: the driver reads the AccessKit
//! tree, which the pass builds whether or not it is painted, and a painted frame costs up to
//! 300 ms on WARP.

use egui::{Context, FullOutput, ViewportCommand, ViewportId};

#[derive(Default)]
pub(crate) struct Unpainted {
    captured: bool,
}

impl egui::Plugin for Unpainted {
    fn debug_name(&self) -> &'static str {
        "unpainted"
    }

    // eframe takes a screenshot while painting the frame after the one that asked for it.
    fn output_hook(&mut self, _ctx: &Context, output: &mut FullOutput) {
        let captured = std::mem::replace(&mut self.captured, asks_for_capture(output));
        if !captured {
            output.shapes.clear();
        }
    }
}

fn asks_for_capture(output: &FullOutput) -> bool {
    output
        .viewport_output
        .get(&ViewportId::ROOT)
        .is_some_and(|root| {
            root.commands
                .iter()
                .any(|command| matches!(command, ViewportCommand::Screenshot(_)))
        })
}
