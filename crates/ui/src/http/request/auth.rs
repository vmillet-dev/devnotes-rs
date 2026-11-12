//! A request's auth. The secret half is masked until asked for, and an inherited one says what it
//! is and where it was set.

use egui::{Ui, Vec2};
use engine::http::model::{KeyPlace, RequestAuth};
use engine::http::settings::Inherited;
use i18n::keys;

use crate::api;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::widgets::{self, ChoiceMenu, IconButton, TextField};

const LABEL: f32 = 96.0;
const WIDTH: f32 = 560.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Inherit,
    None,
    Basic,
    Bearer,
    ApiKey,
}

impl Kind {
    const ALL: [Self; 5] = [
        Self::Inherit,
        Self::None,
        Self::Basic,
        Self::Bearer,
        Self::ApiKey,
    ];

    fn of(auth: &RequestAuth) -> Self {
        match auth {
            RequestAuth::Inherit => Self::Inherit,
            RequestAuth::None => Self::None,
            RequestAuth::Basic { .. } => Self::Basic,
            RequestAuth::Bearer { .. } => Self::Bearer,
            RequestAuth::ApiKey { .. } => Self::ApiKey,
        }
    }

    fn name(self) -> i18n::Key {
        use keys::http::auth::kinds;
        match self {
            Self::Inherit => kinds::INHERIT,
            Self::None => kinds::NONE,
            Self::Basic => kinds::BASIC,
            Self::Bearer => kinds::BEARER,
            Self::ApiKey => kinds::API_KEY,
        }
    }

    fn fresh(self) -> RequestAuth {
        match self {
            Self::Inherit => RequestAuth::Inherit,
            Self::None => RequestAuth::None,
            Self::Basic => RequestAuth::Basic {
                username: String::new(),
                password: String::new(),
            },
            Self::Bearer => RequestAuth::Bearer {
                token: String::new(),
            },
            Self::ApiKey => RequestAuth::ApiKey {
                name: String::new(),
                value: String::new(),
                place: KeyPlace::Header,
            },
        }
    }
}

/// What the editor draws.
pub(crate) struct AuthField<'a> {
    pub(crate) auth: &'a RequestAuth,
    /// What « Inherited » takes; `None` when there is nowhere yet to inherit from.
    pub(crate) above: Option<&'a Inherited>,
    /// A collection has nothing above it to inherit from.
    pub(crate) allow_inherit: bool,
}

/// The auth as edited this frame.
pub(crate) fn show(
    ui: &mut Ui,
    editor: &AuthField<'_>,
    revealed: &mut bool,
    language: i18n::Language,
) -> Option<RequestAuth> {
    ui.set_max_width(WIDTH.min(ui.available_width()));
    let auth = shown_auth(editor);
    let mut changed = kind_line(ui, editor, language);
    let tr = |key| i18n::translate(language, key, &[]);
    match auth.clone() {
        RequestAuth::Inherit => inherited(ui, editor.above, language),
        RequestAuth::None => hint(ui, &tr(keys::http::auth::NONE_HINT)),
        RequestAuth::Basic {
            mut username,
            mut password,
        } => {
            let label = tr(keys::http::auth::USERNAME);
            let typed = field(ui, &label, |ui| {
                TextField::new(&mut username, &label, "http-auth-username")
                    .show(ui)
                    .changed()
            });
            let label = tr(keys::http::auth::secrets::PASSWORD);
            let secret = secret(ui, (&label, &mut password), revealed, language);
            if typed || secret {
                changed = Some(RequestAuth::Basic { username, password });
            }
        }
        RequestAuth::Bearer { mut token } => {
            let label = tr(keys::http::auth::secrets::TOKEN);
            if secret(ui, (&label, &mut token), revealed, language) {
                changed = Some(RequestAuth::Bearer { token });
            }
        }
        RequestAuth::ApiKey {
            mut name,
            mut value,
            mut place,
        } => {
            let label = tr(keys::http::auth::KEY_NAME);
            let mut edited = field(ui, &label, |ui| {
                TextField::new(&mut name, &label, "http-auth-key-name")
                    .hint("X-Api-Key")
                    .show(ui)
                    .changed()
            });
            let label = tr(keys::http::auth::secrets::VALUE);
            edited |= secret(ui, (&label, &mut value), revealed, language);
            edited |= place_line(ui, &mut place, language);
            if edited {
                changed = Some(RequestAuth::ApiKey { name, value, place });
            }
        }
    }
    changed
}

/// `Inherit` on a collection is no auth at all: nothing is above it.
fn shown_auth<'a>(editor: &AuthField<'a>) -> &'a RequestAuth {
    if !editor.allow_inherit && *editor.auth == RequestAuth::Inherit {
        &RequestAuth::None
    } else {
        editor.auth
    }
}

/// A label and what it names, on one line.
fn field<T>(ui: &mut Ui, label: &str, content: impl FnOnce(&mut Ui) -> T) -> T {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let words = egui::RichText::new(label).size(12.5).color(p.text_1);
        let size = Vec2::new(LABEL, ui.spacing().interact_size.y);
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        ui.allocate_ui_with_layout(size, layout, |ui| {
            ui.set_width(LABEL);
            ui.add(egui::Label::new(words));
        });
        content(ui)
    })
    .inner
}

fn kind_line(ui: &mut Ui, editor: &AuthField<'_>, language: i18n::Language) -> Option<RequestAuth> {
    let names = Kind::ALL.map(|kind| i18n::translate(language, kind.name(), &[]));
    let options: Vec<(Kind, &str)> = Kind::ALL
        .iter()
        .zip(&names)
        .filter(|(kind, _)| editor.allow_inherit || **kind != Kind::Inherit)
        .map(|(kind, name)| (*kind, name.as_str()))
        .collect();
    let label = i18n::translate(language, keys::http::auth::KIND, &[]);
    let current = Kind::of(shown_auth(editor));
    let mut kind = current;
    field(ui, &label, |ui| {
        ChoiceMenu::new(&label, "http-auth-kind", &options).show(ui, &mut kind);
    });
    (kind != current).then(|| kind.fresh())
}

fn place_line(ui: &mut Ui, place: &mut KeyPlace, language: i18n::Language) -> bool {
    let tr = |key| i18n::translate(language, key, &[]);
    let (header, query) = (
        tr(keys::http::auth::places::HEADER),
        tr(keys::http::auth::places::QUERY),
    );
    let options = [
        (KeyPlace::Header, header.as_str()),
        (KeyPlace::Query, query.as_str()),
    ];
    let label = tr(keys::http::auth::PLACE);
    field(ui, &label, |ui| {
        ChoiceMenu::new(&label, "http-auth-place", &options)
            .show(ui, place)
            .changed()
    })
}

/// The secret, masked until asked for; `true` when it changed.
fn secret(
    ui: &mut Ui,
    (label, value): (&str, &mut String),
    revealed: &mut bool,
    language: i18n::Language,
) -> bool {
    field(ui, label, |ui| {
        let eye = i18n::translate(
            language,
            if *revealed {
                keys::http::auth::HIDE
            } else {
                keys::http::auth::SHOW
            },
            &[],
        );
        let icon = if *revealed { Icon::EyeOff } else { Icon::Eye };
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let pressed = IconButton::new(icon, &eye, "http-auth-reveal")
                .pressed(*revealed)
                .show(ui);
            if pressed.clicked() {
                *revealed = !*revealed;
            }
            TextField::new(value, label, "http-auth-secret")
                .password(!*revealed)
                .show(ui)
                .changed()
        })
        .inner
    })
}

/// « Bearer {{accessToken}} », the secret masked unless it is a variable.
fn summary(auth: &RequestAuth) -> String {
    let secret = |value: &str| {
        let whole = api::http::spans(value)
            .first()
            .is_some_and(|(range, _)| *range == (0..value.len()));
        if whole {
            value.to_owned()
        } else {
            "••••••".to_owned()
        }
    };
    match auth {
        RequestAuth::Basic { username, .. } => format!("Basic {username}"),
        RequestAuth::Bearer { token } => format!("Bearer {}", secret(token)),
        RequestAuth::ApiKey { name, value, .. } => format!("{name}: {}", secret(value)),
        RequestAuth::Inherit | RequestAuth::None => String::new(),
    }
}

/// What « Inherited » takes, once the request has a place to inherit from.
fn inherited(ui: &mut Ui, above: Option<&Inherited>, language: i18n::Language) {
    let words = above.map_or_else(
        || i18n::translate(language, keys::http::auth::PLACE_FIRST, &[]),
        |above| inherited_words(above, language),
    );
    let p = palette(ui.ctx());
    let shown = egui::RichText::new(words).size(12.5).color(p.text_1);
    let shown = ui.add(egui::Label::new(shown).wrap());
    widgets::address(ui, &shown, "http-auth-inherited");
}

fn inherited_words(above: &Inherited, language: i18n::Language) -> String {
    let Some(from) = &above.auth_from else {
        return i18n::translate(language, keys::http::auth::NOTHING_ABOVE, &[]);
    };
    let key = match from.kind {
        engine::http::model::HttpItemKind::Folder => keys::http::auth::from::FOLDER,
        _ => keys::http::auth::from::COLLECTION,
    };
    let origin = i18n::translate(language, key, &[("name", from.name.as_str().into())]);
    format!("{} {origin}", summary(&above.auth))
}

fn hint(ui: &mut Ui, words: &str) {
    let p = palette(ui.ctx());
    let words = egui::RichText::new(words).size(12.0).color(p.text_2);
    ui.add(egui::Label::new(words).wrap());
}
