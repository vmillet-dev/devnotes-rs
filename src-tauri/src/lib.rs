//! The Tauri shell over `engine`: the plugins, the window, the tray and the one list of
//! commands the front end may call.

pub mod commands;
pub mod desktop;
mod host;

use std::sync::Arc;

use engine::{Engine, app_info, attachments, backup, http, layout, notes, vault};
use tauri::Manager;
use tauri_plugin_window_state::StateFlags;
use tauri_specta::{Builder, collect_commands};

use commands::{
    attachments::{
        attach_clipboard_image, attach_file, delete_attachment, list_attachments, open_attachment,
        read_attachment, save_attachment,
    },
    backup::{list_backups, restore_backup, take_restored_aside},
    changelog::{app_changelog, release_notes},
    folders::{
        arrange_board, board_view, create_folder, delete_folder, file_notes, file_notes_back,
        list_folders, move_folder, move_folder_back, recolour_folder, rename_folder,
        save_board_layout,
    },
    http::{
        add_graphql_variables, cancel_http_run, cancel_http_send, check_http_headers,
        check_http_variables, clear_http_cookies, clear_http_history, close_websocket,
        connect_websocket, count_http_contents, count_http_cookies, count_http_history,
        create_http_collection, create_http_environment, create_http_folder, create_http_request,
        create_http_request_in, delete_http_cookie, delete_http_cookie_domain,
        delete_http_environment, delete_http_history_entry, delete_http_item, describe_graphql,
        describe_http_body, duplicate_http_environment, duplicate_http_item, forget_http_response,
        generate_http_code, get_http_request, http_cookies, http_environments, http_history,
        http_history_draft, http_history_entry, http_history_onto_request, http_response_image,
        http_response_note, http_run_order, http_settings, http_tree, http_variables,
        implied_http_headers, inherited_http_settings, move_http_item, read_http_headers,
        read_http_run_data, rename_http_item, reorder_http_collection, restore_http_history_entry,
        run_http_collection, save_http_environment, save_http_globals, save_http_request,
        save_http_response, save_http_run_order, save_http_settings, search_http_requests,
        send_http_request, send_websocket, sync_http_query,
    },
    json::explore_json,
    libraries::{create_library, delete_library, list_libraries, open_library, rename_library},
    notes::{
        convert_note, count_notes_tagged, create_note, delete_note, delete_notes, delete_tags,
        detect_language, drawing_picture, drawing_thumbnail, duplicate_note, empty_trash,
        evaluate_calc, field_spans, fill_placeholders, get_note, list_drawings,
        list_global_placeholders, list_revisions, list_tags, list_trash, move_notes,
        move_notes_back, present_note, purge_notes, query_notes, remove_tag, rename_tags,
        restore_notes, restore_priorities, restore_revision, restore_tags, restore_trash,
        revision_diff, seed_samples, set_global_placeholders, set_pinned, set_placeholder_values,
        set_priority, tag_notes, untag_notes, update_note,
    },
    recovery::{archive_locked_library, set_aside_damaged_library},
    spaces::{
        count_space_contents, create_space, delete_space, list_spaces, pin_space, rename_space,
    },
    tools::{
        check_digits, convert_case, convert_data, convert_size, copy_qr_code, current_instant,
        decode_bytes, decode_jwt, describe_cidr, describe_colour, describe_cron, describe_instant,
        describe_permissions, describe_qr_code, diff_json, diff_text, encode_base64_file,
        encode_bytes, estimate_transfer, export_identities, fix_line_breaks, generate_identifiers,
        generate_identity, generate_json, generate_passwords, hash_input, inspect_identifier,
        lorem_ipsum, measure_durations, parse_url, place_in_zones, save_bytes, save_qr_code,
        search_time_zones, text_stats, time_in_zone, url_codec,
    },
    transfer::{count_export, export_is_protected, export_notes, import_notes, share_notes},
    vault::{change_passphrase, create_vault, unlock_vault, vault_state},
};
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
            query_notes,
            get_note,
            drawing_picture,
            drawing_thumbnail,
            list_drawings,
            create_note,
            duplicate_note,
            detect_language,
            evaluate_calc,
            present_note,
            field_spans,
            explore_json,
            convert_case,
            fix_line_breaks,
            parse_url,
            url_codec,
            encode_bytes,
            encode_base64_file,
            decode_bytes,
            save_bytes,
            hash_input,
            generate_passwords,
            generate_identifiers,
            inspect_identifier,
            text_stats,
            describe_qr_code,
            save_qr_code,
            copy_qr_code,
            describe_cidr,
            describe_colour,
            convert_data,
            generate_json,
            generate_identity,
            export_identities,
            lorem_ipsum,
            diff_json,
            diff_text,
            describe_instant,
            current_instant,
            measure_durations,
            search_time_zones,
            place_in_zones,
            time_in_zone,
            describe_cron,
            convert_size,
            estimate_transfer,
            describe_permissions,
            check_digits,
            decode_jwt,
            seed_samples,
            update_note,
            convert_note,
            list_revisions,
            revision_diff,
            restore_revision,
            delete_note,
            delete_notes,
            restore_notes,
            list_trash,
            restore_trash,
            purge_notes,
            empty_trash,
            move_notes,
            move_notes_back,
            tag_notes,
            untag_notes,
            remove_tag,
            restore_tags,
            set_pinned,
            set_priority,
            restore_priorities,
            count_notes_tagged,
            list_tags,
            rename_tags,
            delete_tags,
            fill_placeholders,
            set_placeholder_values,
            list_global_placeholders,
            set_global_placeholders,
            list_folders,
            http_tree,
            search_http_requests,
            create_http_collection,
            create_http_folder,
            create_http_request,
            create_http_request_in,
            get_http_request,
            save_http_request,
            rename_http_item,
            count_http_contents,
            delete_http_item,
            duplicate_http_item,
            move_http_item,
            reorder_http_collection,
            sync_http_query,
            http_settings,
            save_http_settings,
            inherited_http_settings,
            describe_http_body,
            implied_http_headers,
            check_http_headers,
            read_http_headers,
            describe_graphql,
            add_graphql_variables,
            connect_websocket,
            send_websocket,
            close_websocket,
            send_http_request,
            cancel_http_send,
            run_http_collection,
            http_run_order,
            save_http_run_order,
            cancel_http_run,
            read_http_run_data,
            save_http_response,
            forget_http_response,
            http_response_image,
            http_response_note,
            http_history,
            http_history_entry,
            http_history_draft,
            http_history_onto_request,
            delete_http_history_entry,
            restore_http_history_entry,
            count_http_history,
            clear_http_history,
            http_cookies,
            delete_http_cookie,
            delete_http_cookie_domain,
            count_http_cookies,
            clear_http_cookies,
            http_environments,
            create_http_environment,
            save_http_environment,
            save_http_globals,
            duplicate_http_environment,
            delete_http_environment,
            check_http_variables,
            http_variables,
            generate_http_code,
            board_view,
            save_board_layout,
            arrange_board,
            create_folder,
            rename_folder,
            recolour_folder,
            delete_folder,
            move_folder,
            move_folder_back,
            file_notes,
            file_notes_back,
            list_spaces,
            count_space_contents,
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
            count_export,
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
            take_restored_aside,
            list_libraries,
            create_library,
            open_library,
            rename_library,
            delete_library,
            app_changelog,
            release_notes,
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
    let runtime = tauri::async_runtime::handle().inner().clone();
    app.manage(Arc::new(Engine::new(paths, events, runtime)));

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
                engine.wipe_index();
                attachments::sealed::sweep_plaintext(engine.paths());
            }
        });
}

#[cfg(test)]
mod tests {
    use super::{StateFlags, WINDOW_STATE_FLAGS, app_info};

    /// The one flag that turns "remembers its geometry" into "does not come back".
    #[test]
    fn the_window_state_never_remembers_that_it_was_hidden() {
        assert!(!WINDOW_STATE_FLAGS.contains(StateFlags::VISIBLE));
        assert!(WINDOW_STATE_FLAGS.contains(StateFlags::SIZE));
        assert!(WINDOW_STATE_FLAGS.contains(StateFlags::POSITION));
    }

    /// Tauri refuses an `openUrl` outside the scope at runtime, with nothing on screen: read
    /// from the shipped capability, next to the constant it constrains.
    #[test]
    fn the_repository_stays_inside_the_scope_the_capability_allows() {
        const CAPABILITY: &str = include_str!("../capabilities/default.json");

        let declared: serde_json::Value = serde_json::from_str(CAPABILITY).expect("valid JSON");
        let scopes = declared["permissions"]
            .as_array()
            .expect("a permissions array")
            .iter()
            .filter(|permission| permission["identifier"] == "opener:allow-open-url")
            .flat_map(|permission| permission["allow"].as_array().expect("an allow list"))
            .filter_map(|entry| entry["url"].as_str())
            .collect::<Vec<_>>();

        assert!(
            !scopes.is_empty(),
            "no opener:allow-open-url scope to check"
        );
        assert!(
            scopes.iter().any(|scope| match scope.strip_suffix('*') {
                Some(prefix) => app_info::METADATA.repository.starts_with(prefix),
                None => *scope == app_info::METADATA.repository,
            }),
            "{} is outside {scopes:?}",
            app_info::METADATA.repository
        );
    }
}
