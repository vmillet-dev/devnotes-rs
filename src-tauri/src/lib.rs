// A module holding commands is `pub` — `#[specta::specta]` resolves its generated macro
// from the crate root, and `tests/` is a separate crate. Everything else is `pub(crate)`,
// which is what gives `unreachable_pub` and `dead_code` something to say.
pub mod attachments;
pub mod backup;
pub mod changelog;
pub mod db;
pub mod desktop;
pub mod error;
pub mod folders;
pub mod libraries;
pub mod notes;
pub mod recovery;
pub mod spaces;
pub mod transfer;
pub mod vault;

pub(crate) mod app_info;
pub(crate) mod closed_enum;
pub(crate) mod count;
pub(crate) mod layout;
pub(crate) mod name;

use tauri::Manager;
use tauri_plugin_window_state::StateFlags;
use tauri_specta::{Builder, collect_commands};

use attachments::{
    attach_clipboard_image, attach_file, delete_attachment, list_attachments, open_attachment,
    read_attachment, save_attachment,
};
use backup::{list_backups, restore_backup};
use changelog::app_changelog;
use desktop::{set_global_shortcuts, set_window_behavior, sync_tray};
use folders::{
    arrange_board, board_view, create_folder, delete_folder, file_notes, file_notes_back,
    list_folders, recolour_folder, rename_folder, save_board_layout,
};
use libraries::{create_library, delete_library, list_libraries, open_library, rename_library};
use notes::{
    count_notes_tagged, create_note, delete_note, delete_notes, delete_tags, empty_trash,
    fill_placeholders, list_global_placeholders, list_revisions, list_tags, list_trash, move_notes,
    move_notes_back, purge_notes, query_notes, rename_tags, restore_notes, restore_revision,
    revision_diff, seed_samples, set_global_placeholders, set_placeholder_values, tag_notes,
    untag_notes, update_note,
};
use recovery::{archive_locked_library, set_aside_damaged_library};
use spaces::{create_space, delete_space, list_spaces, pin_space, rename_space};
use transfer::{export_is_protected, export_notes, import_notes, share_notes};
use vault::{change_passphrase, create_vault, unlock_vault, vault_state};

/// ⚠️ Resolved from the manifest: a relative path writes the file next to whatever the
/// current directory happens to be, without saying a word.
const BINDINGS_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../src/app/core/ipc/bindings.ts"
);

/// ⚠️ Not a `#[cfg(test)]`: on Windows the test executable lives in
/// `target/debug/deps/`, without the `WebView2Loader.dll` that linking `export`
/// then requires — the whole test binary stops starting.
pub fn export_bindings() -> Result<(), specta_typescript::Error> {
    ipc_builder().export(specta_typescript::Typescript::default(), BINDINGS_PATH)
}

/// The single source of the signatures: it registers with Tauri *and* writes
/// `bindings.ts`. A command absent from it exists nowhere.
fn ipc_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            query_notes,
            create_note,
            seed_samples,
            update_note,
            list_revisions,
            revision_diff,
            restore_revision,
            delete_note,
            delete_notes,
            restore_notes,
            list_trash,
            purge_notes,
            empty_trash,
            move_notes,
            move_notes_back,
            tag_notes,
            untag_notes,
            count_notes_tagged,
            list_tags,
            rename_tags,
            delete_tags,
            fill_placeholders,
            set_placeholder_values,
            list_global_placeholders,
            set_global_placeholders,
            list_folders,
            board_view,
            save_board_layout,
            arrange_board,
            create_folder,
            rename_folder,
            recolour_folder,
            delete_folder,
            file_notes,
            file_notes_back,
            list_spaces,
            create_space,
            rename_space,
            pin_space,
            delete_space,
            attach_file,
            attach_clipboard_image,
            list_attachments,
            read_attachment,
            open_attachment,
            save_attachment,
            delete_attachment,
            export_notes,
            import_notes,
            share_notes,
            export_is_protected,
            vault_state,
            create_vault,
            unlock_vault,
            change_passphrase,
            set_aside_damaged_library,
            archive_locked_library,
            list_backups,
            restore_backup,
            list_libraries,
            create_library,
            open_library,
            rename_library,
            delete_library,
            app_changelog,
            sync_tray,
            set_global_shortcuts,
            set_window_behavior,
        ])
        // Reachable from no command, so exported on its own — with the topic it travels
        // on, which neither side then spells twice.
        .typ::<desktop::GlobalAction>()
        .constant("GLOBAL_ACTION_EVENT", desktop::ACTION_EVENT)
        // From `Cargo.toml`, so the front keeps no second copy of the name.
        .constant("APP_METADATA", app_info::METADATA)
        // ⚠️ The native side still registers these before the front end exists — that is
        // what a default is for here — but the front reads them rather than retyping them.
        .constant("DEFAULT_SHORTCUTS", desktop::ShortcutBindings::defaults())
        // The `{{field}}` rule, so the form refuses in the same terms the back end does.
        .constant("FIELD_NAME_PATTERN", notes::placeholder::FIELD_NAME_PATTERN)
        // ⚠️ Rust decides where a library lives, so it names the preference files too — a
        // second spelling on the front would open a second file.
        .constant("PREFERENCES_FILE", layout::PREFERENCES)
        // Read by Rust out of the application's file before the front end has booted.
        .constant("AUTOMATIC_BACKUPS_KEY", backup::AUTOMATIC_BACKUPS_KEY)
}

/// ⚠️ Not the plugin's default `all()`, which carries `VISIBLE`: quitting from the tray
/// saves a hidden window, and the next launch would restore it hidden — an application
/// that starts with nothing on screen. `DECORATIONS` and `FULLSCREEN` never change here,
/// so saving them would only store noise.
const WINDOW_STATE_FLAGS: StateFlags = StateFlags::SIZE
    .union(StateFlags::POSITION)
    .union(StateFlags::MAXIMIZED);

/// ⚠️ Order matters: `single_instance` before every other plugin, and `log` before the
/// plugins that already log during their own initialisation.
fn with_plugins(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    #[allow(unused_mut)]
    let mut builder = builder
        // A second launch would otherwise open a second process on the same SQLite file,
        // and silently lose every global shortcut to the instance already holding them.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            desktop::reveal(app);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir { file_name: None },
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        // ⚠️ The only surface the palette has left once it has hidden the window. Never
        // load-bearing: the operating system may refuse it, and the copy still happened.
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(WINDOW_STATE_FLAGS)
                .build(),
        );

    // ⚠️ The end-to-end harness. `tauri_plugin_log` has already taken the global logger,
    // so WDIO captures no backend log — read the log plugin's targets instead.
    #[cfg(feature = "e2e")]
    {
        builder = builder
            .plugin(tauri_plugin_wdio::init())
            .plugin(tauri_plugin_wdio_webdriver::init());
    }

    builder
}

/// The plugins that need a handle rather than a builder, the native state, and the
/// database — in that order, because everything after the connection assumes it.
fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // `tauri.conf.json` carries the crate's lowercase product name; the window wears the
    // name the user is shown everywhere else.
    //
    // ⚠️ The window is declared `"visible": false` and shown here, because
    // `tauri-plugin-window-state` restores the geometry from `on_webview_ready`, which
    // has already run by the time `setup` does — created visible, it shows the config
    // size and then jumps. Here rather than from the front end, so a front end that
    // fails to boot does not leave a process with no window at all.
    if let Some(window) = app.get_webview_window("main") {
        window.set_title(app_info::METADATA.name)?;
        window.show()?;
    }

    app.handle()
        .plugin(tauri_plugin_updater::Builder::new().build())?;

    // ⚠️ `MacosLauncher` is not macOS code that slipped in: the plugin takes it on every
    // platform and ignores it off macOS, so deleting it would not compile.
    app.handle().plugin(tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        None,
    ))?;

    // The tray waits for its translated labels to arrive from the front end.
    desktop::init(app.handle())?;

    // ⚠️ Logged and carried on, never propagated: an error out of `setup` reaches
    // `run(…).expect(…)` and panics before the window has anything in it — a full disk
    // used to kill the application with nothing on screen but a line in the log
    // directory. The failure comes back where it can be read instead: `vault_state`,
    // `create_vault` and `unlock_vault` all reach for this directory, and their error
    // names it in the banner over the unlock screen.
    //
    // Carrying on is only safe because the library is already opened behind the
    // passphrase: a command that runs before it answers `StorageError::Locked` rather
    // than reading a database nobody opened.
    match app.path().app_data_dir() {
        Ok(directory) => {
            if let Err(error) = std::fs::create_dir_all(&directory) {
                log::error!(
                    "The data directory {} could not be prepared: {error}",
                    directory.display()
                );
            }
        }
        Err(error) => log::error!("No data directory to store the library in: {error}"),
    }

    // ⚠️ Nothing is opened here any more: the key comes from a passphrase the front end
    // has not asked for yet. `vault::unlock` is what fills this and runs the sweeps.
    app.manage(db::Db::new(None));

    Ok(())
}

/// The launch chores: a rolling copy of the library, then what makes retention hold
/// even if nobody opens the trash. None of them is fatal — the application has to start.
///
/// ⚠️ Moved behind the unlock with the database itself. A sweep needs to read the notes,
/// and before the passphrase there is nothing to read.
pub(crate) fn sweep(handle: &tauri::AppHandle) {
    let db = handle.state::<db::Db>();

    // ⚠️ Before the sweeps, not after: the copy is worth most when it holds what the
    // retention is about to purge.
    backup::take(handle, &db);

    notes::trash::sweep_at_startup(&db);
    if let Err(error) = attachments::sweep_orphan_files(&db) {
        log::warn!("Orphan attachment files not swept: {error}");
    }
    // ⚠️ The decrypted copies `open_attachment` had to write. They cannot be deleted on
    // close — the application that opened one still holds it — so this is the guarantee:
    // gone by the next launch.
    attachments::sealed::sweep_plaintext(handle);
}

/// ⚠️ Both are refused when there is no tray to find the window in (see `desktop`).
fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    match event {
        tauri::WindowEvent::CloseRequested { api, .. }
            if desktop::hides_on_close(window.app_handle()) =>
        {
            api.prevent_close();
            let _ = window.hide();
        }
        // ⚠️ Tauri emits nothing for "minimized": `Resized` is the only way through.
        tauri::WindowEvent::Resized(_)
            if desktop::hides_on_minimize(window.app_handle())
                && window.is_minimized().unwrap_or(false) =>
        {
            let _ = window.hide();
        }
        _ => {}
    }
}

pub fn run() {
    let builder = ipc_builder();

    // Not in release: the front-end `src/` does not exist next to an installed binary.
    // ⚠️ `eprintln!` and not `log::warn!`: this runs before `tauri_plugin_log` has taken
    // the global logger, so a logged line would reach the no-op default and vanish.
    #[cfg(debug_assertions)]
    if let Err(error) = export_bindings() {
        eprintln!("TypeScript bindings not regenerated: {error}");
    }

    with_plugins(tauri::Builder::default())
        .setup(|app| setup(app))
        .on_window_event(on_window_event)
        .invoke_handler(builder.invoke_handler())
        .build(tauri::generate_context!())
        .expect("error while launching the Tauri application")
        .run(|handle, event| {
            // ⚠️ Built and run rather than `run` alone, for this one event: a decrypted
            // copy handed to another application should not outlive the session that
            // asked for it. Best effort by design — one the desktop still holds is
            // locked and stays, and a crash reaches none of this, which is what the
            // sweep at launch is for.
            if matches!(event, tauri::RunEvent::Exit) {
                attachments::sealed::sweep_plaintext(handle);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::{StateFlags, WINDOW_STATE_FLAGS};

    /// The one flag that turns "remembers its geometry" into "does not come back".
    #[test]
    fn the_window_state_never_remembers_that_it_was_hidden() {
        assert!(!WINDOW_STATE_FLAGS.contains(StateFlags::VISIBLE));
        assert!(WINDOW_STATE_FLAGS.contains(StateFlags::SIZE));
        assert!(WINDOW_STATE_FLAGS.contains(StateFlags::POSITION));
    }
}
