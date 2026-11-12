//! The tray's menu in the language on screen: the system knows none of ours.

use engine::app_info::METADATA;
use i18n::{Key, Language, Message, keys};
use platform::tray::Labels;

pub fn labels(language: Language) -> Labels {
    let named = |key: Key| Message::new(key).with("app", METADATA.name).text(language);
    let plain = |key: Key| Message::new(key).text(language);
    Labels {
        tooltip: METADATA.name.to_owned(),
        open: named(keys::tray::OPEN),
        new_note: plain(keys::tray::NEW_NOTE),
        capture: plain(keys::tray::CAPTURE),
        palette: plain(keys::tray::PALETTE),
        quit: named(keys::tray::QUIT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_speaks_the_language_on_screen_and_names_the_application() {
        let french = labels(Language::Fr);
        let english = labels(Language::En);

        assert_ne!(french.new_note, english.new_note);
        assert!(english.open.contains(METADATA.name), "{}", english.open);
        assert!(english.quit.contains(METADATA.name), "{}", english.quit);
        assert_eq!(english.tooltip, METADATA.name);
    }
}
