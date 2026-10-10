---
name: backup-restore
description: Use when changing or investigating library backups, restore, recovery of damaged libraries, or import/export/share (backup/, recovery.rs, transfer/). Protects user data first.
---

# Backup, restore, import/export

- Never run destructive experiments on a real library or the user's profile; use `tempfile::TempDir` or the e2e profile.
- Launch copy: taken before sweeps via `VACUUM INTO` (under WAL the file alone is not a snapshot), key file and `attachments/` hard-linked, at most one per day.
- Restore: empty the `Mutex` before any file moves; put the live pair and `attachments/` aside in `replaced/` (never delete); match the id against the listing.
- Changing the passphrase must re-wrap every retained backup's `vault.json`.
- Files that must not be half-written: write to a staging name, then rename.
- Open path: `db::quick_check` before migrations; damaged libraries go through `recovery::set_aside`.
- Import: one transaction, unknown enum values degrade to defaults and are counted, always report the result (even "nothing imported"); new exported fields need `#[serde(default)]`; revisions are never exported.
- Verify: `cargo test` (`tests/transfer`, backup tests); e2e specs `10`, `16`, `23`, `24`, `25` when behaviour changes. Reference: "The copies, and putting one back", "Import, export and copying out".
