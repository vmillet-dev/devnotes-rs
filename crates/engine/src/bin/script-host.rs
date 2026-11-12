//! The script host alone, for this crate's tests: the application relaunches itself with
//! `--devnotes-script-host`, but no application binary is built here.

fn main() {
    std::process::exit(engine::serve_scripts());
}
