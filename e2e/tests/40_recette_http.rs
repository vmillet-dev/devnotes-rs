//! The acceptance sheet's main paths through the HTTP area that no other scenario walks whole, on
//! the local server. A step opens on the case it runs, so that a run can be read back into the
//! sheet.

use e2e::{App, Tree, step};
use egui::accesskit::Node;
use egui::{Key, Modifiers};

const COLLECTION: &str = "Recette API";
const BASIC: &str = "http-auth-kind-2";
const BEARER: &str = "http-auth-kind-3";
const POST: &str = "http-method-1";
const JSON_BODY: &str = "http-body-kind-1";
const BIG: &str = r#"{"nom":"Recette","grand":12345678901234567890,"prix":1.10}"#;

fn values(tree: &Tree, author_id: &str) -> Vec<String> {
    tree.all(author_id)
        .into_iter()
        .map(|node| node.value().unwrap_or_default().to_owned())
        .collect()
}

fn url_of(tree: &Tree) -> String {
    tree.find("http-url")
        .and_then(Node::value)
        .unwrap_or_default()
        .to_owned()
}

fn set_url(app: &mut App, url: &str) {
    app.click("http-url");
    app.press(Key::A, Modifiers::COMMAND);
    app.type_text(url);
}

fn send(app: &mut App, url: &str) {
    set_url(app, url);
    app.press(Key::Enter, Modifiers::COMMAND);
    app.eventually("the answer is shown", |tree| {
        tree.has("http-response-status") && !tree.has("http-response-sending")
    });
}

fn answers(app: &mut App, words: &str) {
    app.eventually(&format!("the answer says {words}"), |tree| {
        tree.text("http-response-pretty")
            .or_else(|| tree.text("http-response-body"))
            .is_some_and(|body| body.contains(words))
    });
}

fn a_draft(app: &mut App) {
    app.click("http-tab-new");
    app.wait_for("http-url");
}

fn http(scenario: &'static str, profile: &std::path::Path) -> App {
    let mut app = App::launch(scenario, profile);
    app.create_library();
    app.press(Key::Num3, Modifiers::COMMAND);
    app.wait_for("http-rail-title");
    app
}

#[test]
fn recette_composing() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = http("40_recette_composing", profile.path());
    app.resize(1100, 1100);

    step("HRQ-01 a new request writes nothing", || {
        a_draft(&mut app);
        set_url(&mut app, "https://exemple.fr/get");
        app.click_nth("http-tab-close", 0);
        app.wait_for("http-close-dialog");
        app.capture("hrq-01");
        app.click("http-close-discard");
        app.wait_for("http-rail-empty");
    });

    step("HRL-01 a first collection", || {
        app.click("http-rail-empty-create");
        app.fill("http-create-input", COLLECTION);
        app.press(Key::Enter, Modifiers::NONE);
        app.eventually("it is listed", |tree| {
            tree.texts("http-node-name") == [COLLECTION]
        });
    });

    step("HRQ-02 saved: a name is asked, and a place", || {
        a_draft(&mut app);
        set_url(&mut app, "https://exemple.fr/facture");
        app.press(Key::S, Modifiers::COMMAND);
        app.wait_for("http-save-dialog");
        app.click("http-save-name");
        app.press(Key::A, Modifiers::COMMAND);
        app.press(Key::Backspace, Modifiers::NONE);
        app.click("http-save-submit");
        app.wait_for("http-save-missing");
        app.capture("hrq-02");
        app.fill("http-save-name", "Lire une facture");
        app.click("http-save-submit");
        app.eventually("it is in the rail", |tree| {
            tree.texts("http-node-name")
                .iter()
                .any(|it| it.ends_with("Lire une facture"))
        });
    });

    step(
        "HRQ-04 a saved request changed asks before it closes",
        || {
            set_url(&mut app, "https://exemple.fr/autre");
            app.capture("hrq-04-dot");
            app.click_nth("http-tab-close", 0);
            app.wait_for("http-close-modified");
            app.click("http-close-cancel");
            app.eventually("the dialog closed", |tree| !tree.has("http-close-dialog"));
        },
    );

    step("HRQ-05 the URL and its table agree", || {
        set_url(&mut app, "https://exemple.fr/get?page=2&q=café");
        app.eventually("the table reads the URL", |tree| {
            values(tree, "http-kv-key").starts_with(&["page".to_owned(), "q".to_owned()])
        });
        app.click_nth("http-kv-enabled", 1);
        app.eventually("q leaves the URL and stays in the table", |tree| {
            !url_of(tree).contains("q=") && values(tree, "http-kv-key")[1] == "q"
        });
        app.click_nth("http-kv-key", 2);
        app.type_text("limit");
        app.click_nth("http-kv-value", 2);
        app.type_text("10");
        app.eventually("limit joins the URL", |tree| {
            url_of(tree).contains("limit=10")
        });
        append_to_url(&mut app, "&x={{page}}");
        app.eventually("a variable stays as written", |tree| {
            url_of(tree).contains("{{page}}")
        });
    });

    step(
        "HRQ-06 headers pasted are as many rows, typed ones split",
        || {
            app.click("http-section-headers");
            app.wait_for("http-implied-headers");
            app.click_nth("http-kv-key", 0);
            app.paste("Accept: application/json\nX-Trace: abc\nX-Tenant: recette");
            app.eventually("three rows", |tree| {
                values(tree, "http-kv-key").starts_with(&[
                    "Accept".to_owned(),
                    "X-Trace".to_owned(),
                    "X-Tenant".to_owned(),
                ])
            });
            let last = app.tree().count("http-kv-key") - 1;
            app.click_nth("http-kv-key", last);
            app.type_text("X-Recette: 1");
            app.eventually("key and value apart", |tree| {
                values(tree, "http-kv-key").contains(&"X-Recette".to_owned())
                    && values(tree, "http-kv-value").contains(&"1".to_owned())
            });
        },
    );

    what_goes_with_it(&mut app);
    app.quit_discarding();
}

fn what_goes_with_it(app: &mut App) {
    step("HRQ-09 what leaves unasked is said", || {
        a_draft(app);
        set_url(app, "https://exemple.fr/post");
        app.click("http-section-body");
        app.click(JSON_BODY);
        app.fill("http-body-text", "{}");
        app.click("http-section-headers");
        app.eventually("the implied headers are said", |tree| {
            let said = tree.texts("http-implied-value").join(" ");
            said.contains("DevNotes/") && said.contains("*/*") && said.contains("application/json")
        });
        app.capture("hrq-09");
    });

    step(
        "HRL-06 a folder deleted says which change goes with it",
        || {
            app.click_where("http-node-menu", |node| {
                node.label().is_some_and(|it| it.ends_with(COLLECTION))
            });
            app.click("http-node-delete");
            app.wait_for("http-delete-unsaved");
            app.capture("hrl-06");
            app.click("http-delete-cancel");
            app.eventually("nothing went", |tree| !tree.has("http-delete-unsaved"));
            assert_eq!(app.tree().count("http-node-name"), 2);
        },
    );
}

fn append_to_url(app: &mut App, words: &str) {
    app.click("http-url");
    app.press(Key::End, Modifiers::NONE);
    app.type_text(words);
}

#[test]
fn recette_sending() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = http("40_recette_sending", profile.path());
    let port = e2e::serve();
    let at = |path: &str| format!("http://127.0.0.1:{port}{path}");
    a_draft(&mut app);

    step("HRQ-10 Basic, its password hidden until asked", || {
        app.click("http-section-auth");
        app.click("http-auth-kind");
        app.click(BASIC);
        app.fill("http-auth-username", "ada");
        app.fill("http-auth-secret", "secret");
        app.capture("hrq-10-hidden");
        send(&mut app, &at("/echo"));
        answers(&mut app, "Basic YWRhOnNlY3JldA==");
    });

    step("HRQ-11 Bearer", || {
        app.click("http-auth-kind");
        app.click(BEARER);
        app.click("http-auth-secret");
        app.press(Key::A, Modifiers::COMMAND);
        app.type_text("jeton-recette");
        send(&mut app, &at("/echo"));
        answers(&mut app, "Bearer jeton-recette");
    });

    step("HHI-02 the history masks both", || {
        app.click("http-history-open");
        app.wait_for("http-history-item");
        app.click("http-history-item");
        app.wait_for("http-history-url");
        app.capture("hhi-02");
        app.eventually("no secret is read", |tree| {
            !tree.mentions("jeton-recette") && !tree.mentions("YWRhOnNlY3JldA==")
        });
        app.click("http-history-reopen");
        app.eventually("back to the request", |tree| !tree.has("http-history"));
    });

    step("HRQ-14 a JSON body leaves as it was written", || {
        app.click("http-method");
        app.click(POST);
        app.click("http-section-body");
        app.click(JSON_BODY);
        app.click("http-body-text");
        app.paste(BIG);
        send(&mut app, &at("/echo"));
        answers(&mut app, BIG);
    });

    step("HRS-02 a send under way is said, and cancelled", || {
        set_url(&mut app, &at("/slow"));
        app.press(Key::Enter, Modifiers::COMMAND);
        app.wait_for("http-response-sending");
        app.capture("hrs-02");
        app.click("http-send");
        app.wait_for("http-response-cancelled");
    });

    step("HRS-04 the views of a body", || {
        app.click("http-method");
        app.click("http-method-0");
        send(&mut app, &at("/json"));
        for view in [
            "http-response-view-0",
            "http-response-view-1",
            "http-response-view-3",
        ] {
            app.click(view);
            app.capture(view);
        }
        app.click("http-response-view-0");
        send(&mut app, &at("/png"));
        app.wait_for("http-response-image");
        send(&mut app, &at("/bytes"));
        app.wait_for("http-response-binary");
        app.capture("hrs-04-binary");
    });

    step("HRS-06 redirects are followed and counted", || {
        send(&mut app, &at("/redirect/3"));
        app.click("http-response-section-timeline");
        app.wait_for("http-response-redirect");
        app.capture("hrs-06");
    });

    app.quit_discarding();
}
