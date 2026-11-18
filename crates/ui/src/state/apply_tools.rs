//! What the tools area asks.

use super::AppState;
use crate::backend::Backend;

impl AppState {
    /// What a tool holds of a secret goes with it.
    pub(super) fn leave_tool(&mut self) {
        if let Some(entry) = self.tools.open_tool() {
            self.tools_view.left(entry);
        }
    }

    pub(super) fn ask_tool_search(&mut self) {
        self.leave_tool();
        self.tools.home(None);
        self.tools_view.want_search();
    }

    pub(super) fn apply_tools(&mut self, action: crate::tools::Action, backend: &Backend) {
        use crate::tools::Action as Tools;
        match action {
            Tools::Home(filter) => {
                self.leave_tool();
                self.tools.home(filter);
            }
            Tools::Open(id) => {
                if let Some(entry) = crate::tools::catalogue::find(id) {
                    if self.tools.open_tool().is_some_and(|open| open.id != id) {
                        self.leave_tool();
                    }
                    self.tools.open(entry, self.clock.now());
                    self.save_settings(backend);
                }
            }
            Tools::ToggleFavourite(id) => {
                self.tools.toggle_favourite(id);
                self.save_settings(backend);
            }
            Tools::SearchAsked => self.ask_tool_search(),
            Tools::SetZones(zones) => {
                self.tools.set_zones(zones);
                self.save_settings(backend);
            }
            Tools::SetCsvSeparator(separator) => {
                self.tools.set_csv_separator(separator);
                self.save_settings(backend);
            }
            Tools::SampleLoaded => {
                let said = i18n::Message::new(i18n::keys::tools::SAMPLE_LOADED);
                self.status.report(said);
            }
        }
    }
}
