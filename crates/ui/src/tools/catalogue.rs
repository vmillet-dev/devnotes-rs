//! Every tool, in its panel's order, a file each beside this one. Adding one is an entry here:
//! its name and its line are read from its id.

mod ascii;
mod base64;
mod case;
mod checks;
mod cidr;
mod cron;
mod dates;
mod diff;
mod hash;
mod jwt;
mod line_breaks;
mod lorem;
mod password;
mod permissions;
mod ports;
mod reference;
mod sizes;
mod text_stats;
mod url_codec;
mod zones;

use egui::Color32;
use engine::notes::kind::NoteKind;
use engine::notes::language::Language as NoteLanguage;
use i18n::{Key, Language, keys};

use super::Tool;
use crate::theme::icons::Icon;
use crate::theme::tokens::Palette;

/// In the home's order, which the rail follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Text,
    Crypto,
    Encode,
    Generate,
    Time,
    Calc,
    Reference,
}

impl Category {
    pub const ALL: [Self; 7] = [
        Self::Text,
        Self::Crypto,
        Self::Encode,
        Self::Generate,
        Self::Time,
        Self::Calc,
        Self::Reference,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Crypto => "crypto",
            Self::Encode => "encode",
            Self::Generate => "generate",
            Self::Time => "time",
            Self::Calc => "calc",
            Self::Reference => "reference",
        }
    }

    pub fn name(self) -> Key {
        match self {
            Self::Text => keys::tools::categories::TEXT,
            Self::Crypto => keys::tools::categories::CRYPTO,
            Self::Encode => keys::tools::categories::ENCODE,
            Self::Generate => keys::tools::categories::GENERATE,
            Self::Time => keys::tools::categories::TIME,
            Self::Calc => keys::tools::categories::CALC,
            Self::Reference => keys::tools::categories::REFERENCE,
        }
    }

    /// Its mark, on the home's panels.
    pub fn icon(self) -> Icon {
        match self {
            Self::Text => Icon::Type,
            Self::Crypto => Icon::Lock,
            Self::Encode => Icon::Code,
            Self::Generate => Icon::Sparkle,
            Self::Time => Icon::Clock,
            Self::Calc => Icon::Calculator,
            Self::Reference => Icon::BookOpen,
        }
    }

    pub fn hue(self, p: &Palette) -> Color32 {
        match self {
            Self::Text => p.blue,
            Self::Crypto => p.purple,
            Self::Encode => p.green,
            Self::Generate => p.red,
            Self::Time => p.teal,
            Self::Calc => p.pink,
            Self::Reference => p.olive,
        }
    }
}

/// Which notes can fill an input: no kind listed is every kind; a language narrows to the
/// snippets written in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteInput {
    pub kinds: &'static [NoteKind],
    pub languages: &'static [NoteLanguage],
}

/// A todo list gives its Markdown, as copying it does.
const ANY_NOTE: NoteInput = NoteInput {
    kinds: &[],
    languages: &[],
};
const JSON_SNIPPET: NoteInput = NoteInput {
    kinds: &[NoteKind::Snippet],
    languages: &[NoteLanguage::Json],
};
const STRUCTURED_SNIPPET: NoteInput = NoteInput {
    kinds: &[NoteKind::Snippet],
    languages: &[
        NoteLanguage::Json,
        NoteLanguage::Yml,
        NoteLanguage::Toml,
        NoteLanguage::Xml,
    ],
};

/// Its name and its line are the keys `tools.<id>.name` and `tools.<id>.description`.
pub struct ToolEntry {
    pub id: &'static str,
    pub category: Category,
    /// What a search also finds it by, in no language in particular: `md5`, `camelCase`.
    pub keywords: &'static [&'static str],
    /// The inputs a note can fill, by the name the tool gives them. ⚠️ A key, a password, a token
    /// or a card number is never one: its field offers no import.
    pub from_note: &'static [(&'static str, NoteInput)],
    pub make: fn() -> Box<dyn Tool>,
}

impl ToolEntry {
    pub fn name(&self, language: Language) -> String {
        self.word(language, "name")
    }

    pub fn line(&self, language: Language) -> String {
        self.word(language, "description")
    }

    fn word(&self, language: Language, which: &str) -> String {
        Key::find(&format!("tools.{}.{which}", self.id))
            .map(|key| i18n::translate(language, key, &[]))
            .unwrap_or_default()
    }
}

impl std::fmt::Debug for ToolEntry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.id)
    }
}

pub fn find(id: &str) -> Option<&'static ToolEntry> {
    TOOLS.iter().find(|tool| tool.id == id)
}

pub fn in_category(category: Category) -> impl Iterator<Item = &'static ToolEntry> {
    TOOLS.iter().filter(move |tool| tool.category == category)
}

/// A tool whose folder has not come over yet: its place in the catalogue, an empty body.
struct Coming;

impl Tool for Coming {
    fn ui(&mut self, _: &mut egui::Ui, _: &mut super::Drawn<'_>) {}

    fn clear(&mut self) {}
}

fn coming() -> Box<dyn Tool> {
    Box::new(Coming)
}

/// A table to search: nothing is computed, and no note fills it.
const fn reference(
    id: &'static str,
    keywords: &'static [&'static str],
    make: fn() -> Box<dyn Tool>,
) -> ToolEntry {
    ToolEntry {
        id,
        category: Category::Reference,
        keywords,
        from_note: &[],
        make,
    }
}

pub static TOOLS: [ToolEntry; 32] = [
    ToolEntry {
        id: "case",
        category: Category::Text,
        keywords: &[
            "camelCase",
            "PascalCase",
            "snake_case",
            "kebab-case",
            "CONSTANT_CASE",
            "dot.case",
            "path/case",
            "Train-Case",
            "lowercase",
            "UPPERCASE",
            "flatcase",
            "slug",
            "permalink",
            "ascii",
        ],
        from_note: &[("text", ANY_NOTE)],
        make: case::make,
    },
    ToolEntry {
        id: "line-breaks",
        category: Category::Text,
        keywords: &["crlf", "lf", "eol", "newline", "whitespace"],
        from_note: &[("text", ANY_NOTE)],
        make: line_breaks::make,
    },
    ToolEntry {
        id: "text-stats",
        category: Category::Text,
        keywords: &[
            "count",
            "characters",
            "words",
            "lines",
            "length",
            "frequency",
            "compter",
            "caractères",
            "mots",
        ],
        from_note: &[("text", ANY_NOTE)],
        make: text_stats::make,
    },
    ToolEntry {
        id: "diff",
        category: Category::Text,
        keywords: &[
            "diff", "compare", "patch", "diff -u", "unified", "json", "rfc 6902",
        ],
        from_note: &[("text", ANY_NOTE), ("structured", STRUCTURED_SNIPPET)],
        make: diff::make,
    },
    ToolEntry {
        id: "hash",
        category: Category::Crypto,
        keywords: &[
            "md5", "sha1", "sha256", "sha512", "sha3", "hmac", "digest", "checksum", "webhook",
        ],
        from_note: &[("text", ANY_NOTE)],
        make: hash::make,
    },
    ToolEntry {
        id: "password",
        category: Category::Crypto,
        keywords: &["password", "passphrase", "secret", "random", "entropy"],
        from_note: &[],
        make: password::make,
    },
    ToolEntry {
        id: "identifiers",
        category: Category::Crypto,
        keywords: &[
            "uuid",
            "guid",
            "v4",
            "v7",
            "ulid",
            "nanoid",
            "cuid",
            "objectid",
            "mongodb",
            "ksuid",
            "identifier",
        ],
        from_note: &[],
        make: coming,
    },
    ToolEntry {
        id: "jwt",
        category: Category::Crypto,
        keywords: &[
            "jwt",
            "json web token",
            "bearer",
            "hs256",
            "claims",
            "exp",
            "token",
        ],
        from_note: &[],
        make: jwt::make,
    },
    ToolEntry {
        id: "url-codec",
        category: Category::Encode,
        keywords: &[
            "url",
            "uri",
            "query",
            "querystring",
            "percent",
            "encodeURIComponent",
            "escape",
            "parse",
        ],
        from_note: &[("text", ANY_NOTE)],
        make: url_codec::make,
    },
    ToolEntry {
        id: "base64",
        category: Category::Encode,
        keywords: &[
            "base64",
            "b64",
            "base32",
            "hex",
            "hexadecimal",
            "binary",
            "binaire",
            "bytes",
            "jwt",
            "data uri",
        ],
        from_note: &[("text", ANY_NOTE)],
        make: base64::make,
    },
    ToolEntry {
        id: "convert",
        category: Category::Encode,
        keywords: &["json", "toml", "xml", "yaml", "yml", "convert"],
        from_note: &[("text", STRUCTURED_SNIPPET)],
        make: coming,
    },
    ToolEntry {
        id: "colour",
        category: Category::Encode,
        keywords: &["color", "hex", "rgb", "hsl", "oklch", "contrast", "wcag"],
        from_note: &[],
        make: coming,
    },
    ToolEntry {
        id: "json-generator",
        category: Category::Generate,
        keywords: &["json", "schema", "mock", "fixture", "fake", "faker"],
        from_note: &[("text", JSON_SNIPPET)],
        make: coming,
    },
    ToolEntry {
        id: "identity",
        category: Category::Generate,
        keywords: &[
            "identity",
            "identité",
            "person",
            "personne",
            "fake",
            "faker",
            "test data",
            "iban",
            "siren",
            "siret",
            "nir",
        ],
        from_note: &[],
        make: coming,
    },
    ToolEntry {
        id: "lorem",
        category: Category::Generate,
        keywords: &["lorem", "ipsum", "placeholder", "text"],
        from_note: &[],
        make: lorem::make,
    },
    ToolEntry {
        id: "qr",
        category: Category::Generate,
        keywords: &[
            "qr", "qr code", "qrcode", "wifi", "vcard", "mailto", "svg", "png", "barcode",
        ],
        from_note: &[("text", ANY_NOTE)],
        make: coming,
    },
    ToolEntry {
        id: "dates",
        category: Category::Time,
        keywords: &[
            "timestamp",
            "unix",
            "epoch",
            "iso 8601",
            "rfc 3339",
            "rfc 2822",
            "date",
            "now",
        ],
        from_note: &[],
        make: dates::make,
    },
    ToolEntry {
        id: "zones",
        category: Category::Time,
        keywords: &[
            "timezone",
            "time zone",
            "tz",
            "iana",
            "utc",
            "gmt",
            "dst",
            "offset",
        ],
        from_note: &[],
        make: zones::make,
    },
    ToolEntry {
        id: "cron",
        category: Category::Time,
        keywords: &["cron", "crontab", "schedule", "quartz", "@daily", "job"],
        from_note: &[],
        make: cron::make,
    },
    ToolEntry {
        id: "sizes",
        category: Category::Calc,
        keywords: &[
            "byte", "octet", "kb", "mb", "gb", "kib", "mib", "gib", "bit", "mbit", "size",
        ],
        from_note: &[],
        make: sizes::make,
    },
    ToolEntry {
        id: "permissions",
        category: Category::Calc,
        keywords: &[
            "chmod", "umask", "rwx", "octal", "setuid", "setgid", "sticky", "ls -l",
        ],
        from_note: &[],
        make: permissions::make,
    },
    ToolEntry {
        id: "checks",
        category: Category::Calc,
        keywords: &[
            "luhn",
            "iban",
            "mod 97",
            "credit card",
            "carte",
            "checksum",
            "check digits",
            "contrôle",
            "clé",
            "bic",
            "visa",
            "mastercard",
        ],
        from_note: &[],
        make: checks::make,
    },
    ToolEntry {
        id: "cidr",
        category: Category::Calc,
        keywords: &[
            "cidr",
            "subnet",
            "sous-réseau",
            "netmask",
            "masque",
            "ipv4",
            "ipv6",
            "ip",
            "rfc 1918",
            "broadcast",
            "wildcard",
            "vlsm",
            "plan",
            "supernet",
            "agrégation",
            "aggregate",
            "in-addr.arpa",
            "reverse dns",
        ],
        from_note: &[],
        make: cidr::make,
    },
    reference(
        "http-status",
        &[
            "http", "status", "statut", "code", "rfc 9110", "404", "500", "redirect", "nginx",
        ],
        coming,
    ),
    reference(
        "http-headers",
        &[
            "header",
            "en-tête",
            "cors",
            "csp",
            "hsts",
            "cookie",
            "cache-control",
            "content-type",
        ],
        coming,
    ),
    reference(
        "mime-types",
        &[
            "mime",
            "media type",
            "content-type",
            "extension",
            "json",
            "png",
            "pdf",
        ],
        coming,
    ),
    reference(
        "ports",
        &[
            "port",
            "tcp",
            "udp",
            "iana",
            "ssh",
            "postgresql",
            "redis",
            "firewall",
            "pare-feu",
        ],
        ports::make,
    ),
    reference(
        "signals",
        &[
            "signal", "kill", "sigterm", "sigkill", "sighup", "sigint", "posix", "trap",
        ],
        coming,
    ),
    reference(
        "ascii",
        &[
            "ascii",
            "character",
            "caractère",
            "code",
            "hex",
            "control",
            "newline",
            "escape",
        ],
        ascii::make,
    ),
    reference(
        "regex-sheet",
        &[
            "regex",
            "regexp",
            "expression régulière",
            "pcre",
            "lookbehind",
            "lookahead",
            "cheat sheet",
        ],
        coming,
    ),
    reference(
        "markdown-sheet",
        &[
            "markdown",
            "md",
            "gfm",
            "commonmark",
            "readme",
            "cheat sheet",
        ],
        coming,
    ),
    reference(
        "mustache-sheet",
        &[
            "mustache",
            "handlebars",
            "template",
            "gabarit",
            "{{",
            "partial",
            "cheat sheet",
        ],
        coming,
    ),
];

#[cfg(test)]
mod calc_tests;
#[cfg(test)]
mod encoding_tests;
#[cfg(test)]
mod text_tests;
#[cfg(test)]
mod time_tests;
