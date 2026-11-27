//! What Windows reads from the executable itself: the shell, the installer's shortcuts and the
//! taskbar take the icon from the file, not from the window.
#![cfg(windows)]

const RT_GROUP_ICON: u32 = 14;
const RT_VERSION: u32 = 16;

#[test]
fn the_executable_carries_its_icon_and_its_version() {
    let file = std::fs::read(env!("CARGO_BIN_EXE_devnotes")).unwrap();
    let types = resource_types(&file);

    assert!(types.contains(&RT_GROUP_ICON), "no icon: {types:?}");
    assert!(types.contains(&RT_VERSION), "no version: {types:?}");
}

/// The types listed by the file's root resource directory, read from its PE headers; none when it
/// has no resource table.
fn resource_types(file: &[u8]) -> Vec<u32> {
    let u16_at = |at: usize| usize::from(u16::from_le_bytes([file[at], file[at + 1]]));
    let u32_at = |at: usize| u32::from_le_bytes(file[at..at + 4].try_into().unwrap());

    let pe = u32_at(0x3C) as usize;
    let optional = pe + 24;
    let directories = optional + if u16_at(optional) == 0x20B { 112 } else { 96 };
    let table = u32_at(directories + 2 * 8);
    if table == 0 {
        return Vec::new();
    }

    let headers = optional + u16_at(pe + 20);
    let root = (0..u16_at(pe + 6))
        .map(|index| headers + 40 * index)
        .find_map(|section| {
            let start = u32_at(section + 12);
            let length = u32_at(section + 8).max(u32_at(section + 16));
            (start..start + length)
                .contains(&table)
                .then(|| (table - start + u32_at(section + 20)) as usize)
        })
        .expect("the resource table lies in a section");

    let named = u16_at(root + 12);
    let numbered = u16_at(root + 14);
    (named..named + numbered)
        .map(|entry| u32_at(root + 16 + 8 * entry))
        .collect()
}
