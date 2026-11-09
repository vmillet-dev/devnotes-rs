---
name: performance-profiling
description: Use when a DevNotes operation feels slow (query_notes, search, board, import, startup, rendering) and you must measure before optimising.
---

# Performance profiling

1. Measure first; no optimisation without a number. Rust: `cargo bench -p engine` (criterion, 8000 notes of ~13 kB; not in CI). Compare with `--save-baseline main` then `--baseline main`. Interface: `DEVNOTES_PERF=<profile> cargo test --release -p ui --lib app::perf -- --nocapture` (the journeys of plan § 6.3), `app --features profiling` with `puffin_viewer` for egui's frames, `e2e/examples/footprint.rs` for memory and idle CPU.
2. Known facts: one lock serialises commands, and `query_notes` holds it for ~90 % of its cost; lists send previews (`PREVIEW_LINES`); side tables are read by bound ids up to 500, else by subquery; decrypting sealed fields dominates.
3. Fix at the owning layer (the core for data; a `QueryCell` compared by value, one call in flight; virtualise past ~100 rows; cache a costly `LayoutJob`; nothing costly in the frame).
4. Keep `autobenches = false`, `bench = false` and no `Drop` on `Corpus` (load-bearing).
5. Report before/after numbers on the same machine; update the table in `docs/architecture.md` → "Benchmarks" only for durable changes.
