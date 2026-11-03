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

/// A real core over an empty profile, its events kept. The directory goes with the guard.
pub(crate) fn backend() -> (crate::backend::Backend, tempfile::TempDir) {
    let profile = tempfile::tempdir().unwrap();
    let backend = backend_on(profile.path());
    (backend, profile)
}

/// A real core over `profile`, as a later launch finds it.
pub(crate) fn backend_on(profile: &std::path::Path) -> crate::backend::Backend {
    let paths = engine::Paths {
        profile: profile.to_path_buf(),
        resources: profile.to_path_buf(),
    };
    crate::backend::Backend::start(
        paths,
        std::sync::Arc::new(engine::RecordingSink::default()),
        crate::inputs::Waker::default(),
    )
    .unwrap()
}

/// A test value, never anyone's phrase.
pub(crate) const PASSPHRASE: &str = "correct horse battery staple";

/// A real library, created and open, through the same calls the gate makes.
pub(crate) fn unlocked() -> (crate::backend::Backend, tempfile::TempDir) {
    let (backend, profile) = backend();
    let mut created =
        crate::api::vault::create(&backend, zeroize::Zeroizing::new(PASSPHRASE.to_string()));
    backend.settle();
    created.poll().unwrap().unwrap();
    (backend, profile)
}
