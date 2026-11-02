//! The Tauri shell over `engine`: the plugins, the window, the tray and the one list of
//! commands the front end may call.

pub mod desktop;
mod host;

use std::sync::Arc;

use engine::{Engine, app_info, attachments, backup, http, layout, notes, vault};
use tauri::Manager;
use tauri_plugin_window_state::StateFlags;
use tauri_specta::{Builder, collect_commands};

use desktop::{
    hold_quit, release_global_shortcuts, set_global_shortcuts, set_window_behavior, sync_tray,
};

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
#[allow(
    clippy::too_many_lines,
    reason = "one line per command: the list is the point"
)]
fn ipc_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            engine::query_notes::<tauri::Wry>,
            engine::get_note::<tauri::Wry>,
            engine::drawing_picture::<tauri::Wry>,
            engine::drawing_thumbnail::<tauri::Wry>,
            engine::list_drawings::<tauri::Wry>,
            engine::create_note::<tauri::Wry>,
            engine::duplicate_note::<tauri::Wry>,
            engine::detect_language,
            engine::evaluate_calc,
            engine::present_note,
            engine::field_spans,
            engine::explore_json,
            engine::convert_case,
            engine::fix_line_breaks,
            engine::parse_url,
            engine::url_codec,
            engine::encode_bytes,
            engine::encode_base64_file,
            engine::decode_bytes,
            engine::save_bytes,
            engine::hash_input,
            engine::generate_passwords,
            engine::generate_identifiers,
            engine::inspect_identifier,
            engine::text_stats,
            engine::describe_qr_code,
            engine::save_qr_code,
            engine::copy_qr_code::<tauri::Wry>,
            engine::describe_cidr,
            engine::describe_colour,
            engine::convert_data,
            engine::generate_json,
            engine::generate_identity::<tauri::Wry>,
            engine::export_identities::<tauri::Wry>,
            engine::lorem_ipsum,
            engine::diff_json,
            engine::diff_text,
            engine::describe_instant,
            engine::current_instant,
            engine::measure_durations,
            engine::search_time_zones,
            engine::place_in_zones,
            engine::time_in_zone,
            engine::describe_cron,
            engine::convert_size,
            engine::estimate_transfer,
            engine::describe_permissions,
            engine::check_digits,
            engine::decode_jwt,
            engine::seed_samples::<tauri::Wry>,
            engine::update_note::<tauri::Wry>,
            engine::convert_note::<tauri::Wry>,
            engine::list_revisions::<tauri::Wry>,
            engine::revision_diff::<tauri::Wry>,
            engine::restore_revision::<tauri::Wry>,
            engine::delete_note::<tauri::Wry>,
            engine::delete_notes::<tauri::Wry>,
            engine::restore_notes::<tauri::Wry>,
            engine::list_trash::<tauri::Wry>,
            engine::restore_trash::<tauri::Wry>,
            engine::purge_notes::<tauri::Wry>,
            engine::empty_trash::<tauri::Wry>,
            engine::move_notes::<tauri::Wry>,
            engine::move_notes_back::<tauri::Wry>,
            engine::tag_notes::<tauri::Wry>,
            engine::untag_notes::<tauri::Wry>,
            engine::remove_tag::<tauri::Wry>,
            engine::restore_tags::<tauri::Wry>,
            engine::set_pinned::<tauri::Wry>,
            engine::set_priority::<tauri::Wry>,
            engine::restore_priorities::<tauri::Wry>,
            engine::count_notes_tagged::<tauri::Wry>,
            engine::list_tags::<tauri::Wry>,
            engine::rename_tags::<tauri::Wry>,
            engine::delete_tags::<tauri::Wry>,
            engine::fill_placeholders::<tauri::Wry>,
            engine::set_placeholder_values::<tauri::Wry>,
            engine::list_global_placeholders::<tauri::Wry>,
            engine::set_global_placeholders::<tauri::Wry>,
            engine::list_folders::<tauri::Wry>,
            engine::http_tree::<tauri::Wry>,
            engine::search_http_requests::<tauri::Wry>,
            engine::create_http_collection::<tauri::Wry>,
            engine::create_http_folder::<tauri::Wry>,
            engine::create_http_request::<tauri::Wry>,
            engine::create_http_request_in::<tauri::Wry>,
            engine::get_http_request::<tauri::Wry>,
            engine::save_http_request::<tauri::Wry>,
            engine::rename_http_item::<tauri::Wry>,
            engine::count_http_contents::<tauri::Wry>,
            engine::delete_http_item::<tauri::Wry>,
            engine::duplicate_http_item::<tauri::Wry>,
            engine::move_http_item::<tauri::Wry>,
            engine::reorder_http_collection::<tauri::Wry>,
            engine::sync_http_query,
            engine::http_settings::<tauri::Wry>,
            engine::save_http_settings::<tauri::Wry>,
            engine::inherited_http_settings::<tauri::Wry>,
            engine::describe_http_body,
            engine::implied_http_headers::<tauri::Wry>,
            engine::check_http_headers,
            engine::read_http_headers,
            engine::describe_graphql,
            engine::add_graphql_variables,
            engine::connect_websocket::<tauri::Wry>,
            engine::send_websocket::<tauri::Wry>,
            engine::close_websocket::<tauri::Wry>,
            engine::send_http_request::<tauri::Wry>,
            engine::cancel_http_send::<tauri::Wry>,
            engine::run_http_collection::<tauri::Wry>,
            engine::http_run_order::<tauri::Wry>,
            engine::save_http_run_order::<tauri::Wry>,
            engine::cancel_http_run::<tauri::Wry>,
            engine::read_http_run_data,
            engine::save_http_response::<tauri::Wry>,
            engine::forget_http_response::<tauri::Wry>,
            engine::http_response_image::<tauri::Wry>,
            engine::http_response_note::<tauri::Wry>,
            engine::http_history::<tauri::Wry>,
            engine::http_history_entry::<tauri::Wry>,
            engine::http_history_draft::<tauri::Wry>,
            engine::http_history_onto_request::<tauri::Wry>,
            engine::delete_http_history_entry::<tauri::Wry>,
            engine::restore_http_history_entry::<tauri::Wry>,
            engine::count_http_history::<tauri::Wry>,
            engine::clear_http_history::<tauri::Wry>,
            engine::http_cookies::<tauri::Wry>,
            engine::delete_http_cookie::<tauri::Wry>,
            engine::delete_http_cookie_domain::<tauri::Wry>,
            engine::count_http_cookies::<tauri::Wry>,
            engine::clear_http_cookies::<tauri::Wry>,
            engine::http_environments::<tauri::Wry>,
            engine::create_http_environment::<tauri::Wry>,
            engine::save_http_environment::<tauri::Wry>,
            engine::save_http_globals::<tauri::Wry>,
            engine::duplicate_http_environment::<tauri::Wry>,
            engine::delete_http_environment::<tauri::Wry>,
            engine::check_http_variables,
            engine::http_variables::<tauri::Wry>,
            engine::generate_http_code::<tauri::Wry>,
            engine::board_view::<tauri::Wry>,
            engine::save_board_layout::<tauri::Wry>,
            engine::arrange_board::<tauri::Wry>,
            engine::create_folder::<tauri::Wry>,
            engine::rename_folder::<tauri::Wry>,
            engine::recolour_folder::<tauri::Wry>,
            engine::delete_folder::<tauri::Wry>,
            engine::move_folder::<tauri::Wry>,
            engine::move_folder_back::<tauri::Wry>,
            engine::file_notes::<tauri::Wry>,
            engine::file_notes_back::<tauri::Wry>,
            engine::list_spaces::<tauri::Wry>,
            engine::count_space_contents::<tauri::Wry>,
            engine::create_space::<tauri::Wry>,
            engine::rename_space::<tauri::Wry>,
            engine::pin_space::<tauri::Wry>,
            engine::delete_space::<tauri::Wry>,
            engine::attach_file::<tauri::Wry>,
            engine::attach_clipboard_image::<tauri::Wry>,
            engine::list_attachments::<tauri::Wry>,
            engine::read_attachment::<tauri::Wry>,
            engine::open_attachment::<tauri::Wry>,
            engine::save_attachment::<tauri::Wry>,
            engine::delete_attachment::<tauri::Wry>,
            engine::export_notes::<tauri::Wry>,
            engine::count_export::<tauri::Wry>,
            engine::import_notes::<tauri::Wry>,
            engine::share_notes::<tauri::Wry>,
            engine::export_is_protected,
            engine::vault_state::<tauri::Wry>,
            engine::create_vault::<tauri::Wry>,
            engine::unlock_vault::<tauri::Wry>,
            engine::change_passphrase::<tauri::Wry>,
            engine::set_aside_damaged_library::<tauri::Wry>,
            engine::archive_locked_library::<tauri::Wry>,
            engine::list_backups::<tauri::Wry>,
            engine::restore_backup::<tauri::Wry>,
            engine::take_restored_aside,
            engine::list_libraries::<tauri::Wry>,
            engine::create_library::<tauri::Wry>,
            engine::open_library::<tauri::Wry>,
            engine::rename_library::<tauri::Wry>,
            engine::delete_library::<tauri::Wry>,
            engine::app_changelog,
            engine::release_notes,
            sync_tray,
            set_global_shortcuts,
            release_global_shortcuts::<tauri::Wry>,
            set_window_behavior,
            hold_quit,
        ])
        // Reachable from no command, so exported on its own, with the topic it travels on.
        .typ::<desktop::GlobalAction>()
        .constant("GLOBAL_ACTION_EVENT", desktop::ACTION_EVENT)
        .typ::<http::websocket::WebsocketEvent>()
        .typ::<http::console::ConsoleEvent>()
        .typ::<http::runner::RunEvent>()
        .constant("WEBSOCKET_EVENT", http::websocket::WEBSOCKET_EVENT)
        .constant("HTTP_CONSOLE_EVENT", http::console::HTTP_CONSOLE_EVENT)
        .constant("HTTP_RUN_EVENT", http::runner::HTTP_RUN_EVENT)
        .constant("DEFAULT_TRANSPORT", http::transport::Transport::default())
        .constant("APP_METADATA", app_info::METADATA)
        // The native side registers them before the front end exists; the front reads them.
        .constant("DEFAULT_SHORTCUTS", desktop::ShortcutBindings::defaults())
        .constant("FIELD_NAME_PATTERN", notes::placeholder::FIELD_NAME_PATTERN)
        .constant("CALC_UNITS", notes::calc::units::catalogue())
        .constant("CALC_WORDS", notes::calc::words::catalogue())
        .constant("MINIMUM_PASSPHRASE_LENGTH", vault::MINIMUM_LENGTH)
        .constant(
            "ATTACHMENT_MAX_MEGABYTES",
            attachments::model::MAX_MEGABYTES,
        )
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
    let paths = host::paths(app.handle())?;
    if let Err(error) = std::fs::create_dir_all(&paths.profile) {
        log::error!(
            "The data directory {} could not be prepared: {error}",
            paths.profile.display()
        );
    }

    // No library until `vault::unlock` opens one.
    let events = Arc::new(host::TauriHost(app.handle().clone()));
    app.manage(Arc::new(Engine::new(paths, events)));

    Ok(())
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
        // Closing the last window ends the process: the page is asked first, as for the
        // tray's « Quitter », and ends it itself.
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            desktop::request_quit(window.app_handle());
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
                let engine = handle.state::<Arc<Engine>>();
                attachments::sealed::sweep_plaintext(engine.paths());
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
