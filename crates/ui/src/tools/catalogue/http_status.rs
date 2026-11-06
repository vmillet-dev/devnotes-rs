//! The status codes: the facts here, what each means in the reference's words.

use i18n::{Language, keys};

use super::reference::{Reference, Table, columns_of, groups_of, word};
use crate::tools::Tool;
use crate::tools::ui::{Cell, ReferenceRow};

const CLASSES: [&str; 5] = ["1xx", "2xx", "3xx", "4xx", "5xx"];
/// A card is no narrower: four columns of sentences.
const WIDE_CARD: f32 = 760.0;

/// The code, the reason phrase as specified, who defines it, and its standing outside the
/// registry.
type Status = (u32, &'static str, &'static str, Option<&'static str>);

const STATUSES: [Status; 78] = [
    (100, "Continue", "RFC 9110", None),
    (101, "Switching Protocols", "RFC 9110", None),
    (102, "Processing", "RFC 2518", Some("obsolete")),
    (103, "Early Hints", "RFC 8297", None),
    (200, "OK", "RFC 9110", None),
    (201, "Created", "RFC 9110", None),
    (202, "Accepted", "RFC 9110", None),
    (203, "Non-Authoritative Information", "RFC 9110", None),
    (204, "No Content", "RFC 9110", None),
    (205, "Reset Content", "RFC 9110", None),
    (206, "Partial Content", "RFC 9110", None),
    (207, "Multi-Status", "RFC 4918", None),
    (208, "Already Reported", "RFC 5842", None),
    (226, "IM Used", "RFC 3229", None),
    (300, "Multiple Choices", "RFC 9110", None),
    (301, "Moved Permanently", "RFC 9110", None),
    (302, "Found", "RFC 9110", None),
    (303, "See Other", "RFC 9110", None),
    (304, "Not Modified", "RFC 9110", None),
    (305, "Use Proxy", "RFC 9110", Some("obsolete")),
    (307, "Temporary Redirect", "RFC 9110", None),
    (308, "Permanent Redirect", "RFC 9110", None),
    (400, "Bad Request", "RFC 9110", None),
    (401, "Unauthorized", "RFC 9110", None),
    (402, "Payment Required", "RFC 9110", None),
    (403, "Forbidden", "RFC 9110", None),
    (404, "Not Found", "RFC 9110", None),
    (405, "Method Not Allowed", "RFC 9110", None),
    (406, "Not Acceptable", "RFC 9110", None),
    (407, "Proxy Authentication Required", "RFC 9110", None),
    (408, "Request Timeout", "RFC 9110", None),
    (409, "Conflict", "RFC 9110", None),
    (410, "Gone", "RFC 9110", None),
    (411, "Length Required", "RFC 9110", None),
    (412, "Precondition Failed", "RFC 9110", None),
    (413, "Content Too Large", "RFC 9110", None),
    (414, "URI Too Long", "RFC 9110", None),
    (415, "Unsupported Media Type", "RFC 9110", None),
    (416, "Range Not Satisfiable", "RFC 9110", None),
    (417, "Expectation Failed", "RFC 9110", None),
    (418, "I'm a teapot", "RFC 2324", Some("unofficial")),
    (421, "Misdirected Request", "RFC 9110", None),
    (422, "Unprocessable Content", "RFC 9110", None),
    (423, "Locked", "RFC 4918", None),
    (424, "Failed Dependency", "RFC 4918", None),
    (425, "Too Early", "RFC 8470", None),
    (426, "Upgrade Required", "RFC 9110", None),
    (428, "Precondition Required", "RFC 6585", None),
    (429, "Too Many Requests", "RFC 6585", None),
    (431, "Request Header Fields Too Large", "RFC 6585", None),
    (440, "Login Time-out", "IIS", Some("unofficial")),
    (444, "No Response", "nginx", Some("unofficial")),
    (449, "Retry With", "IIS", Some("unofficial")),
    (451, "Unavailable For Legal Reasons", "RFC 7725", None),
    (494, "Request Header Too Large", "nginx", Some("unofficial")),
    (495, "SSL Certificate Error", "nginx", Some("unofficial")),
    (496, "SSL Certificate Required", "nginx", Some("unofficial")),
    (
        497,
        "HTTP Request Sent to HTTPS Port",
        "nginx",
        Some("unofficial"),
    ),
    (499, "Client Closed Request", "nginx", Some("unofficial")),
    (500, "Internal Server Error", "RFC 9110", None),
    (501, "Not Implemented", "RFC 9110", None),
    (502, "Bad Gateway", "RFC 9110", None),
    (503, "Service Unavailable", "RFC 9110", None),
    (504, "Gateway Timeout", "RFC 9110", None),
    (505, "HTTP Version Not Supported", "RFC 9110", None),
    (506, "Variant Also Negotiates", "RFC 2295", None),
    (507, "Insufficient Storage", "RFC 4918", None),
    (508, "Loop Detected", "RFC 5842", None),
    (
        509,
        "Bandwidth Limit Exceeded",
        "Apache",
        Some("unofficial"),
    ),
    (510, "Not Extended", "RFC 2774", Some("obsolete")),
    (511, "Network Authentication Required", "RFC 6585", None),
    (
        520,
        "Web Server Returned an Unknown Error",
        "Cloudflare",
        Some("unofficial"),
    ),
    (521, "Web Server Is Down", "Cloudflare", Some("unofficial")),
    (
        522,
        "Connection Timed Out",
        "Cloudflare",
        Some("unofficial"),
    ),
    (
        523,
        "Origin Is Unreachable",
        "Cloudflare",
        Some("unofficial"),
    ),
    (524, "A Timeout Occurred", "Cloudflare", Some("unofficial")),
    (
        525,
        "SSL Handshake Failed",
        "Cloudflare",
        Some("unofficial"),
    ),
    (
        526,
        "Invalid SSL Certificate",
        "Cloudflare",
        Some("unofficial"),
    ),
];

fn row((code, name, by, standing): Status, language: Language) -> ReferenceRow {
    let group = format!("{}xx", code / 100);
    let key = code.to_string();
    let meaning = word(
        language,
        &format!("tools.http-status.words.codes.{code}.meaning"),
    );
    let usage = word(
        language,
        &format!("tools.http-status.words.codes.{code}.use"),
    );
    let heading = word(language, &format!("tools.http-status.groups.{group}"));
    let mut named = Cell::text(name);
    if let Some(standing) = standing {
        named = named.below(word(
            language,
            &format!("tools.http-status.standing.{standing}"),
        ));
    }
    let mut said = Cell::text(meaning.clone());
    if !usage.is_empty() {
        let lead = i18n::translate(language, keys::tools::http_status::USE, &[]);
        said = said.below(format!("{lead} {usage}"));
    }
    let cells = vec![Cell::mono(key.clone()), named, said, Cell::text(by)];
    let title = format!("{key} {name}");
    ReferenceRow::new(key.clone(), group, cells)
        .searched_by([key.as_str(), name, &meaning, &usage, by, &heading])
        .found_as(title, meaning)
}

pub(crate) fn table(language: Language) -> Table {
    Table {
        rows: STATUSES.iter().map(|it| row(*it, language)).collect(),
        groups: groups_of(language, "tools.http-status.groups", &CLASSES),
        columns: columns_of(
            language,
            "tools.http-status.columns",
            &[
                ("code", Some(0.07)),
                ("name", Some(0.27)),
                ("meaning", None),
                ("by", Some(0.13)),
            ],
        ),
        caption: keys::tools::http_status::NAME,
        placeholder: keys::tools::http_status::PLACEHOLDER,
        card_width: WIDE_CARD,
        matches: None,
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Reference::of(table)
}
