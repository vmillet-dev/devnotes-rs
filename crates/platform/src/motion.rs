//! Whether the system asks for less motion, read through safe means only (D7): the registry
//! under Windows, `gsettings` under GNOME — what the reference's `prefers-reduced-motion` read.
//! Elsewhere, and when neither answers, the interface moves.

/// Windows turns « Animation effects » off by clearing `MinAnimate`, among other flags.
#[cfg(windows)]
pub fn system_prefers_reduced() -> bool {
    let desktop = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    desktop
        .open_subkey(r"Control Panel\Desktop\WindowMetrics")
        .and_then(|metrics| metrics.get_value::<String, _>("MinAnimate"))
        .is_ok_and(|value| value.trim() == "0")
}

#[cfg(target_os = "linux")]
pub fn system_prefers_reduced() -> bool {
    std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "enable-animations"])
        .output()
        .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).trim() == "false")
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn system_prefers_reduced() -> bool {
    false
}
