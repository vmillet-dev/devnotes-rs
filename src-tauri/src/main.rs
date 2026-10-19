// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Before Tauri: a script host opens no window and takes no single instance.
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == devnotes_lib::SCRIPT_HOST_ARG)
    {
        std::process::exit(devnotes_lib::serve_scripts());
    }
    devnotes_lib::run();
}
