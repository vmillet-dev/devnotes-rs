//! The screens of the Angular reference (plan § 5.2) drawn by the egui window: its data, its
//! gestures, its file names and its window sizes, for a look side by side with the captures of
//! the `angular-reference` pre-release. Nothing here compares pixels, and nothing runs unless
//! `DEVNOTES_SCREENS` names where to write.

use std::path::{Path, PathBuf};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use engine::folders::model::{FolderColour, FolderDraft};
use engine::http::model::{HttpMethod, HttpRequestDraft, RequestDocument, RequestKind};
use engine::notes::checklist::ChecklistItem;
use engine::notes::kind::NoteKind;
use engine::notes::language::Language;
use engine::notes::model::{NoteDraft, NoteLifecycle, NotePatch};
use engine::notes::priority::Priority;
use engine::spaces::model::SpaceDraft;

use super::DevNotesApp;
use super::tests::startup;
use crate::action::{Action, AppAction};
use crate::services::preferences::Preferences;
use crate::state::Area;
use crate::testing::PASSPHRASE;
use crate::theme::Appearance;

type Window = Harness<'static, DevNotesApp>;

/// What the reference's 1 100 × 720 window left to its page.
const DEFAULT_SIZE: (f32, f32) = (1084.0, 681.0);
const LARGE_SIZE: (f32, f32) = (1904.0, 1041.0);
const SMALL_SIZE: (f32, f32) = (624.0, 441.0);

const SNIPPET: &str = "Relancer le service de paiement";
const NOTE: &str = "Procédure de mise en production";
const CHECKLIST: &str = "Avant la démonstration";
const CALC: &str = "Budget du déplacement";
const DRAWING: &str = "Schéma du réseau";
const TRASHED: &str = "Ancienne procédure";

const NOTE_BODY: &str = "# Mise en production\n\nTrois étapes, **dans cet ordre** :\n\n\
1. Geler la branche `dev`\n2. Lancer la [chaîne de publication](https://exemple.fr/ci)\n\
3. Prévenir l’astreinte\n\n| Étape | Durée |\n| --- | --- |\n| Build | 12 min |\n\
| Tests | 25 min |\n\n```mermaid\nflowchart LR\n  build --> tests --> publication\n```\n\n\
> Ne jamais publier un vendredi.";

fn output() -> Option<PathBuf> {
    std::env::var_os("DEVNOTES_SCREENS").map(PathBuf::from)
}

fn preferences(compact: bool) -> Preferences {
    let mut preferences = Preferences::default();
    preferences.set("devnotes.locale", "en");
    if compact {
        preferences.set("devnotes.density", "compact");
    }
    preferences
}

fn launch(profile: &Path, size: (f32, f32), compact: bool) -> Window {
    text::highlight::warm();
    let mut startup = startup(profile);
    startup.preferences = preferences(compact);
    let mut window = Harness::builder()
        .with_size(egui::vec2(size.0, size.1))
        .build_eframe(|cc| DevNotesApp::new(cc, startup).unwrap());
    // The window finds what the embedded fonts lack in the system's, which a bench has not.
    window
        .ctx
        .set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
    settle(&mut window);
    window
}

fn settle(window: &mut Window) {
    super::tests::settle(window);
}

/// Past a debounce: the bench's clock moves as frames do.
fn wait(window: &mut Window) {
    for _ in 0..30 {
        window.step();
        settle(window);
    }
}

fn click(window: &mut Window, author_id: &str) {
    window
        .get_by(|node| node.author_id() == Some(author_id))
        .click();
    settle(window);
}

fn click_named(window: &mut Window, author_id: &str, name: &str) {
    window
        .get_by(|node| node.author_id() == Some(author_id) && node.label().as_deref() == Some(name))
        .click();
    settle(window);
}

fn type_into(window: &mut Window, author_id: &str, words: &str) {
    click(window, author_id);
    window
        .get_by(|node| node.author_id() == Some(author_id))
        .type_text(words);
    settle(window);
}

fn act(window: &mut Window, action: Action) {
    let ctx = window.ctx.clone();
    let app = window.state_mut();
    app.state.apply(action, &ctx, &app.backend);
    settle(window);
}

fn area(window: &mut Window, area: Area) {
    act(window, Action::App(AppAction::ShowArea(area)));
}

/// By the card's head: its middle may be a task, which a click would tick.
fn card(window: &mut Window, title: &str) {
    let rect = window
        .get_by(|node| {
            node.author_id() == Some("note-card-open") && node.label().as_deref() == Some(title)
        })
        .rect();
    let pos = rect.min + egui::vec2(60.0, 14.0);
    window.event(egui::Event::PointerMoved(pos));
    window.run_steps(1);
    for pressed in [true, false] {
        window.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        window.run_steps(1);
    }
    settle(window);
}

fn hover_card(window: &mut Window, title: &str) {
    window
        .get_by(|node| {
            node.author_id() == Some("note-card-open") && node.label().as_deref() == Some(title)
        })
        .hover();
    settle(window);
}

/// The check box of the card titled `title`, drawn under the pointer.
fn tick(window: &mut Window, title: &str) {
    hover_card(window, title);
    let open = window
        .get_by(|node| {
            node.author_id() == Some("note-card-open") && node.label().as_deref() == Some(title)
        })
        .rect();
    window
        .get_all_by(|node| node.author_id() == Some("note-card-check"))
        .find(|node| open.contains(node.rect().center()))
        .expect("the card draws a check box")
        .click();
    settle(window);
}

/// What the first launch seeds, and the data of `screens.e2e.ts` beside it.
#[expect(clippy::too_many_lines, reason = "the reference's seed, in its order")]
fn seed(window: &Window) {
    let backend = window.state().backend();
    let scene = String::new();
    engine::db::run_here(backend.engine(), move |engine| {
        let home = engine::spaces::list_spaces(engine)?.remove(0).id;
        let work = engine::spaces::create_space(
            engine,
            SpaceDraft {
                name: "Projets".to_string(),
            },
        )?;
        let ops = engine::folders::create_folder(
            engine,
            FolderDraft {
                space_id: work.id.clone(),
                name: "Exploitation".to_string(),
                parent_id: None,
            },
        )?;
        let releases = engine::folders::create_folder(
            engine,
            FolderDraft {
                space_id: work.id.clone(),
                name: "Livraisons".to_string(),
                parent_id: Some(ops.id.clone()),
            },
        )?;
        engine::folders::recolour_folder(engine, releases.id.clone(), FolderColour::Amber)?;

        let draft = |space: &str, title: &str, kind, content: &str| NoteDraft {
            space_id: space.to_string(),
            folder_id: None,
            title: title.to_string(),
            language: Language::default(),
            content: content.to_string(),
            source: String::new(),
            tags: Vec::new(),
            pinned: false,
            lifecycle: NoteLifecycle::Permanent,
            kind,
            priority: Priority::None,
            items: Vec::new(),
        };
        let snippet = engine::notes::create_note(
            engine,
            NoteDraft {
                language: Language::Sh,
                source: "Exploitation / Paiements".to_string(),
                tags: vec!["ops".to_string(), "kubernetes".to_string()],
                pinned: true,
                ..draft(
                    &home,
                    SNIPPET,
                    NoteKind::Snippet,
                    "kubectl rollout restart deployment/{{service}} -n {{namespace}}",
                )
            },
        )?;
        engine::notes::create_note(
            engine,
            NoteDraft {
                tags: vec!["livraison".to_string()],
                ..draft(&home, NOTE, NoteKind::Note, NOTE_BODY)
            },
        )?;
        let item = |text: &str, done| ChecklistItem {
            text: text.to_string(),
            done,
        };
        engine::notes::create_note(
            engine,
            NoteDraft {
                items: vec![
                    item("Vider le cache du navigateur", true),
                    item("Préparer le jeu de données", false),
                    item("Couper les notifications", false),
                ],
                priority: Priority::High,
                ..draft(&home, CHECKLIST, NoteKind::Checklist, "")
            },
        )?;
        engine::notes::create_note(
            engine,
            draft(
                &home,
                CALC,
                NoteKind::Calc,
                "train = 2 × 89 €\nhôtel = 3 × 112 €\nrepas = 6 × 18 €\ntotal = train + hôtel + repas",
            ),
        )?;
        engine::notes::create_note(engine, draft(&home, DRAWING, NoteKind::Drawing, &scene))?;
        engine::notes::create_note(
            engine,
            NoteDraft {
                folder_id: Some(ops.id.clone()),
                language: Language::Sql,
                ..draft(&work.id, "Rotation des clés", NoteKind::Snippet, "SELECT 1;")
            },
        )?;
        engine::notes::create_note(
            engine,
            NoteDraft {
                folder_id: Some(releases.id.clone()),
                ..draft(
                    &work.id,
                    "Notes de version 2.4",
                    NoteKind::Note,
                    "## 2.4\n\n- Export CSV",
                )
            },
        )?;
        let trashed = engine::notes::create_note(
            engine,
            draft(&home, TRASHED, NoteKind::Snippet, "obsolète"),
        )?;
        engine::notes::update_note(
            engine,
            snippet.id.clone(),
            NotePatch {
                content: Some(format!(
                    "{}\nkubectl get pods -n {{{{namespace}}}}",
                    snippet.content
                )),
                ..NotePatch::default()
            },
        )?;
        engine::notes::delete_note(engine, trashed.id.clone())?;

        let api = engine::http::create_http_collection(engine, "API Facturation".to_string())?;
        let auth = engine::http::create_http_folder(
            engine,
            api.id.clone(),
            None,
            "Authentification".to_string(),
        )?;
        let request = |folder_id: Option<String>, name: &str, method| HttpRequestDraft {
            collection_id: api.id.clone(),
            folder_id,
            name: name.to_string(),
            kind: RequestKind::Http,
            method,
            document: RequestDocument {
                url: format!(
                    "https://api.exemple.fr/{}",
                    name.to_lowercase().replace(' ', "-")
                ),
                ..RequestDocument::default()
            },
        };
        engine::http::create_http_request(
            engine,
            request(Some(auth.id.clone()), "Connexion", HttpMethod::Post),
        )?;
        engine::http::create_http_request(
            engine,
            request(None, "Lister les factures", HttpMethod::Get),
        )?;
        engine::http::create_http_environment(engine, "Recette".to_string())?;
        Ok::<_, engine::error::AppError>(())
    })
    .unwrap();
}

/// A profile holding the samples and the seed, its library closed as a process leaves it.
fn seeded() -> tempfile::TempDir {
    let profile = tempfile::tempdir().unwrap();
    super::captures::copy_identity_lists(profile.path());
    let mut window = launch(profile.path(), DEFAULT_SIZE, false);
    type_into(&mut window, "vault-passphrase", PASSPHRASE);
    type_into(&mut window, "vault-confirmation", PASSPHRASE);
    click(&mut window, "vault-submit");
    seed(&window);
    crate::testing::close_library(window.state().backend());
    profile
}

fn unlocked(profile: &Path, size: (f32, f32), compact: bool) -> Window {
    let mut window = launch(profile, size, compact);
    type_into(&mut window, "vault-passphrase", PASSPHRASE);
    click(&mut window, "vault-submit");
    settle(&mut window);
    window
}

fn draw(window: &mut Window, directory: &Path, name: &str, appearance: Appearance) {
    act(window, Action::App(AppAction::SetAppearance(appearance)));
    // kittest draws its pointer, and where a click left it.
    window.event(egui::Event::PointerGone);
    settle(window);
    let image = window
        .render()
        .expect("a software adapter draws the window");
    image.save(directory.join(format!("{name}.png"))).unwrap();
}

type Reach = fn(&mut Window);

/// A profile of its own for a screen: what one reaches is remembered, the rail put away, a
/// board shown.
fn copy_of(profile: &Path) -> tempfile::TempDir {
    let copy = tempfile::tempdir().unwrap();
    copy_directory(profile, copy.path());
    copy
}

fn copy_directory(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_directory(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Both themes, as the reference drew each screen.
fn both_themes(profile: &Path, directory: &Path, screens: &[(&str, Reach)]) {
    for (id, reach) in screens {
        let profile = copy_of(profile);
        let mut window = unlocked(profile.path(), DEFAULT_SIZE, false);
        reach(&mut window);
        draw(
            &mut window,
            directory,
            &format!("{id}--dark"),
            Appearance::Dark,
        );
        draw(
            &mut window,
            directory,
            &format!("{id}--light"),
            Appearance::Light,
        );
        crate::testing::close_library(window.state().backend());
    }
}

fn with_output(run: impl FnOnce(&Path, &Path)) {
    let Some(directory) = output() else {
        return;
    };
    std::fs::create_dir_all(&directory).unwrap();
    let profile = seeded();
    run(profile.path(), &directory);
}

fn board(window: &mut Window) {
    click_named(window, "space-option", "Projets");
    click(window, "view-board");
}

fn search(window: &mut Window, words: &str) {
    type_into(window, "search-input", words);
    wait(window);
}

fn nothing(_: &mut Window) {}

const NOTES: [(&str, Reach); 14] = [
    ("notes-list", nothing),
    ("notes-board", board),
    ("notes-search", |window| search(window, "production")),
    ("notes-search-empty", |window| {
        search(window, "zzzz introuvable");
    }),
    ("notes-facets", |window| click(window, "facets-toggle")),
    ("notes-sort-menu", |window| click(window, "sort-trigger")),
    ("notes-new-kind-menu", |window| {
        click(window, "new-note-kind");
    }),
    ("notes-space-switcher", |window| {
        click(window, "library-rail-toggle");
        click(window, "space-switcher");
    }),
    ("notes-folder-switcher", |window| {
        click(window, "library-rail-toggle");
        click(window, "folder-switcher");
    }),
    ("notes-card-menu", |window| {
        hover_card(window, SNIPPET);
        click(window, "note-card-menu");
    }),
    ("notes-selection", |window| {
        tick(window, SNIPPET);
        tick(window, NOTE);
    }),
    ("notes-fields-form", |window| {
        hover_card(window, SNIPPET);
        click(window, "note-card-fill");
    }),
    ("notes-tag-manager", |window| {
        click(window, "facets-toggle");
        click(window, "tag-manage");
    }),
    ("notes-trash", |window| click(window, "trash-open")),
];

const EDITORS: [(&str, Reach); 9] = [
    ("notes-palette", |window| {
        window.key_press_modifiers(
            egui::Modifiers::COMMAND | egui::Modifiers::ALT,
            egui::Key::P,
        );
        settle(window);
    }),
    ("editor-snippet", |window| card(window, SNIPPET)),
    ("editor-snippet-fields", |window| {
        card(window, SNIPPET);
        click(window, "placeholder-panel-toggle");
    }),
    ("editor-snippet-revisions", |window| {
        card(window, SNIPPET);
        click(window, "revisions-toggle");
    }),
    ("editor-note", |window| card(window, NOTE)),
    ("editor-note-presentation", |window| {
        card(window, NOTE);
        click(window, "editor-present");
    }),
    ("editor-checklist", |window| card(window, CHECKLIST)),
    ("editor-calc", |window| card(window, CALC)),
    ("editor-drawing", |window| card(window, DRAWING)),
];

fn preferences_page(window: &mut Window, page: &str) {
    click(window, "file-menu");
    click(window, "file-preferences");
    click(window, page);
}

const MENUS: [(&str, Reach); 10] = [
    ("file-menu", |window| click(window, "file-menu")),
    ("about-menu", |window| click(window, "about-open")),
    ("libraries", |window| {
        click(window, "file-menu");
        click(window, "file-libraries");
    }),
    ("settings-general", |window| {
        preferences_page(window, "settings-page-general");
    }),
    ("settings-shortcuts", |window| {
        preferences_page(window, "settings-page-shortcuts");
    }),
    ("settings-security", |window| {
        preferences_page(window, "settings-page-security");
    }),
    ("settings-variables", |window| {
        preferences_page(window, "settings-page-variables");
    }),
    ("settings-change-passphrase", |window| {
        preferences_page(window, "settings-page-security");
        click(window, "setting-change-passphrase");
    }),
    ("getting-started", |window| {
        click(window, "about-open");
        click(window, "about-getting-started");
    }),
    ("tools-home", |window| area(window, Area::Tools)),
];

fn http_request(window: &mut Window) {
    area(window, Area::Http);
    click_named(window, "http-node-name", "GET Lister les factures");
}

fn new_tab(window: &mut Window, kind: &str) {
    area(window, Area::Http);
    click(window, "http-tab-new-kind");
    click(window, &format!("http-new-kind-{kind}"));
}

const HTTP: [(&str, Reach); 10] = [
    ("tools-no-match", |window| {
        area(window, Area::Tools);
        type_into(window, "tools-search", "zzzz");
        wait(window);
    }),
    ("http-home", |window| area(window, Area::Http)),
    ("http-request", http_request),
    ("http-folder-menu", |window| {
        area(window, Area::Http);
        window
            .get_all_by(|node| node.author_id() == Some("http-node-name"))
            .next()
            .expect("the rail lists the collection")
            .hover();
        settle(window);
        window
            .get_all_by(|node| node.author_id() == Some("http-node-menu"))
            .next()
            .expect("a hovered row offers its menu")
            .click();
        settle(window);
    }),
    ("http-graphql", |window| new_tab(window, "graphql")),
    ("http-websocket", |window| new_tab(window, "websocket")),
    ("http-environments", |window| {
        area(window, Area::Http);
        click(window, "http-environments-open");
    }),
    ("http-cookies", |window| {
        area(window, Area::Http);
        click(window, "http-cookies-open");
    }),
    ("http-history", |window| {
        area(window, Area::Http);
        click(window, "http-history-open");
    }),
    ("http-code", |window| {
        http_request(window);
        click(window, "http-code-open");
    }),
];

#[test]
fn the_notes() {
    with_output(|profile, directory| both_themes(profile, directory, &NOTES));
}

#[test]
fn the_editors() {
    with_output(|profile, directory| both_themes(profile, directory, &EDITORS));
}

#[test]
fn the_menus() {
    with_output(|profile, directory| both_themes(profile, directory, &MENUS));
}

#[test]
fn the_http_area() {
    with_output(|profile, directory| both_themes(profile, directory, &HTTP));
}

/// Dark only: the tools share one frame, their bodies are what differs.
#[test]
fn the_tools() {
    with_output(|profile, directory| {
        let mut window = unlocked(profile, DEFAULT_SIZE, false);
        area(&mut window, Area::Tools);
        for tool in &crate::tools::catalogue::TOOLS {
            act(
                &mut window,
                Action::Tools(crate::tools::Action::Open(tool.id)),
            );
            if window
                .query_by(|node| node.author_id() == Some("tool-sample"))
                .is_some()
            {
                click(&mut window, "tool-sample");
            }
            wait(&mut window);
            let name = format!("tool-{}--dark", tool.id);
            draw(&mut window, directory, &name, Appearance::Dark);
        }
    });
}

/// The screens whose layout answers the window, dark: at the two other sizes, then compact.
#[test]
fn the_sizes_and_the_density() {
    let reaches: [(&str, Reach); 6] = [
        ("notes-list", nothing),
        ("notes-board", board),
        ("editor-note", |window| {
            search(window, "Procédure");
            card(window, NOTE);
        }),
        ("tools-home", |window| area(window, Area::Tools)),
        ("http-request", http_request),
        ("editor-snippet", |window| card(window, SNIPPET)),
    ];
    with_output(|profile, directory| {
        for (id, reach) in &reaches[..5] {
            for (size, label) in [(LARGE_SIZE, "1920x1080"), (SMALL_SIZE, "640x480")] {
                let profile = copy_of(profile);
                let mut window = unlocked(profile.path(), size, false);
                reach(&mut window);
                let name = format!("{id}--dark--{label}");
                draw(&mut window, directory, &name, Appearance::Dark);
                crate::testing::close_library(window.state().backend());
            }
        }
        let compact = reaches
            .iter()
            .copied()
            .filter(|(id, _)| *id != "editor-note")
            .chain([(
                "editor-checklist",
                (|window| card(window, CHECKLIST)) as Reach,
            )]);
        for (id, reach) in compact {
            let profile = copy_of(profile);
            let mut window = unlocked(profile.path(), DEFAULT_SIZE, true);
            reach(&mut window);
            let name = format!("{id}--dark--compact");
            draw(&mut window, directory, &name, Appearance::Dark);
            crate::testing::close_library(window.state().backend());
        }
    });
}
