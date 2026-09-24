// A module holding commands is `pub`: `#[specta::specta]` resolves its macro from the crate
// root. The rest is `pub(crate)`, so `unreachable_pub` and `dead_code` can see them.
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
    count_notes_tagged, create_note, delete_note, delete_notes, delete_tags, duplicate_note,
    empty_trash, fill_placeholders, get_note, list_global_placeholders, list_revisions, list_tags,
    list_trash, move_notes, move_notes_back, purge_notes, query_notes, rename_tags, restore_notes,
    restore_revision, revision_diff, seed_samples, set_global_placeholders, set_placeholder_values,
    tag_notes, untag_notes, update_note,
};
use recovery::{archive_locked_library, set_aside_damaged_library};
use spaces::{create_space, delete_space, list_spaces, pin_space, rename_space};
use transfer::{export_is_protected, export_notes, import_notes, share_notes};
use vault::{change_passphrase, create_vault, unlock_vault, vault_state};

/// From the manifest: a relative path would follow the current directory.
const BINDINGS_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../src/app/core/ipc/bindings.ts"
);

/// ⚠️ Not a `#[cfg(test)]`: on Windows the test executable sits in `target/debug/deps/`,
/// without the `WebView2Loader.dll` linking `export` requires, and never starts.
pub fn export_bindings() -> Result<(), specta_typescript::Error> {
    ipc_builder().export(specta_typescript::Typescript::default(), BINDINGS_PATH)
}

/// The single list of commands: it registers them with Tauri and writes `bindings.ts`.
fn ipc_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            query_notes,
            get_note,
            create_note,
            duplicate_note,
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
        // Reachable from no command, so exported on its own, with the topic it travels on.
        .typ::<desktop::GlobalAction>()
        .constant("GLOBAL_ACTION_EVENT", desktop::ACTION_EVENT)
        .constant("APP_METADATA", app_info::METADATA)
        // The native side registers them before the front end exists; the front reads them.
        .constant("DEFAULT_SHORTCUTS", desktop::ShortcutBindings::defaults())
        .constant("FIELD_NAME_PATTERN", notes::placeholder::FIELD_NAME_PATTERN)
        .constant("MINIMUM_PASSPHRASE_LENGTH", vault::MINIMUM_LENGTH)
        // Rust decides where a library lives, so it names its preference file too.
        .constant("PREFERENCES_FILE", layout::PREFERENCES)
        // Read by Rust out of the application's file before the front end has booted.
        .constant("AUTOMATIC_BACKUPS_KEY", backup::AUTOMATIC_BACKUPS_KEY)
}

/// ⚠️ Not the plugin's `all()`, which carries `VISIBLE`: quitting from the tray saves a
/// hidden window, and the next launch restores it hidden, with nothing on screen.
const WINDOW_STATE_FLAGS: StateFlags = StateFlags::SIZE
    .union(StateFlags::POSITION)
    .union(StateFlags::MAXIMIZED);

/// ⚠️ `single_instance` before every other plugin, and `log` before the plugins that log
/// while they initialise.
fn with_plugins(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    #[allow(unused_mut)]
    let mut builder = builder
        // A second process on the same SQLite file would also lose every global shortcut.
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
        // What the palette has left once the window is hidden; the OS may refuse it.
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(WINDOW_STATE_FLAGS)
                .build(),
        );

    // The end-to-end harness. `tauri_plugin_log` holds the global logger, so WDIO captures
    // no backend log: read the log plugin's targets instead.
    #[cfg(feature = "e2e")]
    {
        builder = builder
            .plugin(tauri_plugin_wdio::init())
            .plugin(tauri_plugin_wdio_webdriver::init());
    }

    builder
}

/// Plugins that need a handle, the native state, then the database slot.
fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // ⚠️ Declared `"visible": false` and shown here: the window-state plugin restores the
    // geometry before `setup` runs, so a window created visible shows the config size, then
    // jumps. Here rather than in the front end, which may fail to boot.
    if let Some(window) = app.get_webview_window("main") {
        window.set_title(app_info::METADATA.name)?;
        window.show()?;
    }

    app.handle()
        .plugin(tauri_plugin_updater::Builder::new().build())?;

    // The plugin takes `MacosLauncher` on every platform and ignores it off macOS.
    app.handle().plugin(tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        None,
    ))?;

    desktop::init(app.handle())?;

    // Logged, never propagated: an error out of `setup` panics before the window shows
    // anything. `vault_state`, `create_vault` and `unlock_vault` report it where it is read.
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

    // Empty until `vault::unlock` fills it with the connection and the key.
    app.manage(db::Db::new(None));

    Ok(())
}

/// The launch chores, run after the unlock: nothing can be read before it, and none of
/// them is fatal.
pub(crate) fn sweep(handle: &tauri::AppHandle) {
    let db = handle.state::<db::Db>();

    // ⚠️ The copy before the sweeps: it is worth most holding what retention will purge.
    backup::take(handle, &db);

    notes::trash::sweep_at_startup(&db);
    if let Err(error) = attachments::sweep_orphan_files(&db) {
        log::warn!("Orphan attachment files not swept: {error}");
    }
    // The decrypted copies `open_attachment` wrote: another application may still hold
    // them on close, so they go at the next launch at the latest.
    attachments::sealed::sweep_plaintext(handle);
}

/// Both are refused when there is no tray to find the window in (see `desktop`).
fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    match event {
        tauri::WindowEvent::CloseRequested { api, .. }
            if desktop::hides_on_close(window.app_handle()) =>
        {
            api.prevent_close();
            let _ = window.hide();
        }
        // Tauri emits nothing for "minimized": `Resized` is the only way through.
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

    // Not in release: no `src/` sits next to an installed binary. `eprintln!` because the
    // log plugin has not taken the global logger yet.
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
            // Built then run for this event: a decrypted copy should not outlive the
            // session. Best effort — the sweep at launch covers a locked file or a crash.
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
