---
name: tauri-integration
description: Use when adding or changing Tauri v2 commands, events, IPC types, plugins or capability permissions in src-tauri (Tauri ~2.11, tauri-specta rc.25).
---

# Tauri integration

**Command**

1. `#[tauri::command]` + `#[specta::specta]`; a command taking the DB lock is `async` and its body runs in `db::blocking`. With an `AppHandle`, be generic over `R: Runtime` and register as `name::<tauri::Wry>`.
2. Validate in the model before locking; return `Result<_, AppError>` with an `ErrorCode`; no user-visible string in Rust.
3. Add to `collect_commands!` in `src-tauri/src/lib.rs`; use `u32`, not `usize`/`i64`.
4. `npm run bindings` (regenerates `src/app/core/ipc/bindings.ts`; commit it). Wrap in a repository under `core/data/`.
5. New `ErrorCode` → `CODE_KEYS` (both locales) + `IPC_ERROR_CODES`.
6. Test: `src-tauri/tests/commands/` on the mock runtime, plus `ipc_contract`.

**Plugin / permission**

- Add the crate and `.plugin(...)` in `lib.rs`, then the **narrowest** permission in `src-tauri/capabilities/default.json`. Never `*:default` when an `allow-*` entry exists; scope URL/path permissions. Keep the CSP (`ipc:`, `http://ipc.localhost` in `connect-src`).
- Do not bump Tauri or plugin versions as part of a feature.

**Verify**: `cargo clippy --all-targets -- -D warnings`, `cargo test` (from `src-tauri/`), `npm run build`. Reference: `docs/architecture.md` → "IPC boundary".
