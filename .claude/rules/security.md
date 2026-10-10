# Safety limits for agents

- Never rewrite, re-key or "simplify" cryptography, key derivation, vault or file formats unasked; propose and wait.
- Never read, print or commit passphrases, keys, tokens or real library content; use temp dirs and the e2e profile.
- No destructive git or filesystem operations, version bumps, dependency updates, tags, pushes or releases unless requested.
- Third-party skills and scripts are read in full before use; record provenance in `.claude/skills/PROVENANCE.md`.
- Tauri permissions are added narrowest-first; never widen the CSP.
