//! The application under test: a button that adds cards, a search field that saves what is typed
//! on Enter into the profile, and the cards. Every control carries its `data-testid` as AccessKit
//! `author_id` (plan § 8.2). `app --profile <dir>`, inspection on with the `e2e` feature.

use eframe::egui;

struct App {
    profile: std::path::PathBuf,
    cards: u32,
    search: String,
}

fn tag(ui: &egui::Ui, response: &egui::Response, test_id: &str) {
    ui.ctx().accesskit_node_builder(response.id, |node| node.set_author_id(test_id.to_owned()));
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            let add = ui.button("Nouvelle note");
            tag(ui, &add, "add-note");
            if add.clicked() {
                self.cards += 1;
            }
            let search = ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Rechercher"));
            tag(ui, &search, "search");
            if search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                std::fs::write(self.profile.join("saved.txt"), &self.search).expect("profile writable");
            }
            for n in 0..self.cards {
                let card = ui.label(format!("Carte {n}"));
                tag(ui, &card, "note-card");
            }
        });
    }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let profile: std::path::PathBuf = args.iter().position(|a| a == "--profile").and_then(|at| args.get(at + 1)).expect("--profile").into();
    let search = std::fs::read_to_string(profile.join("saved.txt")).unwrap_or_default();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]).with_title("S10 app"),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native("s10-app", options, Box::new(move |_| Ok(Box::new(App { profile, cards: 0, search }))))
}
