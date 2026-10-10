---
name: devnotes-security-review
description: DevNotes-specific security review for encryption, vault/passphrase, secrets, attachments, HTTP client, Tauri permissions and privacy. Use when touching vault/, libraries, backup/, attachments, http/, capabilities, CSP or anything logged. Complements the built-in generic /security-review.
---

# DevNotes security review

**Hard rules**: never rewrite or "improve" the cryptography, key derivation or file formats on your own; propose and ask. Do not log, print or persist passphrases, keys, tokens or note content. Passphrase buffers are `zeroize`d before the command returns.

Checklist on the diff (`git diff main...`):

1. **Sealed line**: titles, bodies, sources, items, names, field values, attachments stay sealed; no new plaintext copy (logs, temp files, `open/`, caches, error messages, history that skips `mask`).
2. **Lock/unlock**: nothing answers while `Db` is empty; no command bypasses the lock; HTTP sends never hold the lock.
3. **Files**: staged then renamed for anything that must not be half-written; attachment names pass `model::stored_name`; no path built from imported ids without the filter; backups re-wrapped when the passphrase changes.
4. **Tauri**: capability changes are the narrowest possible; CSP unchanged or tighter; no `eval`/`new Function`; `opener` URLs scoped.
5. **Network/privacy**: no telemetry, no data in URLs; `Authorization`/API keys masked in history.
6. **Dependencies**: see `dependency-audit` if `Cargo.toml`/`package.json` changed.
7. Report findings by severity with file:line; state what you did not check. Do not auto-fix crypto findings.

Reference: `docs/architecture.md` → "Encryption at rest".
