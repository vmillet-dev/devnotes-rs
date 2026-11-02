// A module holding commands is `pub`, and its commands are re-exported here: the macros
// `#[tauri::command]` and `#[specta::specta]` write sit at the crate root, where the shell
// finds them as `engine::<command>`. The rest is `pub(crate)`, for `unreachable_pub`.
pub mod attachments;
pub mod backup;
pub mod changelog;
pub mod db;
pub mod error;
pub mod folders;
pub mod host;
pub mod http;
pub mod json;
pub mod libraries;
pub mod notes;
pub mod recovery;
pub mod spaces;
pub mod tools;
pub mod transfer;
pub mod vault;

pub mod app_info;
pub(crate) mod closed_enum;
pub(crate) mod count;
pub mod layout;
pub(crate) mod name;

use std::sync::Arc;

pub use host::{EngineEvent, EventSink, Paths, RecordingSink};

pub use attachments::{
    attach_clipboard_image, attach_file, delete_attachment, list_attachments, open_attachment,
    read_attachment, save_attachment,
};
pub use backup::{list_backups, restore_backup, take_restored_aside};
pub use changelog::{app_changelog, release_notes};
pub use folders::{
    arrange_board, board_view, create_folder, delete_folder, file_notes, file_notes_back,
    list_folders, move_folder, move_folder_back, recolour_folder, rename_folder, save_board_layout,
};
pub use http::{
    add_graphql_variables, cancel_http_run, cancel_http_send, check_http_headers,
    check_http_variables, clear_http_cookies, clear_http_history, close_websocket,
    connect_websocket, count_http_contents, count_http_cookies, count_http_history,
    create_http_collection, create_http_environment, create_http_folder, create_http_request,
    create_http_request_in, delete_http_cookie, delete_http_cookie_domain, delete_http_environment,
    delete_http_history_entry, delete_http_item, describe_graphql, describe_http_body,
    duplicate_http_environment, duplicate_http_item, forget_http_response, generate_http_code,
    get_http_request, http_cookies, http_environments, http_history, http_history_draft,
    http_history_entry, http_history_onto_request, http_response_image, http_response_note,
    http_run_order, http_settings, http_tree, http_variables, implied_http_headers,
    inherited_http_settings, move_http_item, read_http_headers, read_http_run_data,
    rename_http_item, reorder_http_collection, restore_http_history_entry, run_http_collection,
    save_http_environment, save_http_globals, save_http_request, save_http_response,
    save_http_run_order, save_http_settings, search_http_requests, send_http_request,
    send_websocket, sync_http_query,
};
pub use json::explore_json;
pub use libraries::{create_library, delete_library, list_libraries, open_library, rename_library};
pub use notes::{
    convert_note, count_notes_tagged, create_note, delete_note, delete_notes, delete_tags,
    detect_language, drawing_picture, drawing_thumbnail, duplicate_note, empty_trash,
    evaluate_calc, field_spans, fill_placeholders, get_note, list_drawings,
    list_global_placeholders, list_revisions, list_tags, list_trash, move_notes, move_notes_back,
    present_note, purge_notes, query_notes, remove_tag, rename_tags, restore_notes,
    restore_priorities, restore_revision, restore_tags, restore_trash, revision_diff, seed_samples,
    set_global_placeholders, set_pinned, set_placeholder_values, set_priority, tag_notes,
    untag_notes, update_note,
};
pub use recovery::{archive_locked_library, set_aside_damaged_library};
pub use spaces::{
    count_space_contents, create_space, delete_space, list_spaces, pin_space, rename_space,
};
pub use tools::{
    check_digits, convert_case, convert_data, convert_size, copy_qr_code, current_instant,
    decode_bytes, decode_jwt, describe_cidr, describe_colour, describe_cron, describe_instant,
    describe_permissions, describe_qr_code, diff_json, diff_text, encode_base64_file, encode_bytes,
    estimate_transfer, export_identities, fix_line_breaks, generate_identifiers, generate_identity,
    generate_json, generate_passwords, hash_input, inspect_identifier, lorem_ipsum,
    measure_durations, parse_url, place_in_zones, save_bytes, save_qr_code, search_time_zones,
    text_stats, time_in_zone, url_codec,
};
pub use transfer::{count_export, export_is_protected, export_notes, import_notes, share_notes};
pub use vault::{change_passphrase, create_vault, unlock_vault, vault_state};

/// The core's state, created once by the shell and shared behind an `Arc`.
pub struct Engine {
    pub(crate) db: db::Db,
    pub(crate) restored: backup::Restored,
    pub(crate) sending: http::send::Sending,
    pub(crate) runs: http::runner::Runs,
    pub(crate) sockets: http::websocket::Sockets,
    pub(crate) paths: Paths,
    pub(crate) events: Arc<dyn EventSink>,
}

impl Engine {
    /// No library is open: `vault::unlock` opens one.
    pub fn new(paths: Paths, events: Arc<dyn EventSink>) -> Self {
        Self {
            db: db::Db::new(None),
            restored: backup::Restored::default(),
            sending: http::send::Sending::default(),
            runs: http::runner::Runs::default(),
            sockets: http::websocket::Sockets::default(),
            paths,
            events,
        }
    }

    /// For the tests, which open a library of their own.
    pub fn db(&self) -> &db::Db {
        &self.db
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub(crate) fn emit(&self, event: EngineEvent) {
        self.events.emit(event);
    }
}

/// The launch chores, run after the unlock: nothing can be read before it, and none of
/// them is fatal.
pub(crate) fn sweep(engine: &Engine) {
    let db = &engine.db;

    // ⚠️ The copy before the sweeps: it is worth most holding what retention will purge.
    backup::take(&engine.paths, db);

    notes::trash::sweep_at_startup(db);
    http::history::sweep_at_startup(db);
    if let Err(error) = attachments::files::sweep_orphan_files(db) {
        log::warn!("Orphan attachment files not swept: {error}");
    }
    // The decrypted copies `open_attachment` wrote: another application may still hold
    // them on close, so they go at the next launch at the latest.
    attachments::sealed::sweep_plaintext(&engine.paths);
}

/// Started with this first argument, the application is a script host and nothing else: no
/// window, no library (`http::scripts::child`).
pub use http::scripts::child::ARG as SCRIPT_HOST_ARG;

/// The script host's `main`: its exit code.
pub fn serve_scripts() -> i32 {
    http::scripts::child::serve()
}
