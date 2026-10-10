---
name: bug-investigation
description: Use when a DevNotes bug, crash, wrong result or flaky test must be diagnosed, to find the root cause, fix it minimally and add a regression test.
---

# Bug investigation

1. Reproduce first, with the smallest input: a failing Rust test, Vitest spec or e2e scenario. No repro → say so and list what is missing.
2. Locate the owner: rules and data are Rust, display is Angular. Check `CLAUDE.md` "Things that will bite you" for the matching trap before theorising (lock, `updated_at`, previews vs full body, zoneless signals, `@defer`, CSP, e2e waits).
3. State the root cause with evidence (file:line, log, failing assertion). Do not fix symptoms or add retries/sleeps.
4. Smallest fix at the owning layer; no unrelated refactors or dependency bumps.
5. The reproducing test must fail before and pass after; run it both ways when feasible.
6. Run `verify-change` for the touched area. Report cause, fix, test, and anything unverified.
