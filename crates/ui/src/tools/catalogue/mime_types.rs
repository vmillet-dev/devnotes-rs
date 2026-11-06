//! The MIME types: the facts here, what each is in the reference's words.

use i18n::{Language, keys};

use super::reference::{Reference, Table, columns_of, word};
use crate::tools::Tool;
use crate::tools::ui::{Cell, Group, ReferenceRow};

/// Named by themselves: a type's first word is the same in every language.
const GROUPS: [&str; 7] = [
    "text",
    "application",
    "image",
    "audio",
    "video",
    "font",
    "multipart",
];
const WIDE_CARD: f32 = 760.0;

/// A type and its extensions, the most common first; none for a type that never sits in a file.
const TYPES: [(&str, &[&str]); 91] = [
    ("text/plain", &[".txt", ".text", ".log"]),
    ("text/html", &[".html", ".htm"]),
    ("text/css", &[".css"]),
    ("text/javascript", &[".js", ".mjs", ".cjs"]),
    ("text/csv", &[".csv"]),
    ("text/tab-separated-values", &[".tsv"]),
    ("text/markdown", &[".md", ".markdown"]),
    ("text/calendar", &[".ics"]),
    ("text/vcard", &[".vcf", ".vcard"]),
    ("text/xml", &[".xml"]),
    ("text/event-stream", &[]),
    ("application/json", &[".json"]),
    ("application/ld+json", &[".jsonld"]),
    ("application/geo+json", &[".geojson"]),
    ("application/manifest+json", &[".webmanifest"]),
    ("application/problem+json", &[]),
    ("application/x-ndjson", &[".ndjson", ".jsonl"]),
    ("application/xml", &[".xml", ".xsd", ".xsl"]),
    ("application/xhtml+xml", &[".xhtml"]),
    ("application/rss+xml", &[".rss"]),
    ("application/atom+xml", &[".atom"]),
    ("application/yaml", &[".yaml", ".yml"]),
    ("application/toml", &[".toml"]),
    ("application/sql", &[".sql"]),
    ("application/wasm", &[".wasm"]),
    ("application/pdf", &[".pdf"]),
    ("application/rtf", &[".rtf"]),
    ("application/epub+zip", &[".epub"]),
    ("application/msword", &[".doc"]),
    (
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        &[".docx"],
    ),
    ("application/vnd.ms-excel", &[".xls"]),
    (
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        &[".xlsx"],
    ),
    ("application/vnd.ms-powerpoint", &[".ppt"]),
    (
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        &[".pptx"],
    ),
    ("application/vnd.oasis.opendocument.text", &[".odt"]),
    ("application/vnd.oasis.opendocument.spreadsheet", &[".ods"]),
    ("application/zip", &[".zip"]),
    ("application/gzip", &[".gz", ".tgz"]),
    ("application/x-tar", &[".tar"]),
    ("application/x-bzip2", &[".bz2"]),
    ("application/x-xz", &[".xz"]),
    ("application/zstd", &[".zst"]),
    ("application/x-7z-compressed", &[".7z"]),
    ("application/vnd.rar", &[".rar"]),
    ("application/java-archive", &[".jar"]),
    ("application/vnd.android.package-archive", &[".apk"]),
    (
        "application/vnd.microsoft.portable-executable",
        &[".exe", ".dll"],
    ),
    ("application/vnd.debian.binary-package", &[".deb"]),
    ("application/x-sh", &[".sh"]),
    ("application/vnd.ms-fontobject", &[".eot"]),
    ("application/octet-stream", &[".bin"]),
    ("application/x-www-form-urlencoded", &[]),
    ("application/jwt", &[]),
    ("image/png", &[".png"]),
    ("image/jpeg", &[".jpg", ".jpeg", ".jfif"]),
    ("image/gif", &[".gif"]),
    ("image/webp", &[".webp"]),
    ("image/avif", &[".avif"]),
    ("image/apng", &[".apng"]),
    ("image/svg+xml", &[".svg"]),
    ("image/vnd.microsoft.icon", &[".ico"]),
    ("image/bmp", &[".bmp"]),
    ("image/tiff", &[".tif", ".tiff"]),
    ("image/heic", &[".heic"]),
    ("audio/mpeg", &[".mp3"]),
    ("audio/aac", &[".aac"]),
    ("audio/mp4", &[".m4a"]),
    ("audio/ogg", &[".ogg", ".oga"]),
    ("audio/opus", &[".opus"]),
    ("audio/wav", &[".wav"]),
    ("audio/flac", &[".flac"]),
    ("audio/webm", &[".weba"]),
    ("audio/midi", &[".mid", ".midi"]),
    ("video/mp4", &[".mp4", ".m4v"]),
    ("video/webm", &[".webm"]),
    ("video/ogg", &[".ogv"]),
    ("video/quicktime", &[".mov"]),
    ("video/x-msvideo", &[".avi"]),
    ("video/x-matroska", &[".mkv"]),
    ("video/mpeg", &[".mpeg", ".mpg"]),
    ("video/mp2t", &[".ts", ".m2ts"]),
    ("video/3gpp", &[".3gp"]),
    ("font/woff2", &[".woff2"]),
    ("font/woff", &[".woff"]),
    ("font/ttf", &[".ttf"]),
    ("font/otf", &[".otf"]),
    ("font/collection", &[".ttc"]),
    ("multipart/form-data", &[]),
    ("multipart/byteranges", &[]),
    ("multipart/mixed", &[]),
    ("multipart/alternative", &[]),
];

/// `.webp` and `webp` both find image/webp: each extension is searched with its dot and without.
fn row((mime, extensions): (&str, &[&str]), language: Language) -> ReferenceRow {
    let description = word(language, &format!("tools.mime-types.words.types.{mime}"));
    let group = mime.split('/').next().unwrap_or_default();
    let shown = if extensions.is_empty() {
        i18n::translate(language, keys::tools::mime_types::NO_EXTENSION, &[])
    } else {
        extensions.join("  ")
    };
    let cells = vec![
        Cell::mono(mime),
        Cell::mono(shown),
        Cell::text(description.clone()),
    ];
    let bare = extensions.iter().map(|it| it.trim_start_matches('.'));
    let searched = [mime, description.as_str()]
        .into_iter()
        .chain(extensions.iter().copied())
        .chain(bare);
    ReferenceRow::new(mime, group, cells)
        .searched_by(searched)
        .found_as(mime, description.clone())
}

pub(crate) fn table(language: Language) -> Table {
    Table {
        rows: TYPES.iter().map(|it| row(*it, language)).collect(),
        groups: GROUPS
            .iter()
            .map(|id| Group {
                id: (*id).to_owned(),
                label: (*id).to_owned(),
            })
            .collect(),
        columns: columns_of(
            language,
            "tools.mime-types.columns",
            &[
                ("type", Some(0.36)),
                ("extensions", Some(0.24)),
                ("description", None),
            ],
        ),
        caption: keys::tools::mime_types::NAME,
        placeholder: keys::tools::mime_types::PLACEHOLDER,
        card_width: WIDE_CARD,
        matches: None,
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Reference::of(table)
}
