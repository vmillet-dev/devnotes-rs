//! S9: prints its version and where it finds its resources; `--update <latest.json URL>
//! <public key>` checks, downloads, verifies and installs the update (AppImage replaced in place).

fn resource_dir() -> std::path::PathBuf {
    // AppImage: $APPDIR/usr/lib/<name>; deb/rpm: /usr/lib/<name>; Windows: beside the exe.
    let exe = std::env::current_exe().expect("exe");
    let beside = exe.parent().expect("dir").to_path_buf();
    for candidate in [
        std::env::var_os("APPDIR").map(|d| std::path::PathBuf::from(d).join("usr/lib/devnotes-s9")),
        Some(beside.join("../lib/devnotes-s9")),
        Some(beside.clone()),
    ]
    .into_iter()
    .flatten()
    {
        if candidate.join("resources/identity/names.txt").exists() || candidate.join("identity/names.txt").exists() {
            return candidate;
        }
    }
    beside
}

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    println!("devnotes-s9 {version}; exe {:?}; resources {:?}", std::env::current_exe().ok(), resource_dir());
    let args: Vec<String> = std::env::args().collect();
    if let (Some("--update"), Some(endpoint), Some(pubkey)) = (args.get(1).map(String::as_str), args.get(2), args.get(3)) {
        let config = cargo_packager_updater::Config {
            endpoints: vec![endpoint.parse().expect("url")],
            pubkey: pubkey.clone(),
            windows: Some(cargo_packager_updater::WindowsConfig { install_mode: Some(cargo_packager_updater::WindowsUpdateInstallMode::Passive), ..Default::default() }),
        };
        match cargo_packager_updater::check_update(version.parse().expect("semver"), config) {
            Ok(Some(update)) => {
                println!("update {} → {} ({})", update.current_version, update.version, update.format);
                match update.download_and_install() {
                    Ok(()) => println!("installed"),
                    Err(error) => println!("install failed: {error}"),
                }
            }
            Ok(None) => println!("up to date"),
            Err(error) => println!("check failed: {error}"),
        }
    }
}
