use std::{env, fs, path::Path};

fn main() {
    export_metadata();
    tauri_build::build();
    embed_test_manifest();
}

/// `tauri_build`'s default manifest: the dependency on Common Controls v6.
const COMMON_CONTROLS_V6: &str = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
        processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*" />
    </dependentAssembly>
  </dependency>
</assembly>
"#;

/// ⚠️ `tauri_build` embeds its manifest in the binaries only. A test executable that builds a
/// Tauri app without it loads `comctl32` v5 and dies before its first test (0xc0000139).
fn embed_test_manifest() {
    let target = |key: &str| env::var(key).unwrap_or_default();
    if target("CARGO_CFG_TARGET_OS") != "windows" || target("CARGO_CFG_TARGET_ENV") != "msvc" {
        return;
    }

    let path =
        Path::new(&env::var("OUT_DIR").expect("OUT_DIR is set by cargo")).join("tests.manifest");
    fs::write(&path, COMMON_CONTROLS_V6).expect("OUT_DIR is writable");

    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
        path.display()
    );
    println!("cargo:rustc-link-arg-tests=/MANIFESTUAC:NO");
}

/// Cargo does not pass `[package.metadata]` to the crate, so it is read here and handed
/// over as environment variables the code reads with `env!` — resolved at compile time,
/// and the manifest stays the only place these values are written.
fn export_metadata() {
    println!("cargo:rerun-if-changed=Cargo.toml");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo");
    let text = fs::read_to_string(Path::new(&manifest_dir).join("Cargo.toml"))
        .expect("Cargo.toml is readable");
    let manifest: toml::Table = toml::from_str(&text).expect("Cargo.toml is valid TOML");

    let metadata = manifest
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("devnotes"))
        .expect("Cargo.toml declares [package.metadata.devnotes]");

    for key in ["display-name", "author-handle"] {
        let value = metadata
            .get(key)
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("[package.metadata.devnotes] {key} is a string"));
        let name = format!("DEVNOTES_{}", key.to_uppercase().replace('-', "_"));

        println!("cargo:rustc-env={name}={value}");
    }

    // Cargo joins `authors` with `;`; the about card names one person.
    let authors = env::var("CARGO_PKG_AUTHORS").unwrap_or_default();
    let author = authors.split(';').next().unwrap_or_default();

    println!("cargo:rustc-env=DEVNOTES_AUTHOR={author}");
    println!(
        "cargo:rustc-env=DEVNOTES_RUST_VERSION={}",
        pinned_toolchain(&manifest_dir)
    );
}

/// The toolchain `rust-toolchain.toml` pins, which rustup resolves for every build.
fn pinned_toolchain(manifest_dir: &str) -> String {
    let path = Path::new(manifest_dir).join("../rust-toolchain.toml");
    println!("cargo:rerun-if-changed=../rust-toolchain.toml");

    let text = fs::read_to_string(&path).expect("rust-toolchain.toml is readable");
    let pinned: toml::Table = toml::from_str(&text).expect("rust-toolchain.toml is valid TOML");

    pinned
        .get("toolchain")
        .and_then(|toolchain| toolchain.get("channel"))
        .and_then(toml::Value::as_str)
        .expect("rust-toolchain.toml pins a channel")
        .to_string()
}
