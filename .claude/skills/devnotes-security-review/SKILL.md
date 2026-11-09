---
name: devnotes-security-review
description: DevNotes-specific security review for encryption, vault/passphrase, secrets, attachments, HTTP client, the platform crate and privacy. Use when touching vault/, libraries, backup/, attachments, http/, the gate, platform/, the updater or anything logged. Complements the built-in generic /security-review.
---

# DevNotes security review

**Hard rules**: never rewrite or "improve" the cryptography, key derivation or file formats on your own; propose and ask. Do not log, print or persist passphrases, keys, tokens or note content. Passphrase buffers are `zeroize`d before the command returns.

Checklist on the change (`git diff main...`, and `git diff HEAD` for what is not committed yet):

1. **Sealed line**: titles, bodies, sources, items, names, field values, attachments stay sealed; no new plaintext copy (logs, temp files, `open/`, caches, error messages, history that skips `mask`).
2. **Lock/unlock**: nothing answers while `Db` is empty; no command bypasses the lock; HTTP sends never hold the lock.
3. **Files**: staged then renamed for anything that must not be half-written; attachment names pass `model::stored_name`; no path built from imported ids without the filter; backups re-wrapped when the passphrase changes.
4. **Interface and platform**: the passphrase only in `PassphraseField` (never a `TextEdit`), eframe persistence off, every cache of library content in `LibraryCaches`; links opened `http`/`https` only (`platform::links`); `egui_inspection` only in the `e2e` flavour; request scripts only in the script host (QuickJS), never `eval` of note content elsewhere; nothing logged but codes and locations (D18).
5. **Network/privacy**: no telemetry, no data in URLs; `Authorization`/API keys masked in history.
6. **Dependencies**: see `dependency-audit` if `Cargo.toml`/`package.json` changed.
7. Report findings by severity with file:line; state what you did not check. Do not auto-fix crypto findings.

Reference: `docs/architecture.md` → "Encryption at rest".
