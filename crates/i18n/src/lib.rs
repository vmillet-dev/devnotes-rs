//! The interface's strings: `fr.json` and `en.json`, in the format the Angular reference wrote
//! them (plan § 3.11). Code never holds a sentence: it holds a [`Key`] and its parameters, and
//! the text is chosen when it is drawn. ⚠️ Never interpolate a user's text into a template.

use std::collections::HashMap;
use std::sync::LazyLock;

pub mod format;
mod plural;
mod regions;
pub mod time;

/// One constant per key, checked at build time: `keys::vault::UNLOCK_TITLE`.
pub mod keys {
    include!(concat!(env!("OUT_DIR"), "/keys.rs"));
}

/// A translation key; only the generated constants and [`Key::find`] make one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(&'static str);

impl Key {
    pub fn path(self) -> &'static str {
        self.0
    }

    /// A key composed at run time, if it is one.
    pub fn find(path: &str) -> Option<Self> {
        FRENCH
            .get_key_value(path)
            .map(|(path, _)| Self(path.as_str()))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Language {
    Fr,
    #[default]
    En,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::Fr, Self::En];

    /// The first of the system's preferred languages whose primary tag is `fr` or `en`;
    /// English when none is.
    pub fn of_system() -> Self {
        Self::first_of(sys_locale::get_locales())
    }

    fn first_of(locales: impl IntoIterator<Item = String>) -> Self {
        locales
            .into_iter()
            .find_map(|locale| {
                let primary = locale.split(['-', '_']).next()?.to_ascii_lowercase();
                match primary.as_str() {
                    "fr" => Some(Self::Fr),
                    "en" => Some(Self::En),
                    _ => None,
                }
            })
            .unwrap_or_default()
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Fr => "fr",
            Self::En => "en",
        }
    }

    fn catalogue(self) -> &'static HashMap<String, String> {
        match self {
            Self::Fr => &FRENCH,
            Self::En => &ENGLISH,
        }
    }
}

/// A parameter: a count is a number, so that its plural is chosen.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Number(f64),
}

impl Value {
    fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(*number),
            Self::Text(text) => text.trim().parse().ok(),
        }
        .filter(|number: &f64| number.is_finite())
    }

    fn text(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Number(number) => plural::number(*number),
        }
    }
}

impl From<&str> for Value {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<String> for Value {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<u32> for Value {
    fn from(count: u32) -> Self {
        Self::Number(f64::from(count))
    }
}

impl From<f64> for Value {
    fn from(number: f64) -> Self {
        Self::Number(number)
    }
}

/// What code answers instead of a sentence: translated where it is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub key: Key,
    /// Named by the template's `{{name}}`s; an error from the core brings its own names.
    pub params: Vec<(String, Value)>,
}

impl Message {
    pub fn new(key: Key) -> Self {
        Self {
            key,
            params: Vec::new(),
        }
    }

    #[must_use]
    pub fn with(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.params.push((name.into(), value.into()));
        self
    }

    pub fn text(&self, language: Language) -> String {
        let params: Vec<(&str, Value)> = self
            .params
            .iter()
            .map(|(name, value)| (name.as_str(), value.clone()))
            .collect();
        translate(language, self.key, &params)
    }
}

/// The application's name, which no translation spells out: a string carries `{{app}}`.
const APP: &str = env!("DEVNOTES_DISPLAY_NAME");

/// The text of `key` in `language`: its plurals chosen, then its `{{name}}`s filled. A
/// parameter not given is left empty, as the reference's Transloco did; `{{app}}` is the
/// application's name unless a caller says otherwise.
pub fn translate(language: Language, key: Key, params: &[(&str, Value)]) -> String {
    let Some(template) = language.catalogue().get(key.path()) else {
        return key.path().to_string();
    };
    interpolate(&plural::expand(template, params, language), params)
}

fn interpolate(text: &str, params: &[(&str, Value)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find("{{") {
        let Some(length) = rest[open + 2..].find("}}") else {
            break;
        };
        let name = rest[open + 2..open + 2 + length].trim();
        out.push_str(&rest[..open]);
        if let Some((_, value)) = params.iter().find(|(param, _)| *param == name) {
            out.push_str(&value.text());
        } else if name == "app" {
            out.push_str(APP);
        }
        rest = &rest[open + 2 + length + 2..];
    }
    out.push_str(rest);
    out
}

static FRENCH: LazyLock<HashMap<String, String>> =
    LazyLock::new(|| flatten(include_str!("../translations/fr.json")));
static ENGLISH: LazyLock<HashMap<String, String>> =
    LazyLock::new(|| flatten(include_str!("../translations/en.json")));

/// `{"vault": {"unlockTitle": "…"}}` as `vault.unlockTitle`. Both files are checked by the
/// build and the tests: a file that does not read is a broken build, not an answer.
fn flatten(json: &str) -> HashMap<String, String> {
    let root: serde_json::Value = serde_json::from_str(json).expect("a translation file is JSON");
    let mut flat = HashMap::new();
    let mut pending = vec![(String::new(), &root)];
    while let Some((prefix, value)) = pending.pop() {
        match value {
            serde_json::Value::Object(entries) => {
                for (segment, child) in entries {
                    let path = if prefix.is_empty() {
                        segment.clone()
                    } else {
                        format!("{prefix}.{segment}")
                    };
                    pending.push((path, child));
                }
            }
            serde_json::Value::String(text) => {
                flat.insert(prefix, text.clone());
            }
            _ => {}
        }
    }
    flat
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// The `{{name}}`s a template fills in.
    fn filled(template: &str) -> BTreeSet<&str> {
        let mut names = BTreeSet::new();
        let mut rest = template;
        while let Some(open) = rest.find("{{") {
            let Some(length) = rest[open + 2..].find("}}") else {
                break;
            };
            names.insert(rest[open + 2..open + 2 + length].trim());
            rest = &rest[open + 2 + length + 2..];
        }
        names
    }

    /// Every name a template reads: filled in, or counted by a plural.
    fn used(template: &str) -> BTreeSet<&str> {
        let mut names = filled(template);
        for (at, _) in template.match_indices(", plural,") {
            let start = template[..at].rfind('{').map_or(0, |open| open + 1);
            names.insert(template[start..at].trim());
        }
        names
    }

    #[test]
    fn both_languages_have_the_same_keys() {
        let french: BTreeSet<&String> = FRENCH.keys().collect();
        let english: BTreeSet<&String> = ENGLISH.keys().collect();

        let only_french: Vec<_> = french.difference(&english).collect();
        let only_english: Vec<_> = english.difference(&french).collect();
        assert!(
            only_french.is_empty(),
            "missing in en.json: {only_french:?}"
        );
        assert!(
            only_english.is_empty(),
            "missing in fr.json: {only_english:?}"
        );
    }

    /// A name one language fills in, the other reads too, filled in or counted: « il y a
    /// {{count}} mois » counts in English. One may count where the other does not
    /// (« le lundi » / « les lundis », but « on Monday »).
    #[test]
    fn both_languages_take_the_same_parameters() {
        let differing: Vec<&String> = FRENCH
            .iter()
            .filter(|(key, french)| {
                ENGLISH.get(*key).is_some_and(|english| {
                    !filled(french).is_subset(&used(english))
                        || !filled(english).is_subset(&used(french))
                })
            })
            .map(|(key, _)| key)
            .collect();

        assert!(differing.is_empty(), "parameters differ: {differing:?}");
    }

    #[test]
    fn a_generated_key_names_its_path() {
        assert_eq!(keys::vault::UNLOCK_TITLE.path(), "vault.unlockTitle");
        assert_eq!(
            translate(Language::Fr, keys::vault::UNLOCK_TITLE, &[]),
            "Déverrouiller la bibliothèque"
        );
    }

    #[test]
    fn the_application_is_named_by_its_manifest_and_never_in_a_translation() {
        assert_eq!(
            interpolate("Bienvenue dans {{app}}", &[]),
            "Bienvenue dans DevNotes"
        );
        for language in Language::ALL {
            let spelled: Vec<&String> = language
                .catalogue()
                .iter()
                .filter(|(_, text)| text.contains(APP))
                .map(|(key, _)| key)
                .collect();
            assert!(
                spelled.is_empty(),
                "{language:?} spells the name out: {spelled:?}"
            );
        }
    }

    /// A count is a plural: « (s) » reads as a form left unfinished.
    #[test]
    fn no_translation_hedges_a_plural() {
        for language in Language::ALL {
            let hedged: Vec<&String> = language
                .catalogue()
                .iter()
                .filter(|(_, text)| text.contains("(s)"))
                .map(|(key, _)| key)
                .collect();
            assert!(hedged.is_empty(), "{language:?}: {hedged:?}");
        }
    }

    #[test]
    fn a_composed_key_is_found_only_when_it_exists() {
        assert_eq!(
            Key::find("vault.unlockTitle"),
            Some(keys::vault::UNLOCK_TITLE)
        );
        assert_eq!(Key::find("vault.noSuchKey"), None);
    }

    #[test]
    fn a_parameter_fills_its_place_and_a_missing_one_leaves_it_empty() {
        let params = [("name", Value::from("Perso"))];

        assert_eq!(
            interpolate("dans « {{ name }} »", &params),
            "dans « Perso »"
        );
        assert_eq!(interpolate("dans « {{other}} »", &params), "dans «  »");
        assert_eq!(interpolate("{{ unclosed", &params), "{{ unclosed");
    }

    #[test]
    fn a_message_is_translated_where_it_is_drawn() {
        let message = Message::new(keys::vault::UNLOCK_TITLE);

        assert_eq!(
            message.text(Language::En),
            translate(Language::En, keys::vault::UNLOCK_TITLE, &[])
        );
    }

    #[test]
    fn the_system_language_is_the_first_french_or_english_one() {
        let of = |locales: &[&str]| Language::first_of(locales.iter().map(ToString::to_string));

        assert_eq!(of(&["de-DE", "fr-CH", "en-US"]), Language::Fr);
        assert_eq!(of(&["en_GB", "fr"]), Language::En);
        assert_eq!(of(&["ja-JP"]), Language::En);
        assert_eq!(of(&[]), Language::En);
    }

    /// The reference's transpiler at counts that tell the rules apart, every plural key of
    /// both files (`golden/plurals`).
    #[test]
    fn every_plural_reads_as_the_reference_read_it() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../../golden/plurals/cases.json")).unwrap();
        let mut wrong = Vec::new();
        for case in corpus.as_array().unwrap() {
            let path = case["input"].as_str().unwrap();
            let options = case["options"].as_object().unwrap();
            let language = match options["lang"].as_str().unwrap() {
                "fr" => Language::Fr,
                _ => Language::En,
            };
            let params: Vec<(&str, Value)> = options
                .iter()
                .filter(|(name, _)| *name != "lang")
                .map(|(name, value)| (name.as_str(), Value::Number(value.as_f64().unwrap())))
                .collect();
            let key = Key::find(path).unwrap_or_else(|| panic!("{path} is no key"));
            let text = translate(language, key, &params);
            if text != case["output"].as_str().unwrap() {
                wrong.push(format!(
                    "{path} {options:?}: {text:?}, expected {}",
                    case["output"]
                ));
            }
        }

        assert!(
            wrong.is_empty(),
            "{} of them differ: {:#?}",
            wrong.len(),
            &wrong[..wrong.len().min(10)]
        );
    }
}
