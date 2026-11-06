---
name: platform-integration
description: Use when adding or changing what DevNotes asks of the operating system (crates/platform — paths, window geometry, single instance, motion, file dialogs, and later tray, shortcuts, clipboard, notifications, autostart, updates) or the binary's start-up in crates/app.
---

# Platform integration

`crates/platform` is the one place that names a native crate (plan § 3.10). Neither egui nor the core appears there; the interface asks it through plain functions and traits.

1. **Safe APIs only** (D7): `unsafe_code` is forbidden in every crate. A native need without a safe crate is a decision for the plan, not an `unsafe` block.
2. **One module per subject** (`paths`, `window`, `instance`, `motion`, `dialog`…), its OS differences behind `cfg(target_os)` inside it. What a screen calls is a trait when a test must fake it (`dialog::FileDialog`, the clipboard, the window): the fake answers, it never reimplements a rule.
3. **A new native crate** goes through `dependency-audit` first (licence in `deny.toml`'s allow list, advisories, maintenance), pinned exactly. Do not bump one as part of a feature.
4. **Start-up order is load-bearing** (`crates/app/src/main.rs`): script host, arguments, instance claim (fails closed: one process per profile, D16), logger, panic hook (location and thread, never the message, D18), preferences, then the window — created hidden, shown by the first frame, eframe persistence off.
5. **Traps**: under native Wayland winit 0.30 drops files and cannot hide the window, so X11 is taken when `DISPLAY` exists (D15); on Windows a named-pipe `PermissionDenied` means the profile is taken; half-written files are staged then renamed.
6. **The `e2e` flavour** (`--features e2e`) adds eframe's inspection port, the software adapter and two doors — `--action <name>` and `DEVNOTES_E2E_DIALOG` — and nothing else; it takes no global key. A release build of it does not compile. Never widen it.
7. Test in the module: a temporary directory for anything on disk; a real second process only where the subject is processes (`instance`).

**Verify**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --exclude e2e`, then run the app on a temporary profile (`cargo run -p app -- --profile <dir>`) on each OS the change touches, and say which ones did not run. Reference: plan § 3.10, § 3.13; `docs/architecture.md` → "The binary, its profile and its window".
