//! The acceptance sheet's main paths through the tools: each case typed as the sheet writes it and
//! its answer read from the window, in French, the sheet's language. A step opens on the case it
//! runs, so that a run can be read back into the sheet.

use e2e::{App, PASSPHRASE, step};
use egui::{Key, Modifiers};

const FRENCH: &str = "setting-locale-1";
const JWT: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJyZWNldHRlIiwibmFtZSI6IlJlY2V0dGUgRGV2Tm90ZXMiLCJpYXQiOjE3NjcyMjU2MDAsImV4cCI6MTkyNDk5MjAwMH0.89V7D9SwqkMbyIoAbtwhgH3y8UgxyoI4vKAN_pgawSY";
const PIXEL: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";
const SCHEMA: &str = r#"{"type":"object","required":["id","email","age","role"],"properties":{"id":{"type":"string","format":"uuid"},"email":{"type":"string","format":"email"},"age":{"type":"integer","minimum":18,"maximum":99},"role":{"enum":["admin","lecteur","redacteur"]}}}"#;

/// One line of the sheet: the tool, its field, what is typed, and what must be read.
type Case = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
);

const CASES: &[Case] = &[
    (
        "TXT-01",
        "case",
        "case-input",
        "parse HTTP response",
        &[
            "parseHttpResponse",
            "ParseHttpResponse",
            "parse_http_response",
            "PARSE_HTTP_RESPONSE",
            "parse-http-response",
            "Parse-Http-Response",
            "parse.http.response",
            "parse/http/response",
            "parsehttpresponse",
            "Parse Http Response",
            "Parse http response",
            "PARSE HTTP RESPONSE",
        ],
    ),
    (
        "CRY-01",
        "hash",
        "hash-input",
        "DevNotes",
        &[
            "7fd0b69809fbca9dc53d03ab5450e18d",
            "f8148cf8eb16a1419d5ffa6819b8338ed76a13b2",
            "7a215fd1357dc4712283d610bd18335a1e28066bc839535bca7c10c5069ea3bb",
            "a1c24cfd5aac355e70003a964af2264228f8214317d3f9f004a89133b340a2ed",
            "345e99d395",
            "8 octets",
        ],
    ),
    (
        "ENC-01",
        "url-codec",
        "url-codec-decoded",
        "café crème & croissants",
        &["caf%C3%A9%20cr%C3%A8me%20%26%20croissants"],
    ),
    (
        "ENC-06",
        "base64",
        "base64-input",
        "Café ☕",
        &[
            "6 caractères",
            "9 octets",
            "Q2Fmw6kg4piV",
            "INQWNQ5JEDRJRFI=",
            "43 61 66 c3 a9 20 e2 98 95",
            "67 97 102 195 169 32 226 152 149",
        ],
    ),
    (
        "ENC-13",
        "convert",
        "convert-input",
        r#"{"service":"billing","replicas":2,"tags":["api","web"],"debug":null}"#,
        &["service: billing", "replicas: 2", "debug:"],
    ),
    (
        "ENC-19",
        "colour",
        "colour-input",
        "#1e90ff",
        &["#1e90ff", "30 144 255", "209.6", "oklch("],
    ),
    (
        "GEN-18",
        "qr",
        "qr-url",
        "https://devnotes.fr",
        &["Récupérable"],
    ),
    (
        "TPS-01",
        "dates",
        "dates-input",
        "1790000000",
        &["2026-09-21T14:13:20Z", "secondes", "lundi"],
    ),
    (
        "TPS-18",
        "cron",
        "cron-input",
        "*/15 9-18 * * MON-FRI",
        &["15 minutes", "lundi", "vendredi"],
    ),
    (
        "CMP-01",
        "sizes",
        "sizes-input",
        "1,5 Go",
        &["1 500 000 000", "1,397", "1 430,51", "12"],
    ),
    (
        "CMP-07",
        "permissions",
        "permissions-octal",
        "755",
        &["rwxr-xr-x", "chmod"],
    ),
    (
        "CMP-13",
        "checks",
        "checks-input",
        "4111 1111 1111 1111",
        &["Valide", "Visa"],
    ),
    (
        "CMP-13",
        "checks",
        "checks-input",
        "4111 1111 1111 1112",
        &["Invalide", "Le dernier chiffre devrait être 1."],
    ),
    (
        "CMP-13",
        "checks",
        "checks-input",
        "378282246310005",
        &["Valide", "American Express"],
    ),
    (
        "CMP-13",
        "checks",
        "checks-input",
        "5555 5555 5555 4444",
        &["Valide", "Mastercard"],
    ),
    (
        "CMP-14",
        "checks",
        "checks-input",
        "FR14 2004 1010 0505 0001 3M02 606",
        &["20041", "01005", "0500013M026", "06", "Valide"],
    ),
    (
        "CMP-19",
        "cidr",
        "cidr-network",
        "10.24.8.0/21",
        &[
            "10.24.8.0",
            "255.255.248.0",
            "0.0.7.255",
            "10.24.8.1",
            "10.24.15.254",
            "10.24.15.255",
            "2 046",
            "2 048",
        ],
    ),
];

/// From the home, its search emptied: the window is tall enough for every tool.
fn open_tool(app: &mut App, tool: &str) {
    if app.has("tool-back") {
        app.click("tool-back");
    }
    app.wait_for("tools-search");
    app.click("tools-search");
    app.press(Key::A, Modifiers::COMMAND);
    app.press(Key::Backspace, Modifiers::NONE);
    app.click(&format!("tools-entry-{tool}"));
    app.wait_for("tool-title");
}

fn reads(app: &mut App, words: &[&str]) {
    for word in words {
        app.eventually(&format!("« {word} » is read"), |tree| tree.mentions(word));
    }
}

fn retype(app: &mut App, field: &str, input: &str) {
    app.click(field);
    app.press(Key::A, Modifiers::COMMAND);
    app.press(Key::Backspace, Modifiers::NONE);
    app.paste(input);
}

fn in_french(app: &mut App) {
    app.click("file-menu");
    app.click("file-preferences");
    app.click("setting-locale");
    app.click(FRENCH);
    app.click("settings-close");
    app.eventually("the panel closed", |tree| !tree.has("settings-dialog"));
}

fn the_tools(scenario: &'static str, profile: &std::path::Path) -> App {
    let mut app = App::launch(scenario, profile);
    app.create_library();
    in_french(&mut app);
    app.resize(1100, 1400);
    app.press(Key::Num2, Modifiers::COMMAND);
    app.wait_for("area-tools");
    app
}

#[test]
fn recette_the_table_of_the_sheet() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = the_tools("39_recette_tools", profile.path());
    for (case, tool, field, input, words) in CASES {
        step(&format!("{case} {tool}: {input}"), || {
            open_tool(&mut app, tool);
            retype(&mut app, field, input);
            reads(&mut app, words);
            app.capture(&format!("{case}-{tool}"));
        });
    }
    app.quit();
}

#[test]
fn recette_tools_of_several_gestures() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = the_tools("39_recette_gestures", profile.path());

    step("TXT-07 mixed endings are read and put right", || {
        open_tool(&mut app, "line-breaks");
        retype(
            &mut app,
            "line-breaks-input",
            "un\r\ndeux  \ntrois\rquatre ",
        );
        app.wait_for("line-breaks-found");
        app.capture("TXT-06");
        app.click("line-breaks-trim");
        app.capture("TXT-07");
    });

    step("TXT-08 what is counted of a flag", || {
        open_tool(&mut app, "text-stats");
        retype(&mut app, "text-stats-input", "Hé 🇫🇷\nok");
        reads(&mut app, &["7", "8", "10", "15"]);
        app.capture("TXT-08");
    });

    step("TXT-11 two texts compared", || {
        open_tool(&mut app, "diff");
        retype(&mut app, "text-diff-a", "un\ndeux\ntrois\n");
        retype(&mut app, "text-diff-b", "un\n2\n3\n");
        app.wait_for("text-diff-rows");
        app.capture("TXT-11");
        app.click("tool-clear");
    });

    step("TXT-14 two documents compared as data", || {
        app.click("diff-mode-1");
        retype(
            &mut app,
            "structured-diff-a",
            r#"{"total":126,"client":{"ville":"Lyon"},"remise":null}"#,
        );
        retype(
            &mut app,
            "structured-diff-b",
            r#"{"total":138,"client":{"ville":"Villeurbanne"},"payeeLe":"2026-10-02"}"#,
        );
        reads(&mut app, &["total", "client.ville", "remise", "payeeLe"]);
        app.capture("TXT-14");
        app.click("tool-clear");
    });

    app.quit();
}

#[test]
fn recette_crypto() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = the_tools("39_recette_crypto", profile.path());

    step("CRY-03 HMAC in hex, then in base64", || {
        open_tool(&mut app, "hash");
        retype(&mut app, "hash-input", "DevNotes");
        app.click("hash-hmac");
        retype(&mut app, "hash-key", "secret");
        reads(
            &mut app,
            &["4b86bba0e71ff3bea4180fc0ee8abc4d7f5939927d5f1a1acee89b837e47ec7a"],
        );
        app.click("hash-encoding-1");
        reads(&mut app, &["S4a7oOcf876kGA/A7oq8TX9ZOZJ9Xxoazuibg35H7Ho="]);
        app.capture("CRY-03");
    });

    step("CRY-08 passwords drawn, and drawn again", || {
        open_tool(&mut app, "password");
        app.wait_for("password-strength");
        app.capture("CRY-08");
        let first = app.tree().texts("output-value");
        app.click("password-generate");
        app.eventually("others are drawn", |tree| {
            tree.texts("output-value") != first
        });
    });

    step("CRY-11 identifiers of each kind", || {
        open_tool(&mut app, "identifiers");
        app.wait_for("identifiers-row");
        app.capture("CRY-11");
    });

    step("CRY-12 identifiers read back", || {
        for (id, words) in [
            ("01ARZ3NDEKTSV4RRFFQ69G5FAV", &["ULID", "2016"][..]),
            (
                "550e8400-e29b-41d4-a716-446655440000",
                &["UUID", "RFC 9562"],
            ),
            ("507f1f77bcf86cd799439011", &["ObjectId", "2012"]),
            ("00000000-0000-0000-0000-000000000000", &["UUID nul"]),
            ("bonjour", &["Aucun identifiant connu"]),
        ] {
            retype(&mut app, "identifiers-checked", id);
            reads(&mut app, words);
        }
        app.capture("CRY-12");
    });

    step("CRY-13 a token read and checked", || {
        open_tool(&mut app, "jwt");
        retype(&mut app, "jwt-token", JWT);
        reads(&mut app, &["HS256", "recette", "Recette DevNotes"]);
        app.click("jwt-secret");
        app.type_text("secret");
        reads(&mut app, &["Signature valide."]);
        app.capture("CRY-13");
    });

    app.quit();
}

#[test]
fn recette_encoders() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = the_tools("39_recette_encoders", profile.path());

    step("ENC-04 a URL taken apart", || {
        open_tool(&mut app, "url-codec");
        app.click("url-tab-1");
        retype(
            &mut app,
            "url-parser-input",
            "https://ada:pwd@api.exemple.fr:8443/v1/factures?statut=payée&page=2#total",
        );
        reads(
            &mut app,
            &[
                "https",
                "ada",
                "pwd",
                "api.exemple.fr",
                "8443",
                "/v1/factures",
                "payée",
                "total",
            ],
        );
        app.capture("ENC-04");
    });

    step("ENC-08 a format recognised, ENC-11 a picture shown", || {
        open_tool(&mut app, "base64");
        app.click("base64-direction-1");
        for (written, read) in [
            ("Q2Fmw6k=", "Café"),
            ("43 61 66 c3 a9", "Café"),
            ("01000011 01100001", "Ca"),
            ("67 97 102", "Caf"),
        ] {
            retype(&mut app, "base64-input", written);
            reads(&mut app, &[read]);
        }
        app.capture("ENC-08");
        retype(&mut app, "base64-input", PIXEL);
        app.wait_for("base64-image-facts");
        app.capture("ENC-11");
    });

    step(
        "ENC-19 what is no colour, ENC-20 a contrast put right",
        || {
            open_tool(&mut app, "colour");
            retype(&mut app, "colour-input", "pas-une-couleur");
            app.wait_for("colour-invalid");
            retype(&mut app, "colour-input", "#777777");
            retype(&mut app, "colour-against", "#ffffff");
            reads(&mut app, &["4,48"]);
            app.click("colour-fix");
            reads(&mut app, &["4,5"]);
            app.capture("ENC-20");
        },
    );

    app.quit();
}

#[test]
fn recette_the_rest() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = the_tools("39_recette_rest", profile.path());

    step("GEN-01 documents drawn from a schema", || {
        open_tool(&mut app, "json-generator");
        retype(&mut app, "json-generator-input", SCHEMA);
        retype(&mut app, "json-generator-count", "3");
        app.wait_for("json-generator-seed-used");
        app.capture("GEN-01");
    });

    step("GEN-16 lorem ipsum, opening on its words", || {
        open_tool(&mut app, "lorem");
        reads(&mut app, &["Lorem ipsum dolor sit amet"]);
        let first = app.tree().text("lorem-output");
        app.click("lorem-again");
        app.eventually("another text", |tree| tree.text("lorem-output") != first);
    });

    step("TPS-07 a gap, TPS-10 a duration", || {
        open_tool(&mut app, "dates");
        app.click("dates-tab-1");
        retype(&mut app, "dates-from", "2026-01-01 00:00");
        retype(&mut app, "dates-to", "2026-03-01 12:00");
        reads(&mut app, &["59 j 12 h 00", "8 sem 3 j", "1 428 h 00"]);
        app.capture("TPS-07");
        retype(&mut app, "dates-duration", "PT1H30M");
        reads(&mut app, &["5 400"]);
        retype(&mut app, "dates-duration", "1h30");
        reads(&mut app, &["PT1H30M"]);
        retype(&mut app, "dates-duration", "P1M");
        reads(&mut app, &["Un mois ou une année n’a pas de durée fixe"]);
        app.capture("TPS-10");
    });

    step("CMP-05 a transfer timed", || {
        open_tool(&mut app, "sizes");
        app.click("sizes-tab-1");
        retype(&mut app, "transfer-size", "4,7");
        retype(&mut app, "transfer-rate", "100");
        reads(&mut app, &["6 min 16 s"]);
        app.capture("CMP-05");
    });

    step("REF-01 a status found by code, name and words", || {
        open_tool(&mut app, "http-status");
        for (searched, read) in [
            ("418", "teapot"),
            ("teapot", "teapot"),
            ("trop de requêtes", "Too Many Requests"),
        ] {
            retype(&mut app, "reference-search", searched);
            reads(&mut app, &[read]);
        }
        app.capture("REF-01");
    });

    app.quit();
}

#[test]
fn recette_the_home() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = the_tools("39_recette_home", profile.path());

    step("OUT-01 the home says what it holds", || {
        reads(&mut app, &["32 utilitaires, tous hors ligne"]);
        app.capture("OUT-01");
    });

    step(
        "OUT-02 a category found by its name, without its accents",
        || {
            app.fill("tools-search", "generateur");
            for tool in ["json-generator", "identity", "lorem", "qr"] {
                app.wait_for(&format!("tools-entry-{tool}"));
            }
        },
    );

    step("OUT-03 a reference found, opened by Enter", || {
        app.press(Key::A, Modifiers::COMMAND);
        app.type_text("418");
        app.wait_for("tools-reference-hit");
        reads(&mut app, &["teapot"]);
        app.capture("OUT-03");
        app.press(Key::Enter, Modifiers::NONE);
        app.wait_for("reference-search");
        app.click("tool-back");
    });

    step("OUT-09 an example, then cleared", || {
        open_tool(&mut app, "case");
        app.click("tool-sample");
        app.eventually("the field is filled", |tree| {
            tree.find("case-input")
                .and_then(egui::accesskit::Node::value)
                .is_some_and(|it| !it.is_empty())
        });
        app.click("tool-clear");
        app.eventually("the field is empty", |tree| {
            tree.find("case-input")
                .and_then(egui::accesskit::Node::value)
                .is_none_or(str::is_empty)
        });
    });

    step("OUT-10 what a tool worked out is kept as a note", || {
        open_tool(&mut app, "hash");
        retype(&mut app, "hash-input", "DevNotes");
        app.click("tool-save-as-note");
        app.wait_for("save-as-note-what");
        app.capture("OUT-10-dialog");
        app.click("save-as-note-submit");
        app.wait_for("status-toast");
        app.capture("OUT-10-saved");
    });

    step("OUT-07 three favourites over a relaunch", || {
        for tool in ["case", "hash", "cron"] {
            open_tool(&mut app, tool);
            app.click("tool-favourite");
        }
        app.click("tool-back");
        app.wait_for("tools-favourite-cron");
    });

    let mut app = app.relaunch();
    app.fill("vault-passphrase", PASSPHRASE);
    app.click("vault-submit");
    app.wait_for("area-tools");

    step("OUT-07 they are still there, and go one by one", || {
        for tool in ["case", "hash", "cron"] {
            app.wait_for(&format!("tools-favourite-{tool}"));
        }
        app.capture("OUT-07");
    });

    app.quit();
}
