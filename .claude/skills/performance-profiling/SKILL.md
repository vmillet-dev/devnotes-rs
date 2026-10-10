---
name: performance-profiling
description: Use when a DevNotes operation feels slow (query_notes, search, board, import, startup, rendering) and you must measure before optimising.
---

# Performance profiling

1. Measure first; no optimisation without a number. Rust: `cargo bench` in `src-tauri/` (criterion, 8000 notes of ~13 kB; not in CI). Compare with `--save-baseline main` then `--baseline main`. Front end: browser/Angular DevTools profiler on a built app.
2. Known facts: one lock serialises commands, and `query_notes` holds it for ~90 % of its cost; lists send previews (`PREVIEW_LINES`); side tables are read by bound ids up to 500, else by subquery; decrypting sealed fields dominates.
3. Fix at the owning layer (Rust for data; `OneInFlight` and `equal` comparators for loaders; no per-card document listeners).
4. Keep `autobenches = false`, `bench = false` and no `Drop` on `Corpus` (load-bearing).
5. Report before/after numbers on the same machine; update the table in `docs/architecture.md` → "Benchmarks" only for durable changes.
