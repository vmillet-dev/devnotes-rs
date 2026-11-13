//! A Calcul note: made from the new-note menu, each line computed as it is typed, an error kept on
//! its line, a result picked and summed in the footer, and the last lines on its card. Results
//! are read by their figures: the language on screen writes the decimal mark and the units.

use e2e::{App, Tree, step};

const TITLE: &str = "Budget sheet";
const SHEET: &str = "# Budget
hébergement = 100,521 €
domaines = 4,99 €
mensuel = total
mensuel × 12
3 h 20 min en min
20 % of 340
100 € en $
en = 5";

fn digits(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect()
}

fn results(tree: &Tree) -> Vec<String> {
    tree.all("calc-result")
        .into_iter()
        .map(|node| digits(node.value().unwrap_or_default()))
        .collect()
}

#[test]
fn calc() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("33_calc", profile.path());
    app.create_library();
    app.wait_for("note-card");

    step("a sheet is made from the new-note menu", || {
        app.click("new-note-kind");
        app.click("new-note-calc");
        app.fill("editor-title", TITLE);
        app.wait_for("calc-counts");
    });

    step(
        "each line is computed, and an error stays on its line",
        || {
            app.click("editor-body");
            app.paste(SHEET);
            app.eventually("the results computed", |tree| {
                let results = results(tree);
                results.contains(&"10551".to_owned()) && results.contains(&"200".to_owned())
            });
            app.eventually("two lines refused, on their lines", |tree| {
                tree.count("calc-failure") == 2
            });
            app.capture("sheet");
        },
    );

    step("a result picked is summed in the footer", || {
        app.click_nth("calc-result", 0);
        app.eventually("the sum said", |tree| {
            tree.text("calc-summary")
                .is_some_and(|said| digits(&said).contains("10052"))
        });
        app.capture("picked");
    });

    step("the card shows the last lines that gave something", || {
        app.click("editor-close");
        app.eventually("the editor is closed", |tree| !tree.has("editor-title"));
        app.capture("card");
    });

    step(
        "a short date before an accent leaves the sheet listed",
        || {
            app.click("new-note-kind");
            app.click("new-note-calc");
            app.fill("editor-title", "Agenda");
            app.click("editor-body");
            app.paste("RDV le 5/11/24 à 10h\n2 + 2");
            app.eventually("the line computed", |tree| {
                results(tree).contains(&"4".to_owned())
            });
            app.click("editor-close");
            app.eventually("the card listed", |tree| {
                tree.texts("note-card-title")
                    .iter()
                    .any(|title| title == "Agenda")
            });
            app.capture("short-date");
        },
    );

    app.quit();
}
