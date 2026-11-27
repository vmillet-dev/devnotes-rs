//! The Windows executable carries its icon and its version: the shell, the installer's shortcuts
//! and the taskbar read them from the file, whatever the window sets.

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=icons/icon.ico");
    // ⚠️ `cfg` is the host's in a build script, and `winresource` is built on Windows hosts alone:
    // each system builds its own packages, so the host is the target.
    #[cfg(windows)]
    windows::embed_icon_and_version();
}

#[cfg(windows)]
mod windows {
    use std::{env, fs, path::Path};

    pub(super) fn embed_icon_and_version() {
        if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
            return;
        }
        let manifest_dir =
            env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo");
        let manifest_dir = Path::new(&manifest_dir);
        let name = product_name(manifest_dir);
        let icon = manifest_dir.join("icons/icon.ico");
        winresource::WindowsResource::new()
            .set_icon(icon.to_str().expect("the repository is at a path of text"))
            .set("ProductName", &name)
            .set("FileDescription", &name)
            .compile()
            .expect("the Windows SDK's resource compiler embeds the icon");
    }

    /// The name the packages give the application, `[package.metadata.packager] product-name`;
    /// the version is the workspace's, which `winresource` reads from Cargo itself.
    fn product_name(manifest_dir: &Path) -> String {
        let text =
            fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("Cargo.toml is readable");
        let manifest: toml::Table = toml::from_str(&text).expect("Cargo.toml is valid TOML");
        manifest
            .get("package")
            .and_then(|package| package.get("metadata"))
            .and_then(|metadata| metadata.get("packager"))
            .and_then(|packager| packager.get("product-name"))
            .and_then(toml::Value::as_str)
            .expect("[package.metadata.packager] product-name is a string")
            .to_owned()
    }
}
