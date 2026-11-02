use std::{env, fs, path::Path};

fn main() {
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
